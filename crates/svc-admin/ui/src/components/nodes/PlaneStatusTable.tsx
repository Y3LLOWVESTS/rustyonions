// crates/svc-admin/ui/src/components/nodes/PlaneStatusTable.tsx
//
// WHAT:
//   Planes status table for a node.
// WHY:
//   Fix column truncation / misalignment and style it like the sidebar rows.
//   Ensures all columns are visible even in a tighter container.
//
// NOTE:
//   This version removes horizontal scrolling and instead tightens column widths,
//   gaps, padding, and chip sizing so the last column doesn't get clipped.

import React, { useMemo } from 'react'

type PlaneLike = {
  name?: string
  plane?: string
  service?: string
  health?: 'healthy' | 'degraded' | 'down' | string
  status?: string
  state?: string
  ready?: boolean | string
  is_ready?: boolean | string
  isReady?: boolean | string
  restarts?: number
  restart_count?: number
  restartCount?: number
}

function toName(p: PlaneLike): string {
  return String(p?.name ?? p?.plane ?? p?.service ?? 'unknown')
}

function toHealth(p: PlaneLike): string {
  const raw = p?.health ?? p?.status ?? p?.state ?? 'unknown'
  const s = String(raw).toLowerCase().trim()
  if (s === 'healthy' || s === 'degraded' || s === 'down') return s
  return 'unknown'
}

function toReady(p: PlaneLike): boolean | null {
  const v = p?.ready ?? p?.is_ready ?? p?.isReady
  if (typeof v === 'boolean') return v
  if (typeof v === 'string') {
    const s = v.toLowerCase().trim()
    if (s === 'ready' || s === 'true' || s === 'ok') return true
    if (s === 'not_ready' || s === 'not ready' || s === 'false') return false
  }
  return null
}

function toRestarts(p: PlaneLike): number {
  const a = p?.restarts
  if (typeof a === 'number' && Number.isFinite(a)) return Math.max(0, Math.floor(a))
  const b = p?.restart_count
  if (typeof b === 'number' && Number.isFinite(b)) return Math.max(0, Math.floor(b))
  const c = p?.restartCount
  if (typeof c === 'number' && Number.isFinite(c)) return Math.max(0, Math.floor(c))
  return 0
}

function chipStyle(tone: 'ok' | 'warn' | 'bad' | 'muted'): React.CSSProperties {
  const map: Record<typeof tone, { bg: string; fg: string; bd: string }> = {
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
      fg: 'rgba(255,255,255,0.86)',
      bd: 'rgba(255,255,255,0.10)',
    },
  }
  const c = map[tone]
  return {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: '2px 6px', // tighter
    borderRadius: 999,
    fontSize: 10, // tighter
    fontWeight: 850,
    background: c.bg,
    color: c.fg,
    border: `1px solid ${c.bd}`,
    whiteSpace: 'nowrap',
    minWidth: 52, // tighter
    lineHeight: 1.2,
  }
}

function healthTone(h: string): 'ok' | 'warn' | 'bad' | 'muted' {
  if (h === 'healthy') return 'ok'
  if (h === 'degraded') return 'warn'
  if (h === 'down') return 'bad'
  return 'muted'
}

export function PlaneStatusTable({ planes }: { planes: unknown[] }) {
  const rows = useMemo(() => {
    const list: PlaneLike[] = Array.isArray(planes) ? (planes as PlaneLike[]) : []
    return list.map((p) => {
      const name = toName(p)
      const health = toHealth(p)
      const ready = toReady(p)
      const restarts = toRestarts(p)
      return { name, health, ready, restarts }
    })
  }, [planes])

  if (!rows.length) {
    return (
      <div
        style={{
          borderRadius: 18,
          border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
          background:
            'linear-gradient(180deg, rgba(255,255,255,0.03), rgba(255,255,255,0.012))',
          padding: 14,
          opacity: 0.85,
        }}
      >
        No planes reported yet.
      </div>
    )
  }

  // No horizontal scroll: keep columns tight so the whole table fits.
  // Plane | Health | Ready | Restarts
  const gridCols = 'minmax(84px, 1fr) 68px 68px 70px'

  return (
    <div
      style={{
        borderRadius: 18,
        border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
        background:
          'linear-gradient(180deg, rgba(255,255,255,0.03), rgba(255,255,255,0.012))',
        boxShadow: '0 14px 34px rgba(0,0,0,0.20)',
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: gridCols,
          gap: 6, // tighter than before
          padding: '10px 10px', // tighter than before
          borderBottom: '1px solid rgba(255,255,255,0.08)',
          fontSize: 10.5, // tighter so "RESTARTS" fits
          fontWeight: 900,
          letterSpacing: '0.01em', // tighter
          textTransform: 'uppercase',
          opacity: 0.9,
          alignItems: 'center',
        }}
      >
        <div>Plane</div>
        <div style={{ textAlign: 'center' }}>Health</div>
        <div style={{ textAlign: 'center' }}>Ready</div>
        <div
          style={{
            textAlign: 'right',
            overflow: 'hidden',
            textOverflow: 'clip',
            whiteSpace: 'nowrap',
          }}
        >
          Restarts
        </div>
      </div>

      {/* Rows */}
      <div style={{ display: 'flex', flexDirection: 'column' }}>
        {rows.map((r, idx) => {
          const hTone = healthTone(r.health)
          const healthLabel =
            r.health === 'unknown'
              ? 'Unknown'
              : r.health.charAt(0).toUpperCase() + r.health.slice(1)

          const readyTone = r.ready === true ? 'ok' : r.ready === false ? 'warn' : 'muted'
          const readyLabel =
            r.ready === true ? 'Ready' : r.ready === false ? 'Not ready' : 'Unknown'

          const restartTone = r.restarts > 0 ? 'warn' : 'muted'

          return (
            <div
              key={`${r.name}:${idx}`}
              style={{
                display: 'grid',
                gridTemplateColumns: gridCols,
                gap: 6, // tighter
                alignItems: 'center',
                padding: '10px 10px', // tighter
                borderBottom: '1px solid rgba(255,255,255,0.06)',
                background: 'rgba(255,255,255,0.01)',
              }}
            >
              <div
                style={{
                  fontWeight: 900,
                  fontSize: 13,
                  minWidth: 0,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
                title={r.name}
              >
                {r.name}
              </div>

              <div style={{ textAlign: 'center' }}>
                <span style={chipStyle(hTone)}>{healthLabel}</span>
              </div>

              <div style={{ textAlign: 'center' }}>
                <span style={chipStyle(readyTone)}>{readyLabel}</span>
              </div>

              <div style={{ textAlign: 'right' }}>
                <span style={chipStyle(restartTone)}>{r.restarts}</span>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
