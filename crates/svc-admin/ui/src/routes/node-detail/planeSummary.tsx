// crates/svc-admin/ui/src/routes/node-detail/planeSummary.tsx
//
// WHAT:
//   Plane normalization helpers + summary pill UI.
// WHY:
//   Keep route thin and avoid duplicating plane shape normalizations across screens.

import React from 'react'
import type { Health } from './health'

export function planeRestarts(p: any): number {
  const a = p?.restarts
  if (typeof a === 'number' && Number.isFinite(a)) return a
  const b = p?.restart_count
  if (typeof b === 'number' && Number.isFinite(b)) return b
  const c = p?.restartCount
  if (typeof c === 'number' && Number.isFinite(c)) return c
  return 0
}

export function planeReadyBool(p: any): boolean | null {
  const r = p?.ready
  if (typeof r === 'boolean') return r
  if (typeof r === 'string') {
    const s = r.toLowerCase().trim()
    if (s === 'ready' || s === 'true' || s === 'ok') return true
    if (s === 'not_ready' || s === 'not ready' || s === 'false') return false
  }
  return null
}

export function planeHealthNorm(p: any): Health | 'unknown' {
  const h = String(p?.health ?? '').toLowerCase().trim()
  if (h === 'healthy') return 'healthy'
  if (h === 'degraded') return 'degraded'
  if (h === 'down') return 'down'
  return 'unknown'
}

export function computePlaneSummary(planes: any[]) {
  const total = planes.length
  const ready = planes.filter((p) => planeReadyBool(p) === true).length
  const degraded = planes.filter((p) => planeHealthNorm(p) === 'degraded').length
  const down = planes.filter((p) => planeHealthNorm(p) === 'down').length
  const restarts = planes.reduce((acc, p) => acc + planeRestarts(p), 0)
  return { total, ready, degraded, down, restarts }
}

export function Pill(props: {
  tone: 'ok' | 'warn' | 'bad' | 'muted'
  children: React.ReactNode
}) {
  const map: Record<
    typeof props.tone,
    { bg: string; fg: string; bd: string }
  > = {
    ok: {
      bg: 'rgba(16,185,129,0.10)',
      fg: 'rgba(110,231,183,0.95)',
      bd: 'rgba(16,185,129,0.22)',
    },
    warn: {
      bg: 'rgba(251,146,60,0.10)',
      fg: 'rgba(253,186,116,0.95)',
      bd: 'rgba(251,146,60,0.24)',
    },
    bad: {
      bg: 'rgba(244,63,94,0.10)',
      fg: 'rgba(253,164,175,0.95)',
      bd: 'rgba(244,63,94,0.24)',
    },
    muted: {
      bg: 'rgba(255,255,255,0.04)',
      fg: 'rgba(255,255,255,0.85)',
      bd: 'rgba(255,255,255,0.10)',
    },
  }

  const c = map[props.tone]
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        padding: '4px 8px',
        borderRadius: 999,
        fontSize: 12,
        fontWeight: 850,
        background: c.bg,
        color: c.fg,
        border: `1px solid ${c.bd}`,
        whiteSpace: 'nowrap',
      }}
    >
      {props.children}
    </span>
  )
}
