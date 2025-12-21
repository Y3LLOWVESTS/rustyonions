/**
 * RO:WHAT — Dev-only Request Inspector drawer for recent API calls.
 * RO:WHY — During macronode wiring, instantly shows which endpoints are failing
 *         (404/501/shape changes), plus latency and error payload snippets.
 * RO:INTERACTS — api/adminClient.httpLog (ring buffer); browser UI only.
 * RO:INVARIANTS — Bounded memory (ring buffer); no background retries; no PII
 *               persistence; drawer must not block normal UI interactions.
 * RO:SECURITY — Shows only what the browser already received; no secrets stored.
 * RO:TEST — Manual: trigger errors; ensure entries appear; copy details works.
 */

import React, { useEffect, useMemo, useState } from 'react'
import { httpLog, type HttpLogEntry } from '../../api/adminClient'

function toneForStatus(status?: number): 'ok' | 'warn' | 'bad' | 'muted' {
  if (status == null) return 'muted'
  if (status >= 200 && status < 300) return 'ok'
  if (status >= 300 && status < 400) return 'warn'
  if (status >= 400) return 'bad'
  return 'muted'
}

function fmtMs(ms: number): string {
  if (!Number.isFinite(ms)) return '—'
  if (ms < 1) return '<1ms'
  if (ms < 1000) return `${ms.toFixed(0)}ms`
  return `${(ms / 1000).toFixed(2)}s`
}

function pillStyle(tone: 'ok' | 'warn' | 'bad' | 'muted'): React.CSSProperties {
  const border = '1px solid rgba(255,255,255,0.14)'
  const base: React.CSSProperties = {
    padding: '2px 8px',
    borderRadius: 999,
    border,
    fontSize: 12,
    lineHeight: 1.2,
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
  }
  if (tone === 'ok') return { ...base, background: 'rgba(16,185,129,0.14)', color: 'rgba(167,243,208,0.95)' }
  if (tone === 'warn') return { ...base, background: 'rgba(251,146,60,0.14)', color: 'rgba(254,215,170,0.95)' }
  if (tone === 'bad') return { ...base, background: 'rgba(244,63,94,0.14)', color: 'rgba(253,164,175,0.95)' }
  return { ...base, background: 'rgba(255,255,255,0.06)', color: 'rgba(226,232,240,0.92)' }
}

export function RequestInspectorDrawer() {
  const [open, setOpen] = useState(false)
  const [entries, setEntries] = useState<HttpLogEntry[]>(httpLog.getEntries())
  const [filter, setFilter] = useState('')
  const [selectedId, setSelectedId] = useState<string | null>(null)

  useEffect(() => {
    return httpLog.subscribe((e) => setEntries(e))
  }, [])

  const filtered = useMemo(() => {
    const q = filter.trim().toLowerCase()
    if (!q) return entries
    return entries.filter((e) => {
      const hay = `${e.method} ${e.path} ${e.status ?? ''} ${e.error ?? ''}`.toLowerCase()
      return hay.includes(q)
    })
  }, [entries, filter])

  const selected = useMemo(() => {
    if (!selectedId) return null
    return entries.find((e) => e.id === selectedId) ?? null
  }, [entries, selectedId])

  const drawerWidth = 520

  const containerStyle: React.CSSProperties = {
    position: 'fixed',
    right: 16,
    bottom: 16,
    zIndex: 9999,
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'flex-end',
    gap: 10,
    pointerEvents: 'none', // enable click only on inner controls
  }

  const buttonStyle: React.CSSProperties = {
    pointerEvents: 'auto',
    display: 'inline-flex',
    alignItems: 'center',
    gap: 8,
    padding: '10px 12px',
    borderRadius: 999,
    border: '1px solid rgba(255,255,255,0.16)',
    background: 'rgba(15,23,42,0.72)',
    backdropFilter: 'blur(10px)',
    color: 'rgba(226,232,240,0.95)',
    boxShadow: '0 14px 34px rgba(0,0,0,0.30)',
    cursor: 'pointer',
    userSelect: 'none',
    fontSize: 13,
  }

  const drawerStyle: React.CSSProperties = {
    pointerEvents: 'auto',
    width: drawerWidth,
    maxWidth: 'calc(100vw - 32px)',
    height: open ? 560 : 0,
    overflow: 'hidden',
    borderRadius: 18,
    border: open ? '1px solid rgba(255,255,255,0.14)' : '0px solid transparent',
    background: open
      ? 'rgba(2,6,23,0.82)'
      : 'transparent',
    backdropFilter: open ? 'blur(10px)' : undefined,
    boxShadow: open ? '0 18px 48px rgba(0,0,0,0.45)' : undefined,
    transition: 'height 160ms ease, border 160ms ease, background 160ms ease',
  }

  const headerStyle: React.CSSProperties = {
    padding: '12px 12px 10px 12px',
    borderBottom: '1px solid rgba(255,255,255,0.10)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: 10,
  }

  const inputStyle: React.CSSProperties = {
    width: '100%',
    padding: '8px 10px',
    borderRadius: 12,
    border: '1px solid rgba(255,255,255,0.14)',
    background: 'rgba(15,23,42,0.55)',
    color: 'rgba(226,232,240,0.95)',
    outline: 'none',
    fontSize: 13,
  }

  const smallBtn: React.CSSProperties = {
    padding: '8px 10px',
    borderRadius: 12,
    border: '1px solid rgba(255,255,255,0.14)',
    background: 'rgba(255,255,255,0.06)',
    color: 'rgba(226,232,240,0.95)',
    cursor: 'pointer',
    fontSize: 13,
    whiteSpace: 'nowrap',
  }

  const listStyle: React.CSSProperties = {
    display: 'grid',
    gridTemplateColumns: '1fr',
    gap: 8,
    padding: 12,
    overflowY: 'auto',
    height: 320,
  }

  const detailStyle: React.CSSProperties = {
    padding: 12,
    borderTop: '1px solid rgba(255,255,255,0.10)',
    height: 560 - 56 - 320, // total - header - list (approx)
    overflow: 'auto',
  }

  const rowBase: React.CSSProperties = {
    borderRadius: 14,
    border: '1px solid rgba(255,255,255,0.10)',
    background: 'rgba(255,255,255,0.04)',
    padding: 10,
    cursor: 'pointer',
  }

  function copySelected() {
    if (!selected) return
    const text = JSON.stringify(selected, null, 2)
    navigator.clipboard?.writeText(text).catch(() => {
      // no-op
    })
  }

  return (
    <div style={containerStyle}>
      <div
        role="button"
        aria-label="Toggle request inspector"
        style={buttonStyle}
        onClick={() => setOpen((v) => !v)}
      >
        <span style={{ fontWeight: 700 }}>API</span>
        <span style={{ opacity: 0.9 }}>Requests</span>
        <span style={{ opacity: 0.75, fontVariantNumeric: 'tabular-nums' }}>
          ({entries.length})
        </span>
        <span style={{ opacity: 0.75 }}>{open ? '▾' : '▴'}</span>
      </div>

      <div style={drawerStyle} aria-hidden={!open}>
        <div style={headerStyle}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            <div style={{ fontSize: 13, fontWeight: 700, color: 'rgba(226,232,240,0.95)' }}>
              Request Inspector
            </div>
            <div style={{ fontSize: 12, opacity: 0.75 }}>
              Dev-only. Shows recent SPA → svc-admin API calls.
            </div>
          </div>

          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <button type="button" style={smallBtn} onClick={() => httpLog.clear()}>
              Clear
            </button>
            <button
              type="button"
              style={{ ...smallBtn, opacity: selected ? 1 : 0.55, cursor: selected ? 'pointer' : 'not-allowed' }}
              disabled={!selected}
              onClick={copySelected}
            >
              Copy
            </button>
          </div>
        </div>

        <div style={{ padding: '0 12px 12px 12px' }}>
          <input
            style={inputStyle}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter by path/status/error (e.g. /api/nodes 404)"
          />
        </div>

        <div style={listStyle}>
          {filtered.length === 0 ? (
            <div style={{ opacity: 0.75, fontSize: 13, padding: 8 }}>
              No matching requests.
            </div>
          ) : (
            filtered.map((e) => {
              const tone = toneForStatus(e.status)
              const active = e.id === selectedId

              return (
                <div
                  key={e.id}
                  style={{
                    ...rowBase,
                    border: active ? '1px solid rgba(99,102,241,0.55)' : rowBase.border,
                    background: active ? 'rgba(99,102,241,0.10)' : rowBase.background,
                  }}
                  onClick={() => setSelectedId(e.id)}
                >
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 10 }}>
                    <div style={{ display: 'flex', gap: 8, alignItems: 'center', minWidth: 0 }}>
                      <span style={pillStyle(tone)}>
                        {e.status ?? '—'}
                      </span>
                      <span style={{ fontSize: 12, opacity: 0.85, fontVariantNumeric: 'tabular-nums' }}>
                        {fmtMs(e.duration_ms)}
                      </span>
                      <span style={{ fontSize: 12, opacity: 0.7, fontVariantNumeric: 'tabular-nums' }}>
                        {e.at.slice(11, 19)}
                      </span>
                    </div>

                    <div style={{ fontSize: 12, opacity: 0.8, whiteSpace: 'nowrap' }}>
                      {e.method}
                    </div>
                  </div>

                  <div style={{ marginTop: 6, fontSize: 13, lineHeight: 1.25, opacity: 0.95, wordBreak: 'break-word' }}>
                    <span style={{ opacity: 0.85 }}>{e.path}</span>
                  </div>

                  {e.error ? (
                    <div style={{ marginTop: 6, fontSize: 12, opacity: 0.8, color: 'rgba(253,164,175,0.92)' }}>
                      {e.error}
                    </div>
                  ) : null}
                </div>
              )
            })
          )}
        </div>

        <div style={detailStyle}>
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: 10 }}>
            <div style={{ fontSize: 13, fontWeight: 700, opacity: 0.95 }}>
              Details
            </div>
            <div style={{ fontSize: 12, opacity: 0.75 }}>
              {selected ? selected.id.slice(0, 8) : '—'}
            </div>
          </div>

          {!selected ? (
            <div style={{ marginTop: 10, opacity: 0.75, fontSize: 13 }}>
              Click a request above to inspect status/error payload snippets.
            </div>
          ) : (
            <div style={{ marginTop: 10, display: 'grid', gap: 8 }}>
              <div style={{ fontSize: 13, opacity: 0.92 }}>
                <strong>When:</strong> {selected.at}
              </div>
              <div style={{ fontSize: 13, opacity: 0.92 }}>
                <strong>Method:</strong> {selected.method}
              </div>
              <div style={{ fontSize: 13, opacity: 0.92 }}>
                <strong>Path:</strong> {selected.path}
              </div>
              <div style={{ fontSize: 13, opacity: 0.92 }}>
                <strong>Status:</strong> {selected.status ?? '—'}
              </div>
              <div style={{ fontSize: 13, opacity: 0.92 }}>
                <strong>Duration:</strong> {fmtMs(selected.duration_ms)}
              </div>

              {selected.error ? (
                <div style={{ fontSize: 13, opacity: 0.92, color: 'rgba(253,164,175,0.92)' }}>
                  <strong>Error:</strong> {selected.error}
                </div>
              ) : null}

              {selected.body_snippet ? (
                <div>
                  <div style={{ fontSize: 13, fontWeight: 700, opacity: 0.95, marginTop: 6 }}>
                    Body snippet
                  </div>
                  <pre
                    style={{
                      marginTop: 8,
                      padding: 10,
                      borderRadius: 14,
                      border: '1px solid rgba(255,255,255,0.10)',
                      background: 'rgba(255,255,255,0.04)',
                      overflowX: 'auto',
                      fontSize: 12,
                      lineHeight: 1.35,
                      color: 'rgba(226,232,240,0.95)',
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-word',
                    }}
                  >
                    {selected.body_snippet}
                  </pre>
                </div>
              ) : (
                <div style={{ marginTop: 10, opacity: 0.75, fontSize: 13 }}>
                  No response body snippet recorded.
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
