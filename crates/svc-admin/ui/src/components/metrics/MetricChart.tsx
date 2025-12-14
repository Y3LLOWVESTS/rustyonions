// crates/svc-admin/ui/src/components/metrics/MetricChart.tsx

import React from 'react'

type Props = {
  facet: string
  rps: number
  errorRate: number
  p95: number
  p99: number
  history?: number[]
}

function formatLatency(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return 'n/a'
  if (ms < 1) return `${ms.toFixed(2)} ms`
  if (ms < 10) return `${ms.toFixed(1)} ms`
  if (ms < 1000) return `${Math.round(ms)} ms`
  const s = ms / 1000
  if (s < 10) return `${s.toFixed(1)} s`
  return `${Math.round(s)} s`
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, n))
}

export function MetricChart({
  facet,
  rps,
  errorRate,
  p95,
  p99,
  history,
}: Props) {
  const series = history && history.length > 0 ? history : [rps]

  const width = 80
  const height = 24
  const padding = 2

  const max = Math.max(...series)
  const min = Math.min(...series)
  const range = max - min || 1

  let points: string
  let lastX = width - padding
  let lastY = height / 2

  if (series.length === 1) {
    const midY = height / 2
    points = `${padding},${midY} ${width - padding},${midY}`
    lastX = width - padding
    lastY = midY
  } else {
    const coords = series.map((value, idx) => {
      const t = idx / (series.length - 1)
      const x = padding + t * (width - padding * 2)
      const norm = (value - min) / range
      const y = height - padding - norm * (height - padding * 2)
      return { x, y }
    })

    points = coords.map((c) => `${c.x},${c.y}`).join(' ')
    const last = coords[coords.length - 1]
    lastX = last.x
    lastY = last.y
  }

  // Keep the dot fully inside the viewBox even when it “breathes”.
  const dotRMin = 0.75
  const dotRMax = 1.0
  const dotPad = dotRMax + 0.55
  const dotX = clamp(lastX, dotPad, width - dotPad)
  const dotY = clamp(lastY, dotPad, height - dotPad)

  const rpsLabel = `${rps.toFixed(1)} rps`
  const errorLabel =
    errorRate > 0 ? `${(errorRate * 100).toFixed(1)}% errors` : null

  const topY = padding
  const bottomY = height - padding

  return (
    <article className="svc-admin-metric-chart">
      <header className="svc-admin-metric-chart-header">
        <span className="svc-admin-metric-chart-facet">{facet}</span>
        <span className="svc-admin-metric-chart-summary">
          <span className="svc-admin-metric-chart-summary-main">{rpsLabel}</span>
          {errorLabel && (
            <span className="svc-admin-metric-chart-summary-error">
              {errorLabel}
            </span>
          )}
        </span>
      </header>

      <div className="svc-admin-metric-chart-bars">
        <svg
          className="svc-admin-metric-chart-sparkline"
          viewBox={`0 0 ${width} ${height}`}
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          <line
            x1={padding}
            y1={topY}
            x2={width - padding}
            y2={topY}
            stroke="currentColor"
            strokeWidth={0.4}
            strokeOpacity={0.25}
          />
          <line
            x1={padding}
            y1={bottomY}
            x2={width - padding}
            y2={bottomY}
            stroke="currentColor"
            strokeWidth={0.4}
            strokeOpacity={0.25}
          />

          <polyline
            fill="none"
            stroke="currentColor"
            strokeWidth={0.25}
            strokeLinecap="round"
            strokeLinejoin="round"
            points={points}
          />

          {/* “Breathing” dot anchored to the last point (no transform = no bobbing). */}
          <circle
            className="svc-admin-metric-chart-last-dot"
            cx={dotX}
            cy={dotY}
            r={dotRMin}
            fill="currentColor"
          >
            <animate
              attributeName="r"
              values={`${dotRMin};${dotRMax};${dotRMin}`}
              dur="3.25s"
              repeatCount="indefinite"
            />
            <animate
              attributeName="opacity"
              values="0.9;0.5;0.9"
              dur="3.25s"
              repeatCount="indefinite"
            />

          </circle>
        </svg>
      </div>

      <footer className="svc-admin-metric-chart-footer">
        <span className="svc-admin-metric-chart-footer-latency">
          p95 {formatLatency(p95)} / p99 {formatLatency(p99)}
        </span>
      </footer>
    </article>
  )
}
