import React, { useMemo } from 'react'
import type { PlaneStatus } from '../../types/admin-api'
import {
  Pill,
  computePlaneSummary,
  planeHealthNorm,
  planeReadyBool,
  planeRestarts,
} from './planeSummary'

export function AllPlanesPanel(props: {
  planes: PlaneStatus[] | any[] | null | undefined
  title?: string
  // Optional pill node (you already show “Metrics: fresh” in the right panel)
  rightPill?: React.ReactNode
}) {
  const planesRaw = props.planes ?? []
  const planes = Array.isArray(planesRaw) ? planesRaw : []

  const summary = useMemo(() => computePlaneSummary(planes), [planes])

  const sorted = useMemo(() => {
    // worst-first: down > degraded > unknown > healthy; not-ready > ready; more restarts first
    const healthRank = (h: ReturnType<typeof planeHealthNorm>) => {
      if (h === 'down') return 0
      if (h === 'degraded') return 1
      if (h === 'unknown') return 2
      return 3
    }

    const readyRank = (r: ReturnType<typeof planeReadyBool>) => {
      if (r === false) return 0
      if (r === null) return 1
      return 2
    }

    return [...planes].sort((a, b) => {
      const ha = healthRank(planeHealthNorm(a))
      const hb = healthRank(planeHealthNorm(b))
      if (ha !== hb) return ha - hb

      const ra = readyRank(planeReadyBool(a))
      const rb = readyRank(planeReadyBool(b))
      if (ra !== rb) return ra - rb

      const pa = planeRestarts(a)
      const pb = planeRestarts(b)
      if (pa !== pb) return pb - pa

      const na = String((a as any)?.name ?? '').toLowerCase()
      const nb = String((b as any)?.name ?? '').toLowerCase()
      return na.localeCompare(nb)
    })
  }, [planes])

  return (
    <div style={card}>
      <div style={hdrRow}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <div style={titleStyle}>{props.title ?? 'All planes'}</div>
          {props.rightPill ? props.rightPill : null}
        </div>

        <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
          <Pill tone={summary.ready === summary.total ? 'ok' : summary.ready === 0 ? 'bad' : 'warn'}>
            {summary.ready}/{summary.total} ready
          </Pill>
          {summary.degraded > 0 ? <Pill tone="warn">{summary.degraded} degraded</Pill> : null}
          {summary.down > 0 ? <Pill tone="bad">{summary.down} down</Pill> : null}
          <Pill tone="muted">{summary.restarts} restarts</Pill>
        </div>
      </div>

      <div style={sub}>
        Health / readiness / restarts for every plane reported by this node.
      </div>

      <div style={list}>
        {sorted.length === 0 ? (
          <div style={empty}>No planes reported yet.</div>
        ) : (
          sorted.map((p) => <PlaneRow key={String((p as any)?.name ?? Math.random())} p={p} />)
        )}
      </div>
    </div>
  )
}

function PlaneRow({ p }: { p: any }) {
  const name = String(p?.name ?? 'unknown')
  const h = planeHealthNorm(p)
  const r = planeReadyBool(p)
  const restarts = planeRestarts(p)
  const age = planeAgeHint(p)

  return (
    <div style={row}>
      <div style={rowTop}>
        <div style={rowName}>{name}</div>

        <div style={rowRight}>
          {age ? <div style={ageTxt}>{age}</div> : null}

          <div style={badges}>
            <Pill tone={h === 'healthy' ? 'ok' : h === 'degraded' ? 'warn' : h === 'down' ? 'bad' : 'muted'}>
              {h === 'unknown' ? 'Unknown' : h[0].toUpperCase() + h.slice(1)}
            </Pill>

            <Pill tone={r === true ? 'ok' : r === false ? 'bad' : 'muted'}>
              {r === true ? 'Ready' : r === false ? 'Not ready' : 'Unknown'}
            </Pill>

            <Pill tone="muted">{restarts} restarts</Pill>
          </div>
        </div>
      </div>
    </div>
  )
}

// Optional: if your plane objects carry “age/latency/sample” info, we’ll show it.
// If not, it simply won’t render.
function planeAgeHint(p: any): string | null {
  const v =
    p?.age_s ??
    p?.ageSecs ??
    p?.sample_age_secs ??
    p?.last_sample_age_secs ??
    p?.ageMs ??
    p?.age_ms ??
    null

  if (v == null || !Number.isFinite(Number(v))) return null
  const n = Number(v)

  // if it looks like ms, convert
  const secs = n > 10_000 ? n / 1000 : n

  if (secs < 1) return `${Math.round(secs * 1000)}ms`
  if (secs < 10) return `${secs.toFixed(1)}s`
  if (secs < 60) return `${Math.round(secs)}s`
  const m = Math.floor(secs / 60)
  const s = Math.round(secs % 60)
  return `${m}m ${s}s`
}

/* styles */
const card: React.CSSProperties = {
  borderRadius: 16,
  border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
  background: 'linear-gradient(180deg, rgba(255,255,255,0.035), rgba(255,255,255,0.015))',
  boxShadow: '0 10px 30px rgba(0,0,0,0.22)',
  padding: 12,
}

const hdrRow: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'flex-start',
  gap: 12,
  flexWrap: 'wrap',
}

const titleStyle: React.CSSProperties = {
  fontWeight: 900,
  letterSpacing: '0.02em',
}

const sub: React.CSSProperties = {
  marginTop: 6,
  marginBottom: 10,
  fontSize: 12,
  opacity: 0.75,
}

const list: React.CSSProperties = {
  display: 'grid',
  gap: 10,
}

const empty: React.CSSProperties = {
  padding: 12,
  borderRadius: 14,
  border: '1px dashed var(--svc-admin-color-border, rgba(255,255,255,0.12))',
  opacity: 0.75,
  fontSize: 12,
}

const row: React.CSSProperties = {
  padding: 10,
  borderRadius: 14,
  border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
  background: 'rgba(255,255,255,0.03)',
}

const rowTop: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  gap: 10,
}

const rowName: React.CSSProperties = {
  fontSize: 13,
  fontWeight: 900,
  opacity: 0.92,
}

const rowRight: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 10,
  flexWrap: 'wrap',
  justifyContent: 'flex-end',
}

const badges: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 8,
  flexWrap: 'wrap',
  justifyContent: 'flex-end',
}

const ageTxt: React.CSSProperties = {
  fontSize: 12,
  opacity: 0.75,
  paddingRight: 2,
}
