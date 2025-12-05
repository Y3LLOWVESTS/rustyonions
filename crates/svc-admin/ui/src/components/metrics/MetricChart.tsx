// crates/svc-admin/ui/src/components/metrics/MetricChart.tsx

import React from 'react'

type Props = {
  facet: string
  rps: number
  errorRate: number
  p95: number
  p99: number
}

/**
 * Lightweight, CSS-only “chart” showing facet RPS and latency.
 *
 * This is intentionally simple: a header line with text + two bars whose
 * widths are proportional to RPS and latency. We can upgrade to a real
 * charting lib later without changing the calling code.
 */
export function MetricChart({ facet, rps, errorRate, p95, p99 }: Props) {
  const errorPercent = Math.round(errorRate * 1000) / 10 // 1 decimal place

  const rpsWidth = Math.min(rps, 100)
  const latencyWidth = Math.min(p95 / 10, 100)

  return (
    <div className="svc-admin-metric-chart">
      <div className="svc-admin-metric-chart-header">
        <strong>{facet}</strong>
        <span className="svc-admin-metric-chart-summary">
          {rps.toFixed(1)} req/s · {errorPercent}% errors
        </span>
      </div>

      <div className="svc-admin-metric-chart-bars">
        <div
          className="svc-admin-metric-chart-bar svc-admin-metric-chart-bar-rps"
          style={{ width: `${rpsWidth}%` }}
        />
        <div
          className="svc-admin-metric-chart-bar svc-admin-metric-chart-bar-latency"
          style={{ width: `${latencyWidth}%` }}
        />
      </div>

      <div className="svc-admin-metric-chart-footer">
        <span>p95: {p95.toFixed(1)} ms</span>
        <span>p99: {p99.toFixed(1)} ms</span>
      </div>
    </div>
  )
}
