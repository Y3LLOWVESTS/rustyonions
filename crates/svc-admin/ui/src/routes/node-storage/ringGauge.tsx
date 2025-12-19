// crates/svc-admin/ui/src/routes/node-storage/ringGauge.tsx
//
// RO:WHAT — Ring gauge widget + DB icon for NodeStoragePage.
// RO:WHY  — Keep UI primitives out of the route file; reuse across storage subroutes.

import React from 'react'

export type GaugeLevel = 'ok' | 'warn' | 'near' | 'crit'

// Interprets "how full" as "how much of node disk is consumed by this DB".
export function gaugeLevelFromPct(pct: number): GaugeLevel {
  if (pct >= 60) return 'crit'
  if (pct >= 40) return 'near'
  if (pct >= 20) return 'warn'
  return 'ok'
}

export function gaugeColor(level: GaugeLevel): string {
  // Prefer theme vars; provide safe fallbacks so we don’t break if a var is missing.
  switch (level) {
    case 'ok':
      return 'var(--svc-admin-color-accent, #3b82f6)'
    case 'warn':
      return 'var(--svc-admin-color-warning-text, #facc15)'
    case 'near':
      return 'var(--svc-admin-color-warning, #fb923c)'
    case 'crit':
      return 'var(--svc-admin-color-danger-text, #ef4444)'
  }
}

export function RingGauge(props: {
  pct: number // 0..100
  label: string
  sublabel?: string
  color: string
  onClick?: () => void
}) {
  const pct = Math.max(0, Math.min(100, props.pct))
  const radius = 36
  const stroke = 8
  const c = 2 * Math.PI * radius
  const dash = (pct / 100) * c
  const gap = c - dash

  return (
    <div
      className="svc-admin-card"
      style={{
        padding: '0.9rem 1rem',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: '0.55rem',
        cursor: props.onClick ? 'pointer' : 'default',
        userSelect: 'none',
      }}
      onClick={props.onClick}
      title={props.label}
    >
      <div style={{ position: 'relative', width: 96, height: 96 }}>
        <svg width="96" height="96" viewBox="0 0 96 96" aria-hidden="true">
          <circle
            cx="48"
            cy="48"
            r={radius}
            fill="none"
            stroke="var(--svc-admin-color-border, rgba(255,255,255,0.12))"
            strokeWidth={stroke}
            opacity={0.9}
          />
          <circle
            cx="48"
            cy="48"
            r={radius}
            fill="none"
            stroke={props.color}
            strokeWidth={stroke}
            strokeLinecap="round"
            strokeDasharray={`${dash} ${gap}`}
            transform="rotate(-90 48 48)"
          />
        </svg>

        <div
          style={{
            position: 'absolute',
            inset: 0,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: '0.1rem',
          }}
        >
          <div style={{ fontSize: '1.05rem', fontWeight: 800, lineHeight: 1 }}>
            {pct.toFixed(0)}%
          </div>
          <div style={{ fontSize: '0.75rem', opacity: 0.85, lineHeight: 1 }}>
            of disk
          </div>
        </div>
      </div>

      <div style={{ textAlign: 'center', width: '100%' }}>
        <div style={{ fontWeight: 750, fontSize: '0.95rem', overflowWrap: 'anywhere' }}>
          {props.label}
        </div>
        {props.sublabel && (
          <div style={{ fontSize: '0.8rem', opacity: 0.8, marginTop: '0.15rem' }}>
            {props.sublabel}
          </div>
        )}
      </div>
    </div>
  )
}

export function DbIcon() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      aria-hidden="true"
      style={{ opacity: 0.9 }}
    >
      <path
        d="M12 2c4.97 0 9 1.79 9 4s-4.03 4-9 4-9-1.79-9-4 4.03-4 9-4Z"
        fill="var(--svc-admin-color-accent, #3b82f6)"
        opacity="0.35"
      />
      <path
        d="M3 6v6c0 2.21 4.03 4 9 4s9-1.79 9-4V6"
        fill="none"
        stroke="var(--svc-admin-color-accent, #3b82f6)"
        strokeWidth="1.6"
        opacity="0.95"
      />
      <path
        d="M3 12v6c0 2.21 4.03 4 9 4s9-1.79 9-4v-6"
        fill="none"
        stroke="var(--svc-admin-color-accent, #3b82f6)"
        strokeWidth="1.6"
        opacity="0.75"
      />
      <path
        d="M3 6c0 2.21 4.03 4 9 4s9-1.79 9-4"
        fill="none"
        stroke="var(--svc-admin-color-accent, #3b82f6)"
        strokeWidth="1.6"
        opacity="0.95"
      />
    </svg>
  )
}
