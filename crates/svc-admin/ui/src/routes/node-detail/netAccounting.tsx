// crates/svc-admin/ui/src/routes/node-detail/netAccounting.tsx
//
// WHAT:
//   Network accounting polling + presentation (minute/hour/day/month + sparkline).
// WHY:
//   Keep NodeDetailPage thin; this panel is reusable later for cluster-wide aggregation.
// INVARIANTS:
//   - Optional endpoint: if 404/405/501, show capability-rollout message (no crash).
//   - SVG sparkline uses normalized line; always renders safely.

import React, { useEffect, useMemo, useState } from 'react'
import { adminClient } from '../../api/adminClient'
import type { NetAccountingDto } from '../../types/admin-api'

type FetchErr = Error & { status?: number }

function isMissingEndpoint(err: unknown): boolean {
  const e = err as FetchErr
  const s = e && typeof e.status === 'number' ? e.status : undefined
  if (s === 404 || s === 405 || s === 501) return true

  const msg = e?.message ?? ''
  return (
    msg.includes(' 404 ') ||
    msg.includes(' 405 ') ||
    msg.includes(' 501 ') ||
    msg.toLowerCase().includes('not found') ||
    msg.toLowerCase().includes('not implemented')
  )
}

function formatBytes(n: number | null | undefined): string {
  const v = typeof n === 'number' && Number.isFinite(n) ? Math.max(0, n) : 0
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB']
  let x = v
  let i = 0
  while (x >= 1024 && i < units.length - 1) {
    x /= 1024
    i++
  }
  if (i === 0) return `${Math.round(x)} ${units[i]}`
  return `${x.toFixed(x >= 10 ? 1 : 2)} ${units[i]}`
}

function formatBps(n: number | null | undefined): string {
  if (typeof n !== 'number' || !Number.isFinite(n)) return '—'
  return `${formatBytes(n)}/s`
}

function formatInt(n: number | null | undefined): string {
  if (typeof n !== 'number' || !Number.isFinite(n)) return '—'
  return Math.round(n).toLocaleString()
}

type NetResolution = 'minute' | 'hour' | 'day' | 'month'

function pickSeries(dto: NetAccountingDto | null, res: NetResolution) {
  if (!dto) return []
  if (res === 'minute') return (dto as any).seriesMinute ?? []
  if (res === 'hour') return (dto as any).seriesHour ?? []
  if (res === 'day') return (dto as any).seriesDay ?? []
  return (dto as any).seriesMonth ?? []
}

function pickRollup(dto: NetAccountingDto | null, res: NetResolution) {
  if (!dto) return null
  if (res === 'minute') return (dto as any).minute
  if (res === 'hour') return (dto as any).hour
  if (res === 'day') return (dto as any).day
  return (dto as any).month
}

function numField(obj: any, keys: string[]): number | null {
  if (!obj || typeof obj !== 'object') return null
  for (const k of keys) {
    const v = obj[k]
    if (typeof v === 'number' && Number.isFinite(v)) return v
  }
  return null
}

function normalizeSeriesPoint(p: any, fallbackIndex: number) {
  const bytes = numField(p, ['totalBytes', 'bytes', 'b']) ?? 0
  const req = numField(p, ['requests', 'req', 'r']) ?? 0
  const t = numField(p, ['ts', 't', 'time', 'at', 'epoch', 'epochSeconds']) ?? fallbackIndex
  return { t, bytes, req }
}

function makeSparkPath(values: number[], w: number, h: number, pad: number): string {
  if (!values || values.length === 0) return ''
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

  return pts
    .map((p, i) => `${i === 0 ? 'M' : 'L'}${p[0].toFixed(2)},${p[1].toFixed(2)}`)
    .join(' ')
}

function topFacetPairs(obj: Record<string, number> | null | undefined, limit: number) {
  if (!obj) return []
  return Object.entries(obj)
    .filter(([, v]) => typeof v === 'number' && Number.isFinite(v))
    .sort((a, b) => b[1] - a[1])
    .slice(0, limit)
}

export function useNetAccounting(nodeId: string, opts: { enabled: boolean; intervalMs: number }) {
  const [net, setNet] = useState<NetAccountingDto | null>(null)
  const [missing, setMissing] = useState(false)

  useEffect(() => {
    if (!nodeId) {
      setNet(null)
      setMissing(false)
      return
    }

    let cancelled = false
    let timer: number | null = null

    const shouldPollNow = () => {
      if (typeof document !== 'undefined' && document.visibilityState === 'hidden') return false
      return true
    }

    const tick = async () => {
      if (cancelled) return
      if (!shouldPollNow()) return
      try {
        const dto = await adminClient.getNodeSystemNetAccounting(nodeId)
        if (cancelled) return
        if (dto == null) {
          setNet(null)
          setMissing(true)
        } else {
          setNet(dto)
          setMissing(false)
        }
      } catch (err) {
        if (cancelled) return
        if (isMissingEndpoint(err)) {
          setNet(null)
          setMissing(true)
        }
      }
    }

    void tick()
    if (opts.enabled) {
      timer = window.setInterval(() => void tick(), Math.max(750, opts.intervalMs))
    }

    const onVis = () => {
      if (typeof document !== 'undefined' && document.visibilityState === 'visible') void tick()
    }
    document.addEventListener?.('visibilitychange', onVis)

    return () => {
      cancelled = true
      if (timer != null) window.clearInterval(timer)
      document.removeEventListener?.('visibilitychange', onVis)
    }
  }, [nodeId, opts.enabled, opts.intervalMs])

  return { net, missing }
}

export function NetAccountingPanel(props: { net: NetAccountingDto | null; missing: boolean }) {
  const { net, missing } = props
  const [res, setRes] = useState<NetResolution>('minute')
  const [showFacets, setShowFacets] = useState(false)

  const roll = useMemo(() => pickRollup(net, res), [net, res])
  const seriesRaw = useMemo(() => pickSeries(net, res), [net, res])

  const series = useMemo(() => (seriesRaw ?? []).map((p, i) => normalizeSeriesPoint(p, i)), [seriesRaw])
  const bytesSeries = useMemo(() => series.map((p) => p.bytes), [series])
  const reqSeries = useMemo(() => series.map((p) => p.req), [series])

  const bytesPath = useMemo(() => makeSparkPath(bytesSeries, 520, 78, 6), [bytesSeries])
  const reqPath = useMemo(() => makeSparkPath(reqSeries, 520, 78, 6), [reqSeries])

  const minutePairs = useMemo(() => topFacetPairs((net as any)?.minute?.requestsByFacet ?? null, 6), [net])
  const hourPairs = useMemo(() => topFacetPairs((net as any)?.hour?.requestsByFacet ?? null, 6), [net])
  const dayPairs = useMemo(() => topFacetPairs((net as any)?.day?.requestsByFacet ?? null, 6), [net])
  const monthPairs = useMemo(() => topFacetPairs((net as any)?.month?.requestsByFacet ?? null, 6), [net])

  const selectedFacetPairs = useMemo(() => {
    if (res === 'minute') return minutePairs
    if (res === 'hour') return hourPairs
    if (res === 'day') return dayPairs
    return monthPairs
  }, [res, minutePairs, hourPairs, dayPairs, monthPairs])

  const rollupRow = (label: string, r: any) => {
    const bytes = r?.totalBytes
    const req = r?.requests
    return (
      <div
        className="svc-admin-card"
        style={{
          padding: '10px 12px',
          borderRadius: 14,
          border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
          background: 'rgba(255,255,255,0.02)',
          display: 'flex',
          flexDirection: 'column',
          gap: 4,
          minWidth: 160,
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: 10 }}>
          <strong style={{ fontSize: '0.92rem' }}>{label}</strong>
          <span style={{ opacity: 0.65, fontSize: '0.85rem' }}>
            {r?.observedSeconds != null ? `${Math.round(r.observedSeconds)}s` : '—'}
          </span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: 10 }}>
          <span style={{ opacity: 0.8 }}>Bytes</span>
          <span style={{ fontVariantNumeric: 'tabular-nums' }}>{formatBytes(bytes)}</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: 10 }}>
          <span style={{ opacity: 0.8 }}>Requests</span>
          <span style={{ fontVariantNumeric: 'tabular-nums' }}>{formatInt(req)}</span>
        </div>
      </div>
    )
  }

  if (missing) {
    return (
      <div style={{ marginTop: 14 }}>
        <div className="svc-admin-card" style={{ padding: 12, borderRadius: 16 }}>
          <strong>Network accounting</strong>
          <p style={{ margin: '6px 0 0 0', opacity: 0.8 }}>
            This node does not implement <code>/api/v1/system/net/accounting</code> yet (capability rollout).
          </p>
        </div>
      </div>
    )
  }

  return (
    <div style={{ marginTop: 14 }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'baseline',
          justifyContent: 'space-between',
          gap: 12,
          flexWrap: 'wrap',
          marginBottom: 10,
        }}
      >
        <div>
          <h3 style={{ margin: 0, fontSize: '1.05rem' }}>Network accounting</h3>
          <div style={{ opacity: 0.78, fontSize: '0.9rem', marginTop: 2 }}>
            RX {formatBps((net as any)?.rxBps)} · TX {formatBps((net as any)?.txBps)}
          </div>
        </div>

        <div style={{ display: 'flex', gap: 10, alignItems: 'center', flexWrap: 'wrap' }}>
          <select
            value={res}
            onChange={(e) => setRes(e.target.value as NetResolution)}
            style={{
              borderRadius: 10,
              padding: '6px 8px',
              border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
              background: 'rgba(255,255,255,0.03)',
              color: 'var(--svc-admin-color-text)',
            }}
            title="Select the series resolution for the sparkline."
          >
            <option value="minute">Minute (60)</option>
            <option value="hour">Hour (24)</option>
            <option value="day">Day (30)</option>
            <option value="month">Month (12)</option>
          </select>

          <button
            type="button"
            className="svc-admin-node-action-button"
            style={{ padding: '6px 10px', borderRadius: 12 }}
            onClick={() => setShowFacets((v) => !v)}
          >
            {showFacets ? 'Hide facets' : 'Show facets'}
          </button>
        </div>
      </div>

      <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginBottom: 12 }}>
        {rollupRow('Minute', (net as any)?.minute)}
        {rollupRow('Hour', (net as any)?.hour)}
        {rollupRow('Day', (net as any)?.day)}
        {rollupRow('Month', (net as any)?.month)}
      </div>

      <div
        className="svc-admin-card"
        style={{
          padding: 12,
          borderRadius: 16,
          border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
          background: 'rgba(255,255,255,0.02)',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12, flexWrap: 'wrap' }}>
          <div style={{ opacity: 0.85 }}>
            <strong>Series</strong> <span style={{ opacity: 0.7 }}>(bytes + requests, normalized)</span>
          </div>
          <div style={{ opacity: 0.7, fontSize: '0.9rem' }}>
            Selected window total: {formatBytes(roll?.totalBytes)} · {formatInt(roll?.requests)} req
          </div>
        </div>

        <svg viewBox="0 0 520 78" width="100%" height="78" style={{ marginTop: 10, display: 'block' }}>
          <path d={bytesPath} fill="none" stroke="currentColor" strokeWidth="2" opacity="0.95" />
          <path d={reqPath} fill="none" stroke="currentColor" strokeWidth="2" opacity="0.35" />
        </svg>

        {showFacets && (
          <div style={{ marginTop: 12 }}>
            <strong style={{ fontSize: '0.95rem' }}>Top facets ({res})</strong>
            {selectedFacetPairs.length === 0 ? (
              <div style={{ opacity: 0.75, marginTop: 6 }}>No facet breakdown available.</div>
            ) : (
              <div
                style={{
                  marginTop: 8,
                  display: 'grid',
                  gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))',
                  gap: 8,
                }}
              >
                {selectedFacetPairs.map(([k, v]) => (
                  <div
                    key={k}
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      gap: 10,
                      padding: '8px 10px',
                      borderRadius: 12,
                      border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
                      background: 'rgba(255,255,255,0.02)',
                      fontVariantNumeric: 'tabular-nums',
                    }}
                  >
                    <span style={{ opacity: 0.9 }}>{k}</span>
                    <span style={{ opacity: 0.85 }}>{Math.round(v).toLocaleString()}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
