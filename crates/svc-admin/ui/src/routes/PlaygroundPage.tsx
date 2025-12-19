// crates/svc-admin/ui/src/routes/PlaygroundPage.tsx
//
// RO:WHAT — Dev-only App Playground (local manifest editor + validation scaffold).
// RO:WHY  — Operators/devs need a safe “sandbox” to iterate on facet manifests
//           and see validation feedback without turning svc-admin into a remote executor.
// RO:INVARIANTS —
//   - Must be gated by uiConfig.dev.enableAppPlayground (dev-only posture).
//   - Read-only posture: no node mutations, no remote file browsing, no arbitrary proxy.
//   - Resilient: if ui-config can’t be fetched, default to “disabled” (safe).
//
// NOTE:
//   This batch wires the route + a functional local manifest “validator” scaffold.
//   In later batches we’ll:
//     - add a Sidebar link (also gated),
//     - introduce real example/compile endpoints or ron-app-sdk-ts integration,
//     - add request-builder + dry-run/proxy (still read-only + audited).

import React, { useEffect, useMemo, useState } from 'react'
import { adminClient } from '../api/adminClient'
import type { UiConfigDto } from '../types/admin-api'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { ErrorBanner } from '../components/shared/ErrorBanner'
import { EmptyState } from '../components/shared/EmptyState'

type Example = {
  id: string
  title: string
  description: string
  manifestToml: string
}

type ParseResult = {
  ok: boolean
  value: any | null
  errors: string[]
  warnings: string[]
}

const EXAMPLES: Example[] = [
  {
    id: 'hello-echo',
    title: 'Echo Facet (minimal)',
    description:
      'A tiny manifest shape to sanity-check parsing and show a “facet + routes” structure.',
    manifestToml: `# Manifest.toml (example)
[facet]
id = "echo"
kind = "echo"

[[route]]
method = "GET"
path = "/ping"

[[route]]
method = "POST"
path = "/echo"
`,
  },
  {
    id: 'static-site',
    title: 'Static Facet (mocked)',
    description:
      'A static facet example with a couple of routes (schema is intentionally permissive for now).',
    manifestToml: `# Manifest.toml (example)
[facet]
id = "static"
kind = "static"

[static]
root = "./public"
cache_seconds = 60

[[route]]
method = "GET"
path = "/"
`,
  },
  {
    id: 'proxy',
    title: 'Proxy Facet (mocked)',
    description:
      'A proxy-style facet example with upstream hints (still local-only in this UI batch).',
    manifestToml: `# Manifest.toml (example)
[facet]
id = "proxy"
kind = "proxy"

[proxy]
upstream = "https://example.com"
timeout_ms = 2500

[[route]]
method = "GET"
path = "/api"
`,
  },
]

function tryCopyToClipboard(text: string): Promise<boolean> {
  try {
    if (!navigator.clipboard?.writeText) return Promise.resolve(false)
    return navigator.clipboard
      .writeText(text)
      .then(() => true)
      .catch(() => false)
  } catch {
    return Promise.resolve(false)
  }
}

// ---------------------------
// Minimal TOML-ish parser
// ---------------------------
// This is intentionally small + tolerant. It supports:
// - [table] headers
// - [[array]] headers (array-of-tables)
// - key = "string" | 'string' | number | bool | [array]
// It does NOT aim to be spec-complete TOML. It’s enough to provide
// “useful operator feedback” without pulling in dependencies yet.

function stripInlineComment(line: string): string {
  // Remove # comments unless inside quotes (simple heuristic).
  let inS = false
  let inD = false
  for (let i = 0; i < line.length; i++) {
    const c = line[i]
    if (c === "'" && !inD) inS = !inS
    else if (c === '"' && !inS) inD = !inD
    else if (c === '#' && !inS && !inD) return line.slice(0, i)
  }
  return line
}

function isQuoted(s: string): boolean {
  return (
    (s.startsWith('"') && s.endsWith('"') && s.length >= 2) ||
    (s.startsWith("'") && s.endsWith("'") && s.length >= 2)
  )
}

function unquote(s: string): string {
  if (!isQuoted(s)) return s
  return s.slice(1, -1)
}

function splitTopLevelCsv(s: string): string[] {
  // Split on commas not inside quotes or nested brackets.
  const out: string[] = []
  let cur = ''
  let depth = 0
  let inS = false
  let inD = false

  for (let i = 0; i < s.length; i++) {
    const c = s[i]
    if (c === "'" && !inD) inS = !inS
    else if (c === '"' && !inS) inD = !inD
    else if (!inS && !inD) {
      if (c === '[') depth++
      if (c === ']') depth = Math.max(0, depth - 1)
      if (c === ',' && depth === 0) {
        out.push(cur.trim())
        cur = ''
        continue
      }
    }
    cur += c
  }

  if (cur.trim().length) out.push(cur.trim())
  return out
}

function parseValue(raw: string, errors: string[]): any {
  const v = raw.trim()

  if (v === 'true') return true
  if (v === 'false') return false

  if (isQuoted(v)) return unquote(v)

  // Array
  if (v.startsWith('[') && v.endsWith(']')) {
    const inner = v.slice(1, -1).trim()
    if (!inner) return []
    const parts = splitTopLevelCsv(inner)
    return parts.map((p) => parseValue(p, errors))
  }

  // Number (int/float)
  if (/^[+-]?\d+(\.\d+)?$/.test(v)) {
    const n = Number(v)
    if (!Number.isFinite(n)) {
      errors.push(`Invalid number: ${v}`)
      return v
    }
    return n
  }

  // Bare string fallback
  return v
}

function ensurePath(obj: any, path: string[]): any {
  let cur = obj
  for (const k of path) {
    if (typeof cur[k] !== 'object' || cur[k] === null || Array.isArray(cur[k])) {
      cur[k] = {}
    }
    cur = cur[k]
  }
  return cur
}

function setDeep(obj: any, dottedKey: string, value: any) {
  const parts = dottedKey.split('.').map((s) => s.trim()).filter(Boolean)
  if (parts.length === 0) return
  const leaf = parts[parts.length - 1]
  const parent = ensurePath(obj, parts.slice(0, -1))
  parent[leaf] = value
}

function parseTomlLoose(input: string): ParseResult {
  const errors: string[] = []
  const warnings: string[] = []

  const root: any = {}
  let curTable: any = root
  let curTablePath: string[] = []

  const lines = input.split(/\r?\n/)

  for (let ln = 0; ln < lines.length; ln++) {
    const rawLine = lines[ln]
    const noComment = stripInlineComment(rawLine).trim()
    if (!noComment) continue

    // [[array-of-tables]]
    const aotMatch = noComment.match(/^\[\[(.+)\]\]$/)
    if (aotMatch) {
      const name = aotMatch[1].trim()
      if (!name) {
        errors.push(`Line ${ln + 1}: Empty [[table]] header`)
        continue
      }
      const path = name.split('.').map((s) => s.trim()).filter(Boolean)
      if (path.length === 0) {
        errors.push(`Line ${ln + 1}: Invalid [[table]] header`)
        continue
      }

      const parent = ensurePath(root, path.slice(0, -1))
      const leaf = path[path.length - 1]
      if (!Array.isArray(parent[leaf])) parent[leaf] = []
      const next = {}
      parent[leaf].push(next)

      curTable = next
      curTablePath = path
      continue
    }

    // [table]
    const tblMatch = noComment.match(/^\[(.+)\]$/)
    if (tblMatch) {
      const name = tblMatch[1].trim()
      if (!name) {
        errors.push(`Line ${ln + 1}: Empty [table] header`)
        continue
      }
      const path = name.split('.').map((s) => s.trim()).filter(Boolean)
      if (path.length === 0) {
        errors.push(`Line ${ln + 1}: Invalid [table] header`)
        continue
      }
      curTable = ensurePath(root, path)
      curTablePath = path
      continue
    }

    // key = value
    const eq = noComment.indexOf('=')
    if (eq <= 0) {
      warnings.push(`Line ${ln + 1}: Unrecognized line (ignored)`)
      continue
    }

    const key = noComment.slice(0, eq).trim()
    const valRaw = noComment.slice(eq + 1).trim()
    if (!key) {
      errors.push(`Line ${ln + 1}: Missing key before '='`)
      continue
    }
    if (!valRaw) {
      errors.push(`Line ${ln + 1}: Missing value after '=' for key "${key}"`)
      continue
    }

    const value = parseValue(valRaw, errors)

    // If key is dotted, set deep relative to current table
    if (key.includes('.')) {
      setDeep(curTable, key, value)
    } else {
      if (Object.prototype.hasOwnProperty.call(curTable, key)) {
        warnings.push(
          `Line ${ln + 1}: Duplicate key "${key}" in [${curTablePath.join('.') || 'root'}] (overwritten)`,
        )
      }
      curTable[key] = value
    }
  }

  // Gentle “shape” hints (non-fatal)
  const hasFacet = typeof root.facet === 'object' && root.facet !== null
  const hasRoutes = Array.isArray(root.route) || Array.isArray(root.routes)
  if (!hasFacet) warnings.push('Hint: expected a [facet] table (common manifest convention).')
  if (!hasRoutes) warnings.push('Hint: expected at least one [[route]] entry.')

  const ok = errors.length === 0
  return { ok, value: ok ? root : null, errors, warnings }
}

export function PlaygroundPage() {
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [uiConfig, setUiConfig] = useState<UiConfigDto | null>(null)

  const enabled = Boolean(uiConfig?.dev?.enableAppPlayground)

  const [selectedExampleId, setSelectedExampleId] = useState(EXAMPLES[0]?.id ?? 'hello-echo')
  const selectedExample = useMemo(
    () => EXAMPLES.find((e) => e.id === selectedExampleId) ?? EXAMPLES[0],
    [selectedExampleId],
  )

  const [manifestText, setManifestText] = useState(selectedExample?.manifestToml ?? '')
  const [parseResult, setParseResult] = useState<ParseResult | null>(null)
  const [copyNote, setCopyNote] = useState<string | null>(null)

  useEffect(() => {
    // Keep manifest in sync when example changes (unless user is actively editing).
    // This is intentionally “reset-on-select” for now; later we can add “dirty” tracking.
    setManifestText(selectedExample?.manifestToml ?? '')
    setParseResult(null)
    setCopyNote(null)
  }, [selectedExampleId])

  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const cfg = await adminClient.getUiConfig()
        if (cancelled) return
        setUiConfig(cfg)
        setError(null)
      } catch (err) {
        if (cancelled) return
        const msg = err instanceof Error ? err.message : 'Failed to load UI config'
        // Safe default: treat as disabled if we can’t confirm dev flag.
        setUiConfig(null)
        setError(msg)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    load()
    return () => {
      cancelled = true
    }
  }, [])

  function runValidate() {
    setCopyNote(null)
    const res = parseTomlLoose(manifestText)
    setParseResult(res)
  }

  async function copyJson() {
    if (!parseResult?.ok || !parseResult.value) return
    const json = JSON.stringify(parseResult.value, null, 2)
    const ok = await tryCopyToClipboard(json)
    setCopyNote(ok ? 'Copied JSON to clipboard.' : 'Clipboard copy not available in this browser.')
    window.setTimeout(() => setCopyNote(null), 2200)
  }

  return (
    <div className="svc-admin-page">
      <header className="svc-admin-page-header">
        <h1>App Playground</h1>
        <p>
          Dev-only sandbox for iterating on facet manifests and seeing validation feedback. This is
          intentionally <strong>read-only</strong> and does not execute code or mutate nodes.
        </p>
      </header>

      {loading && <LoadingSpinner />}

      {!loading && error && (
        <div style={{ marginBottom: '1rem' }}>
          <ErrorBanner message={error} />
        </div>
      )}

      {!loading && !enabled && (
        <EmptyState
          message={[
            'Playground is currently disabled for this svc-admin instance.',
            'Enable it with the dev-only flag:',
            'SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=true',
          ].join('\n')}
        />
      )}

      {!loading && enabled && (
        <div
          className="svc-admin-section"
          style={{
            display: 'grid',
            gridTemplateColumns: 'minmax(260px, 340px) minmax(0, 1fr) minmax(320px, 420px)',
            gap: '1rem',
            alignItems: 'start',
          }}
        >
          {/* Left: examples */}
          <section className="svc-admin-section" style={{ margin: 0 }}>
            <h2 style={{ marginTop: 0 }}>Examples</h2>
            <p style={{ opacity: 0.85, marginTop: 0 }}>
              Pick an example manifest to load into the editor.
            </p>

            <div style={{ display: 'grid', gap: '0.6rem' }}>
              {EXAMPLES.map((ex) => {
                const active = ex.id === selectedExampleId
                return (
                  <button
                    key={ex.id}
                    type="button"
                    onClick={() => setSelectedExampleId(ex.id)}
                    className="svc-admin-node-action-button"
                    style={{
                      textAlign: 'left',
                      padding: '0.7rem 0.8rem',
                      borderRadius: 10,
                      opacity: active ? 1 : 0.92,
                      outline: active ? '2px solid rgba(99, 102, 241, 0.55)' : 'none',
                    }}
                  >
                    <div style={{ fontWeight: 800, marginBottom: 4 }}>{ex.title}</div>
                    <div style={{ fontSize: '0.92rem', opacity: 0.82 }}>{ex.description}</div>
                  </button>
                )
              })}
            </div>

            <div style={{ marginTop: '0.9rem', fontSize: '0.9rem', opacity: 0.8 }}>
              Safety: this page is local-only. No node calls are made in this batch.
            </div>
          </section>

          {/* Middle: editor */}
          <section className="svc-admin-section" style={{ margin: 0 }}>
            <h2 style={{ marginTop: 0 }}>Manifest Editor</h2>
            <p style={{ opacity: 0.85, marginTop: 0 }}>
              Edit a TOML-ish manifest. Click “Validate” to parse and see errors/warnings.
            </p>

            <textarea
              value={manifestText}
              onChange={(e) => setManifestText(e.target.value)}
              spellCheck={false}
              style={{
                width: '100%',
                minHeight: 420,
                resize: 'vertical',
                fontFamily:
                  'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
                fontSize: 13,
                lineHeight: 1.35,
                padding: '0.85rem',
                borderRadius: 12,
                border: '1px solid rgba(255,255,255,0.10)',
                background: 'rgba(0,0,0,0.18)',
                color: 'inherit',
              }}
            />

            <div style={{ display: 'flex', gap: '0.6rem', marginTop: '0.8rem' }}>
              <button
                type="button"
                className="svc-admin-node-action-button"
                onClick={runValidate}
              >
                Validate
              </button>

              <button
                type="button"
                className="svc-admin-node-action-button"
                onClick={() => {
                  setManifestText(selectedExample?.manifestToml ?? '')
                  setParseResult(null)
                  setCopyNote(null)
                }}
              >
                Reset to example
              </button>

              <button
                type="button"
                className="svc-admin-node-action-button"
                disabled={!parseResult?.ok || !parseResult.value}
                onClick={copyJson}
                title={!parseResult?.ok ? 'Validate successfully to enable JSON copy.' : undefined}
              >
                Copy JSON
              </button>

              {copyNote && (
                <span style={{ alignSelf: 'center', fontSize: '0.92rem', opacity: 0.9 }}>
                  {copyNote}
                </span>
              )}
            </div>
          </section>

          {/* Right: results */}
          <section className="svc-admin-section" style={{ margin: 0 }}>
            <h2 style={{ marginTop: 0 }}>Validation Output</h2>
            <p style={{ opacity: 0.85, marginTop: 0 }}>
              Errors block parsing; warnings are advisory.
            </p>

            {!parseResult && (
              <div style={{ opacity: 0.78 }}>
                Run <strong>Validate</strong> to see parse output.
              </div>
            )}

            {parseResult && (
              <>
                <div style={{ display: 'flex', gap: '0.6rem', flexWrap: 'wrap' }}>
                  <span
                    style={{
                      padding: '0.2rem 0.55rem',
                      borderRadius: 999,
                      fontWeight: 800,
                      fontSize: 12,
                      background: parseResult.ok
                        ? 'rgba(16,185,129,0.14)'
                        : 'rgba(239,68,68,0.14)',
                      color: parseResult.ok ? 'rgb(110,231,183)' : 'rgb(252,165,165)',
                      border: '1px solid rgba(255,255,255,0.08)',
                    }}
                  >
                    {parseResult.ok ? 'OK' : 'ERROR'}
                  </span>

                  {parseResult.warnings.length > 0 && (
                    <span
                      style={{
                        padding: '0.2rem 0.55rem',
                        borderRadius: 999,
                        fontWeight: 800,
                        fontSize: 12,
                        background: 'rgba(245,158,11,0.14)',
                        color: 'rgb(253,230,138)',
                        border: '1px solid rgba(255,255,255,0.08)',
                      }}
                    >
                      Warnings: {parseResult.warnings.length}
                    </span>
                  )}

                  {parseResult.errors.length > 0 && (
                    <span
                      style={{
                        padding: '0.2rem 0.55rem',
                        borderRadius: 999,
                        fontWeight: 800,
                        fontSize: 12,
                        background: 'rgba(239,68,68,0.14)',
                        color: 'rgb(252,165,165)',
                        border: '1px solid rgba(255,255,255,0.08)',
                      }}
                    >
                      Errors: {parseResult.errors.length}
                    </span>
                  )}
                </div>

                {parseResult.errors.length > 0 && (
                  <div style={{ marginTop: '0.9rem' }}>
                    <div style={{ fontWeight: 800, marginBottom: '0.35rem' }}>Errors</div>
                    <ul style={{ margin: 0, paddingLeft: '1.15rem' }}>
                      {parseResult.errors.map((e) => (
                        <li key={e} style={{ marginBottom: '0.35rem' }}>
                          {e}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {parseResult.warnings.length > 0 && (
                  <div style={{ marginTop: '0.9rem' }}>
                    <div style={{ fontWeight: 800, marginBottom: '0.35rem' }}>Warnings</div>
                    <ul style={{ margin: 0, paddingLeft: '1.15rem' }}>
                      {parseResult.warnings.map((w) => (
                        <li key={w} style={{ marginBottom: '0.35rem' }}>
                          {w}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                <div style={{ marginTop: '0.9rem' }}>
                  <div style={{ fontWeight: 800, marginBottom: '0.35rem' }}>Parsed JSON</div>
                  <pre
                    style={{
                      margin: 0,
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-word',
                      fontSize: 12.5,
                      lineHeight: 1.35,
                      padding: '0.75rem',
                      borderRadius: 12,
                      border: '1px solid rgba(255,255,255,0.08)',
                      background: 'rgba(0,0,0,0.18)',
                      maxHeight: 420,
                      overflow: 'auto',
                    }}
                  >
                    {parseResult.ok && parseResult.value
                      ? JSON.stringify(parseResult.value, null, 2)
                      : '(no JSON — fix errors and validate again)'}
                  </pre>

                  <div style={{ marginTop: '0.75rem', fontSize: '0.9rem', opacity: 0.8 }}>
                    Next batches will replace this loose parser with the real manifest contract
                    (ron-app-sdk / backend validation), plus a request builder + dry-run.
                  </div>
                </div>
              </>
            )}
          </section>
        </div>
      )}
    </div>
  )
}
