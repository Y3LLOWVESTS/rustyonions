// crates/svc-admin/ui/src/components/metrics/FacetMetricsPanel.tsx
//
// RO:WHAT — Facet metrics panel for a single node.
// RO:WHY  — Show an ops-friendly view of facet RPS/error/latency with
//           clear loading/error/empty states.
//
// FIXES INCLUDED:
//   - Restores real “heartbeat” sparklines from historyByFacet (SVG path).
//   - Adds Search (facet name OR tag), Issues-only toggle, Tag filter dropdown.
//   - Adds facet tags editable via drilldown modal.
//   - Fixes “facet disappears until refresh after tagging” by using state-first tag map.
//   - Keeps purely presentational re: fetching (still only uses props).

import React, { useEffect, useMemo, useState } from 'react'
import type { FacetMetricsSummary } from '../../types/admin-api'
import { LoadingSpinner } from '../shared/LoadingSpinner'
import { ErrorBanner } from '../shared/ErrorBanner'
import { EmptyState } from '../shared/EmptyState'

type Props = {
  // Optional node scope so tags can be per-node (recommended).
  nodeId?: string
  facets: FacetMetricsSummary[] | null
  loading: boolean
  error?: string | null
  // Optional per-facet RPS history (oldest → newest), supplied by the page.
  historyByFacet?: Record<string, number[]>
}

type TagMap = Record<string, string[]>

function normalize(s: string): string {
  return s.trim().toLowerCase()
}

function lsKey(scope: string): string {
  return `svc_admin.facet_tags.v1:${scope}`
}

function loadTags(scope: string): TagMap {
  try {
    const raw = localStorage.getItem(lsKey(scope))
    if (!raw) return {}
    const parsed = JSON.parse(raw)
    if (!parsed || typeof parsed !== 'object') return {}
    const out: TagMap = {}
    for (const [k, v] of Object.entries(parsed)) {
      if (Array.isArray(v)) out[k] = v.map((x) => normalize(String(x))).filter(Boolean)
    }
    return out
  } catch {
    return {}
  }
}

function saveTags(scope: string, map: TagMap) {
  try {
    localStorage.setItem(lsKey(scope), JSON.stringify(map))
  } catch {
    // ignore
  }
}

function num(v: any): number | null {
  return typeof v === 'number' && Number.isFinite(v) ? v : null
}

function formatRps(v: number | null): string {
  if (v == null) return '—'
  return `${v.toFixed(v >= 10 ? 0 : 2)}/s`
}

function formatPctFromRatioOrPct(v: number | null): string {
  if (v == null) return '—'
  const pct = v <= 1 ? v * 100 : v
  return `${pct.toFixed(pct >= 10 ? 1 : 2)}%`
}

function formatMs(v: number | null): string {
  if (v == null) return '—'
  return `${v.toFixed(v >= 10 ? 0 : 1)}ms`
}

function makeSparkPath(values: number[], w: number, h: number, pad: number): string {
  if (!values || values.length < 2) return ''
  const min = Math.min(...values)
  const max = Math.max(...values)
  const span = Math.max(1e-9, max - min)

  const innerW = Math.max(1, w - pad * 2)
  const innerH = Math.max(1, h - pad * 2)

  const pts = values.map((v, i) => {
    const x = pad + (innerW * i) / Math.max(1, values.length - 1)
    const y = pad + innerH - ((v - min) / span) * innerH
    return [x, y] as const
  })

  return pts.map((p, i) => `${i === 0 ? 'M' : 'L'}${p[0].toFixed(2)},${p[1].toFixed(2)}`).join(' ')
}

function getHistory(historyByFacet: Record<string, number[]> | undefined, facet: string): number[] | null {
  if (!historyByFacet) return null
  // Try multiple key normalizations to avoid “history exists but graph is blank”
  const direct = historyByFacet[facet]
  if (Array.isArray(direct) && direct.length > 0) return direct

  const lc = facet.toLowerCase()
  const byLc = historyByFacet[lc]
  if (Array.isArray(byLc) && byLc.length > 0) return byLc

  // Build a lowercase index once (cheap at this scale)
  for (const [k, v] of Object.entries(historyByFacet)) {
    if (k.toLowerCase() === lc && Array.isArray(v) && v.length > 0) return v
  }

  // As a last resort, try suffix match (e.g., different prefixing in the page)
  for (const [k, v] of Object.entries(historyByFacet)) {
    if (k.toLowerCase().endsWith(lc) && Array.isArray(v) && v.length > 0) return v
  }

  return null
}

function isIssueFacet(f: FacetMetricsSummary): boolean {
  const err = num((f as any).error_rate) ?? 0
  const p95 = num((f as any).p95_latency_ms)
  const age = num((f as any).last_sample_age_secs)

  const errPct = err <= 1 ? err * 100 : err

  if (errPct > 0) return true
  if (age != null && age > 10) return true
  if (p95 != null && p95 > 250) return true
  return false
}

function Chip(props: { children: React.ReactNode }) {
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        padding: '4px 10px',
        borderRadius: 999,
        border: '1px solid rgba(255,255,255,0.12)',
        background: 'rgba(255,255,255,0.04)',
        fontSize: 12,
        fontWeight: 850,
        opacity: 0.95,
        whiteSpace: 'nowrap',
      }}
    >
      {props.children}
    </span>
  )
}

function Modal(props: { open: boolean; onClose: () => void; title: string; children: React.ReactNode }) {
  if (!props.open) return null
  return (
    <div
      role="dialog"
      aria-modal="true"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) props.onClose()
      }}
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.55)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 18,
        zIndex: 9999,
      }}
    >
      <div
        className="svc-admin-card"
        style={{
          width: 'min(960px, 96vw)',
          maxHeight: '86vh',
          overflow: 'auto',
          borderRadius: 18,
          border: '1px solid rgba(255,255,255,0.12)',
          background: 'rgba(15,15,20,0.98)',
          boxShadow: '0 24px 80px rgba(0,0,0,0.55)',
          padding: 16,
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12, alignItems: 'baseline' }}>
          <div style={{ fontSize: 16, fontWeight: 950 }}>{props.title}</div>
          <button
            type="button"
            className="svc-admin-node-action-button"
            style={{ padding: '6px 10px', borderRadius: 12 }}
            onClick={props.onClose}
          >
            Close
          </button>
        </div>
        <div style={{ marginTop: 12 }}>{props.children}</div>
      </div>
    </div>
  )
}

export function FacetMetricsPanel({ nodeId, facets, loading, error, historyByFacet }: Props) {
  const hasFacets = !!facets && facets.length > 0

  // Tag scope: per-node if nodeId provided; otherwise shared.
  const scope = nodeId && nodeId.length > 0 ? `node:${nodeId}` : 'global'

  // ✅ state-first tags (fixes “disappears until refresh”)
  const [tagsMap, setTagsMap] = useState<TagMap>(() => loadTags(scope))
  useEffect(() => setTagsMap(loadTags(scope)), [scope])
  useEffect(() => saveTags(scope, tagsMap), [scope, tagsMap])

  const [query, setQuery] = useState('')
  const [issuesOnly, setIssuesOnly] = useState(false)
  const [tagFilter, setTagFilter] = useState('') // normalized tag
  const [sortMode, setSortMode] = useState<'issues' | 'name' | 'rps' | 'err' | 'p95'>('issues')

  const allTags = useMemo(() => {
    const set = new Set<string>()
    for (const v of Object.values(tagsMap)) for (const t of v) set.add(t)
    return Array.from(set.values()).sort()
  }, [tagsMap])

  const normalizedQuery = normalize(query)

  const filtered = useMemo(() => {
    const list = (facets ?? []).filter((f) => {
      const name = f.facet
      const tags = tagsMap[name] ?? []

      const nameHit = normalizedQuery.length === 0 ? true : normalize(name).includes(normalizedQuery)
      const tagHit = normalizedQuery.length === 0 ? true : tags.some((t) => t.includes(normalizedQuery))

      if (!(nameHit || tagHit)) return false
      if (tagFilter && !tags.includes(tagFilter)) return false
      if (issuesOnly && !isIssueFacet(f)) return false
      return true
    })

    const scoreIssues = (f: FacetMetricsSummary) => {
      const err = num((f as any).error_rate) ?? 0
      const p95 = num((f as any).p95_latency_ms) ?? 0
      const age = num((f as any).last_sample_age_secs) ?? 0
      const errPct = err <= 1 ? err * 100 : err
      return errPct * 1000 + Math.min(2000, p95) + Math.min(120, age) * 10
    }

    list.sort((a, b) => {
      if (sortMode === 'name') return a.facet.localeCompare(b.facet)
      if (sortMode === 'rps') return ((b as any).rps ?? -1) - ((a as any).rps ?? -1)
      if (sortMode === 'err') return ((b as any).error_rate ?? -1) - ((a as any).error_rate ?? -1)
      if (sortMode === 'p95') return ((b as any).p95_latency_ms ?? -1) - ((a as any).p95_latency_ms ?? -1)
      return scoreIssues(b) - scoreIssues(a)
    })

    return list
  }, [facets, tagsMap, normalizedQuery, tagFilter, issuesOnly, sortMode])

  // Drilldown modal
  const [openFacet, setOpenFacet] = useState<string | null>(null)
  const open = useMemo(() => (openFacet ? (facets ?? []).find((f) => f.facet === openFacet) ?? null : null), [openFacet, facets])

  const openHistory = useMemo(() => (openFacet ? getHistory(historyByFacet, openFacet) : null), [historyByFacet, openFacet])
  const openPath = useMemo(() => {
    const series = openHistory ?? []
    return makeSparkPath(series.slice(-120), 860, 170, 10)
  }, [openHistory])

  const openTags = useMemo(() => (openFacet ? tagsMap[openFacet] ?? [] : []), [tagsMap, openFacet])
  const [newTag, setNewTag] = useState('')

  const addTag = () => {
    if (!openFacet) return
    const t = normalize(newTag)
    if (!t) return
    setTagsMap((prev) => {
      const cur = prev[openFacet] ?? []
      if (cur.includes(t)) return prev
      return { ...prev, [openFacet]: [...cur, t].sort() }
    })
    setNewTag('')
  }

  const removeTag = (t: string) => {
    if (!openFacet) return
    setTagsMap((prev) => {
      const cur = prev[openFacet] ?? []
      return { ...prev, [openFacet]: cur.filter((x) => x !== t) }
    })
  }

  return (
    <section className="svc-admin-section svc-admin-section-node-metrics">
      <header className="svc-admin-section-header">
        <div className="svc-admin-section-title-row">
          <h2 className="svc-admin-section-title">Facet metrics</h2>
        </div>
        <p className="svc-admin-section-subtitle">
          Large thumbnail cards (2-up). Search by name or tag. Toggle “Issues only” to surface problems fast.
        </p>
      </header>

      {loading && (
        <div className="svc-admin-section-body svc-admin-section-body-centered">
          <LoadingSpinner />
          <p className="svc-admin-section-body-note">Loading facet metrics from node&hellip;</p>
        </div>
      )}

      {!loading && error && (
        <div className="svc-admin-section-body">
          <ErrorBanner
            message={`Failed to load facet metrics from node. The node's admin plane or /metrics endpoint may be offline or refusing connections. (${error})`}
          />
          <p className="svc-admin-section-body-note">
            svc-admin will keep retrying on a short interval. If this is your dev environment and no node is actually
            running on the configured admin URL, this warning is expected.
          </p>
        </div>
      )}

      {!loading && !error && !hasFacets && (
        <div className="svc-admin-section-body">
          <EmptyState message="No facet metrics observed yet. The node may be starting up or has not emitted facet metrics in the recent sampling window." />
        </div>
      )}

      {!loading && !error && hasFacets && facets && (
        <div className="svc-admin-section-body">
          {/* Controls */}
          <div
            style={{
              display: 'flex',
              gap: 10,
              alignItems: 'center',
              flexWrap: 'wrap',
              marginBottom: 12,
            }}
          >
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search facets (name or tag)…"
              style={{
                minWidth: 280,
                flex: '1 1 320px',
                borderRadius: 12,
                padding: '10px 12px',
                border: '1px solid rgba(255,255,255,0.12)',
                background: 'rgba(255,255,255,0.03)',
                color: 'var(--svc-admin-color-text)',
                outline: 'none',
              }}
            />

            <label style={{ display: 'inline-flex', alignItems: 'center', gap: 8, cursor: 'pointer' }}>
              <input type="checkbox" checked={issuesOnly} onChange={(e) => setIssuesOnly(e.target.checked)} />
              <span style={{ fontWeight: 850, opacity: 0.9 }}>Issues only</span>
            </label>

            <select
              value={tagFilter}
              onChange={(e) => setTagFilter(e.target.value)}
              style={{
                borderRadius: 12,
                padding: '10px 10px',
                border: '1px solid rgba(255,255,255,0.12)',
                background: 'rgba(255,255,255,0.03)',
                color: 'var(--svc-admin-color-text)',
              }}
              title="Filter facets by tag."
            >
              <option value="">All tags</option>
              {allTags.map((t) => (
                <option key={t} value={t}>
                  tag:{t}
                </option>
              ))}
            </select>

            <select
              value={sortMode}
              onChange={(e) => setSortMode(e.target.value as any)}
              style={{
                borderRadius: 12,
                padding: '10px 10px',
                border: '1px solid rgba(255,255,255,0.12)',
                background: 'rgba(255,255,255,0.03)',
                color: 'var(--svc-admin-color-text)',
              }}
              title="Sort facets."
            >
              <option value="issues">Sort: issues</option>
              <option value="name">Sort: name</option>
              <option value="rps">Sort: RPS</option>
              <option value="err">Sort: error</option>
              <option value="p95">Sort: p95</option>
            </select>

            <div style={{ opacity: 0.75, fontVariantNumeric: 'tabular-nums' }}>{filtered.length} facets</div>
          </div>

          {filtered.length === 0 ? (
            <EmptyState message="No facets match the current filters." />
          ) : (
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(2, minmax(360px, 1fr))',
                gap: 14,
              }}
            >
              {filtered.map((facet) => {
                const name = facet.facet
                const history = getHistory(historyByFacet, name)
                const series = (history ?? []).slice(-60)
                const path = makeSparkPath(series, 540, 72, 8)
                const issue = isIssueFacet(facet)
                const tags = tagsMap[name] ?? []

                return (
                  <button
                    key={name}
                    type="button"
                    onClick={() => setOpenFacet(name)}
                    className="svc-admin-card"
                    style={{
                      textAlign: 'left',
                      padding: 16,
                      borderRadius: 18,
                      border: issue ? '1px solid rgba(244,63,94,0.28)' : '1px solid rgba(255,255,255,0.12)',
                      background: 'rgba(255,255,255,0.02)',
                      boxShadow: '0 14px 40px rgba(0,0,0,0.24)',
                      cursor: 'pointer',
                    }}
                  >
                    <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12 }}>
                      <div style={{ fontSize: 18, fontWeight: 950, letterSpacing: 0.2 }}>{name}</div>
                      <div style={{ opacity: 0.7, fontVariantNumeric: 'tabular-nums' }}>
                        {(facet as any).last_sample_age_secs != null
                          ? `${Number((facet as any).last_sample_age_secs).toFixed(1)}s`
                          : '—'}
                      </div>
                    </div>

                    <div style={{ marginTop: 8, opacity: 0.9 }}>
                      <span style={{ marginRight: 14 }}>RPS {formatRps(num((facet as any).rps))}</span>
                      <span style={{ marginRight: 14 }}>
                        Err {formatPctFromRatioOrPct(num((facet as any).error_rate))}
                      </span>
                      <span>p95 {formatMs(num((facet as any).p95_latency_ms))}</span>
                    </div>

                    <div style={{ marginTop: 10, display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                      {issue && <Chip>issue</Chip>}
                      {tags.slice(0, 4).map((t) => (
                        <Chip key={t}>{t}</Chip>
                      ))}
                      {tags.length > 4 && <Chip>+{tags.length - 4}</Chip>}
                      {tags.length === 0 && <span style={{ opacity: 0.65 }}>Click for drilldown</span>}
                    </div>

                    {/* ✅ real heartbeat line */}
                    {path ? (
                      <svg viewBox="0 0 540 72" width="100%" height="72" style={{ display: 'block', marginTop: 10 }}>
                        <path d={path} fill="none" stroke="currentColor" strokeWidth="2" opacity="0.9" />
                      </svg>
                    ) : (
                      <div style={{ marginTop: 14, opacity: 0.55, fontSize: 12 }}>
                        Waiting for history (sparkline appears once samples exist).
                      </div>
                    )}
                  </button>
                )
              })}
            </div>
          )}

          {/* Drilldown + tags editor */}
          <Modal open={openFacet != null} onClose={() => setOpenFacet(null)} title={openFacet ?? 'Facet'}>
            {open ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', alignItems: 'center' }}>
                  <Chip>RPS {formatRps(num((open as any).rps))}</Chip>
                  <Chip>Err {formatPctFromRatioOrPct(num((open as any).error_rate))}</Chip>
                  <Chip>p95 {formatMs(num((open as any).p95_latency_ms))}</Chip>
                  <Chip>p99 {formatMs(num((open as any).p99_latency_ms))}</Chip>
                  <Chip>
                    Age{' '}
                    {(open as any).last_sample_age_secs != null
                      ? `${Number((open as any).last_sample_age_secs).toFixed(1)}s`
                      : '—'}
                  </Chip>
                </div>

                <div
                  style={{
                    borderRadius: 16,
                    border: '1px solid rgba(255,255,255,0.12)',
                    background: 'rgba(255,255,255,0.02)',
                    padding: 12,
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', gap: 10, flexWrap: 'wrap' }}>
                    <div style={{ fontWeight: 900, opacity: 0.9 }}>RPS history</div>
                    <div style={{ opacity: 0.7 }}>{openHistory?.length ?? 0} samples</div>
                  </div>

                  {openPath ? (
                    <svg viewBox="0 0 860 170" width="100%" height="170" style={{ display: 'block', marginTop: 10 }}>
                      <path d={openPath} fill="none" stroke="currentColor" strokeWidth="2.5" opacity="0.92" />
                    </svg>
                  ) : (
                    <div style={{ marginTop: 10, opacity: 0.7 }}>No history for this facet yet.</div>
                  )}
                </div>

                <div
                  style={{
                    borderRadius: 16,
                    border: '1px solid rgba(255,255,255,0.12)',
                    background: 'rgba(255,255,255,0.02)',
                    padding: 12,
                  }}
                >
                  <div style={{ fontWeight: 950, marginBottom: 10 }}>Facet tags</div>

                  <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', alignItems: 'center' }}>
                    {openTags.length === 0 ? (
                      <div style={{ opacity: 0.7 }}>No tags yet. Add one below.</div>
                    ) : (
                      openTags.map((t) => (
                        <span
                          key={t}
                          style={{
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: 8,
                            padding: '6px 10px',
                            borderRadius: 999,
                            border: '1px solid rgba(255,255,255,0.12)',
                            background: 'rgba(255,255,255,0.04)',
                            fontSize: 12,
                            fontWeight: 900,
                          }}
                        >
                          {t}
                          <button
                            type="button"
                            onClick={() => removeTag(t)}
                            className="svc-admin-node-action-button"
                            style={{ padding: '4px 8px', borderRadius: 999 }}
                            title="Remove tag"
                          >
                            ×
                          </button>
                        </span>
                      ))
                    )}
                  </div>

                  <div style={{ display: 'flex', gap: 10, marginTop: 12, flexWrap: 'wrap' }}>
                    <input
                      value={newTag}
                      onChange={(e) => setNewTag(e.target.value)}
                      placeholder="Add tag (e.g. auth, storage, routing)…"
                      style={{
                        flex: '1 1 260px',
                        borderRadius: 12,
                        padding: '10px 12px',
                        border: '1px solid rgba(255,255,255,0.12)',
                        background: 'rgba(255,255,255,0.03)',
                        color: 'var(--svc-admin-color-text)',
                        outline: 'none',
                      }}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') addTag()
                      }}
                    />
                    <button
                      type="button"
                      className="svc-admin-node-action-button"
                      style={{ padding: '10px 12px', borderRadius: 12 }}
                      onClick={addTag}
                    >
                      Add tag
                    </button>

                    {openTags.length > 0 && (
                      <button
                        type="button"
                        className="svc-admin-node-action-button"
                        style={{ padding: '10px 12px', borderRadius: 12 }}
                        onClick={() => setTagFilter(openTags[0] ?? '')}
                        title="Filter the grid by one of this facet’s tags."
                      >
                        Filter grid by this facet’s tag
                      </button>
                    )}
                  </div>

                  <div style={{ marginTop: 10, opacity: 0.7, fontSize: 12 }}>
                    Tip: the main search box matches tags too (type “auth”, “routing”, etc.).
                  </div>
                </div>
              </div>
            ) : (
              <div style={{ opacity: 0.8 }}>Facet not found.</div>
            )}
          </Modal>
        </div>
      )}
    </section>
  )
}
