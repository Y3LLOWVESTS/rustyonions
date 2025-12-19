// crates/svc-admin/ui/src/routes/node-detail/utilization.tsx
//
// WHAT:
//   Deterministic mock utilization widgets (CPU/RAM/Storage/Bandwidth) for NodeDetail.
// WHY:
//   These are high-LOC UI primitives; keep them out of the route to preserve readability.
// INVARIANTS:
//   - Deterministic per nodeId (stable mock).
//   - Clamp percentages 0..100.
//   - Pure presentational components.

import React from 'react'

export function seedFromString(s: string): number {
  let acc = 0
  for (let i = 0; i < s.length; i++) acc = (acc * 31 + s.charCodeAt(i)) >>> 0
  return acc >>> 0
}

function clampPct(n: number): number {
  if (!Number.isFinite(n)) return 0
  return Math.max(0, Math.min(100, n))
}

export function mockNodeUtilization(nodeId: string): {
  cpuPct: number
  ramPct: number
  storagePct: number
  bandwidthPct: number
} {
  const seed = seedFromString(nodeId || 'node')
  const cpu = 12 + (seed % 79) // 12..90
  const ram = 25 + ((seed >>> 8) % 70) // 25..94
  const storage = 18 + ((seed >>> 16) % 78) // 18..95
  const bw = 8 + ((seed >>> 24) % 88) // 8..95
  return {
    cpuPct: clampPct(cpu),
    ramPct: clampPct(ram),
    storagePct: clampPct(storage),
    bandwidthPct: clampPct(bw),
  }
}

export function MiniMetricCard(props: {
  title: 'RAM' | 'CPU' | 'Storage' | 'Bandwidth'
  children: React.ReactNode
}) {
  return (
    <div
      style={{
        borderRadius: 16,
        background:
          'linear-gradient(180deg, rgba(255,255,255,0.035), rgba(255,255,255,0.015))',
        border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
        boxShadow: '0 10px 30px rgba(0,0,0,0.22)',
        padding: '8px 10px',
        minHeight: 80,
        overflow: 'hidden',
      }}
    >
      <div style={{ fontWeight: 900, letterSpacing: '0.02em' }}>
        {props.title}
      </div>
      <div style={{ marginTop: 10 }}>{props.children}</div>
    </div>
  )
}

export function ThermometerGauge(props: { pct: number; compact?: boolean }) {
  const pct = clampPct(props.pct)
  const compact = props.compact ?? false
  const fill = pct / 100

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: compact ? '0.75rem' : '1rem',
      }}
    >
      <div
        style={{
          width: compact ? 40 : 46,
          height: compact ? 96 : 118,
          borderRadius: 999,
          position: 'relative',
          overflow: 'hidden',
          background: 'rgba(255,255,255,0.04)',
          border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
          boxShadow: '0 0 0 1px rgba(0,0,0,0.18) inset',
          flex: '0 0 auto',
        }}
        aria-hidden="true"
      >
        <div
          style={{
            position: 'absolute',
            inset: 0,
            opacity: 0.25,
            background:
              'linear-gradient(to bottom, rgba(255,255,255,0.12) 1px, transparent 1px)',
            backgroundSize: '100% 14px',
            pointerEvents: 'none',
          }}
        />
        <div
          style={{
            position: 'absolute',
            left: 6,
            right: 6,
            bottom: 6,
            height: `calc(${fill * 100}% - 12px)`,
            minHeight: 10,
            borderRadius: 999,
            background:
              'linear-gradient(180deg, rgba(99,102,241,0.85) 0%, rgba(250,204,21,0.85) 55%, rgba(251,146,60,0.9) 80%, rgba(244,63,94,0.9) 100%)',
            filter: 'saturate(1.15)',
          }}
        />
        <div
          style={{
            position: 'absolute',
            inset: 0,
            background:
              'linear-gradient(90deg, rgba(255,255,255,0.18), rgba(255,255,255,0.02) 55%, rgba(255,255,255,0.08))',
            opacity: 0.12,
            pointerEvents: 'none',
          }}
        />
      </div>

      <div style={{ minWidth: 0 }}>
        <div
          style={{
            fontSize: compact ? '1.65rem' : '2rem',
            fontWeight: 900,
            lineHeight: 1,
          }}
        >
          {pct.toFixed(0)}%
        </div>
        <div
          style={{
            fontSize: compact ? '0.82rem' : '0.9rem',
            opacity: 0.78,
            marginTop: 4,
          }}
        >
          Memory in use (mock)
        </div>
      </div>
    </div>
  )
}

export function SpeedometerGauge(props: { pct: number; compact?: boolean }) {
  const pct = clampPct(props.pct)
  const compact = props.compact ?? false
  const angle = -110 + (pct / 100) * 220

  const cx = 78
  const cy = 82
  const r = 58

  const start = polarToCartesian(cx, cy, r, 200)
  const end = polarToCartesian(cx, cy, r, -20)
  const arcPath = describeArc(cx, cy, r, 200, -20)

  const svgW = compact ? 132 : 156
  const svgH = compact ? 98 : 118

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: compact ? '0.75rem' : '1rem',
      }}
    >
      <svg width={svgW} height={svgH} viewBox="0 0 156 118" aria-hidden="true">
        <path
          d={arcPath}
          fill="none"
          stroke="rgba(255,255,255,0.10)"
          strokeWidth="12"
          strokeLinecap="round"
        />
        <path
          d={describeArc(cx, cy, r, 200, 200 - (pct / 100) * 220)}
          fill="none"
          stroke="rgba(99,102,241,0.85)"
          strokeWidth="12"
          strokeLinecap="round"
        />
        {Array.from({ length: 9 }).map((_, i) => {
          const a = 200 - (i / 8) * 220
          const p1 = polarToCartesian(cx, cy, r + 2, a)
          const p2 = polarToCartesian(cx, cy, r - 10, a)
          return (
            <line
              key={i}
              x1={p1.x}
              y1={p1.y}
              x2={p2.x}
              y2={p2.y}
              stroke="rgba(255,255,255,0.12)"
              strokeWidth={i % 2 === 0 ? 2 : 1}
            />
          )
        })}

        <g transform={`rotate(${angle} ${cx} ${cy})`}>
          <line
            x1={cx}
            y1={cy}
            x2={cx + 54}
            y2={cy}
            stroke="rgba(191,219,254,0.95)"
            strokeWidth="3"
            strokeLinecap="round"
          />
          <circle cx={cx} cy={cy} r="7" fill="rgba(255,255,255,0.10)" />
          <circle cx={cx} cy={cy} r="4" fill="rgba(99,102,241,0.95)" />
        </g>

        <circle cx={start.x} cy={start.y} r="3" fill="rgba(251,146,60,0.7)" />
        <circle cx={end.x} cy={end.y} r="3" fill="rgba(16,185,129,0.55)" />
      </svg>

      <div style={{ minWidth: 0 }}>
        <div
          style={{
            fontSize: compact ? '1.65rem' : '2rem',
            fontWeight: 900,
            lineHeight: 1,
          }}
        >
          {pct.toFixed(0)}%
        </div>
        <div
          style={{
            fontSize: compact ? '0.82rem' : '0.9rem',
            opacity: 0.78,
            marginTop: 4,
          }}
        >
          CPU utilization (mock)
        </div>
      </div>
    </div>
  )
}

export function StorageWaffleGauge(props: { pct: number; compact?: boolean }) {
  const pct = clampPct(props.pct)
  const compact = props.compact ?? false

  const cols = 10
  const rows = 5
  const total = cols * rows
  const filled = Math.round((pct / 100) * total)

  const size = compact ? 12 : 14
  const gap = compact ? 4 : 5

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: compact ? '0.75rem' : '1rem',
      }}
    >
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${cols}, ${size}px)`,
          gridAutoRows: `${size}px`,
          gap: `${gap}px`,
          padding: 8,
          borderRadius: 14,
          background: 'rgba(255,255,255,0.03)',
          border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
          boxShadow: '0 0 0 1px rgba(0,0,0,0.18) inset',
          flex: '0 0 auto',
        }}
        aria-hidden="true"
      >
        {Array.from({ length: total }).map((_, i) => {
          const on = i < filled
          return (
            <div
              key={i}
              style={{
                width: size,
                height: size,
                borderRadius: 4,
                background: on
                  ? 'rgba(99,102,241,0.70)'
                  : 'rgba(255,255,255,0.07)',
                boxShadow: on ? '0 0 10px rgba(99,102,241,0.18)' : 'none',
              }}
            />
          )
        })}
      </div>

      <div style={{ minWidth: 0 }}>
        <div
          style={{
            fontSize: compact ? '1.65rem' : '2rem',
            fontWeight: 900,
            lineHeight: 1,
          }}
        >
          {pct.toFixed(0)}%
        </div>
        <div
          style={{
            fontSize: compact ? '0.82rem' : '0.9rem',
            opacity: 0.78,
            marginTop: 4,
          }}
        >
          Storage used (mock)
        </div>
      </div>
    </div>
  )
}

export function BandwidthBarsGauge(props: {
  pct: number
  seed: number
  compact?: boolean
}) {
  const pct = clampPct(props.pct)
  const compact = props.compact ?? false

  const bars = 14
  const active = Math.max(1, Math.round((pct / 100) * bars))

  const barW = compact ? 7 : 8
  const barGap = compact ? 5 : 6
  const maxH = compact ? 44 : 52
  const baseH = compact ? 14 : 16

  const colorForIndex = (idx: number) => {
    const t = idx / Math.max(1, bars - 1) // 0..1
    if (t < 0.6) return 'rgba(34,197,94,0.85)'
    if (t < 0.85) return 'rgba(250,204,21,0.85)'
    return 'rgba(244,63,94,0.85)'
  }

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: compact ? '0.75rem' : '1rem',
      }}
    >
      <div
        style={{
          padding: 10,
          borderRadius: 14,
          background: 'rgba(255,255,255,0.03)',
          border: '1px solid var(--svc-admin-color-border, rgba(255,255,255,0.12))',
          boxShadow: '0 0 0 1px rgba(0,0,0,0.18) inset',
          flex: '0 0 auto',
        }}
        aria-hidden="true"
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-end',
            gap: barGap,
            height: maxH,
          }}
        >
          {Array.from({ length: bars }).map((_, i) => {
            const jitter = ((props.seed >>> (i % 16)) & 0x7) / 7 // 0..1
            const h = Math.round(baseH + jitter * (maxH - baseH))
            const on = i < active
            const col = colorForIndex(i)

            return (
              <div
                key={i}
                style={{
                  width: barW,
                  height: h,
                  borderRadius: 999,
                  background: on ? col : 'rgba(255,255,255,0.08)',
                  boxShadow: on
                    ? `0 0 10px ${col.replace('0.85', '0.18')}`
                    : 'none',
                  opacity: on ? 1 : 0.85,
                }}
              />
            )
          })}
        </div>
      </div>

      <div style={{ minWidth: 0 }}>
        <div
          style={{
            fontSize: compact ? '1.65rem' : '2rem',
            fontWeight: 900,
            lineHeight: 1,
          }}
        >
          {pct.toFixed(0)}%
        </div>
        <div
          style={{
            fontSize: compact ? '0.82rem' : '0.9rem',
            opacity: 0.78,
            marginTop: 4,
          }}
        >
          Bandwidth in use (mock)
        </div>
      </div>
    </div>
  )
}

function polarToCartesian(cx: number, cy: number, r: number, angleDeg: number) {
  const a = ((angleDeg - 90) * Math.PI) / 180
  return { x: cx + r * Math.cos(a), y: cy + r * Math.sin(a) }
}

function describeArc(
  cx: number,
  cy: number,
  r: number,
  startAngle: number,
  endAngle: number,
) {
  const s = polarToCartesian(cx, cy, r, startAngle)
  const e = polarToCartesian(cx, cy, r, endAngle)
  const largeArcFlag = Math.abs(endAngle - startAngle) <= 180 ? '0' : '1'
  return `M ${s.x} ${s.y} A ${r} ${r} 0 ${largeArcFlag} 1 ${e.x} ${e.y}`
}
