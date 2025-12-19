// crates/svc-admin/ui/src/routes/node-detail/metricsHealth.tsx
//
// WHAT:
//   Metrics freshness classification + badge renderer.
// WHY:
//   Keep node detail route slim; centralize the freshness contract for reuse.
// INVARIANTS:
//   - error => unreachable
//   - no facets / no valid ages => stale
//   - min age <= threshold => fresh else stale

import React from 'react'
import type { FacetMetricsSummary } from '../../types/admin-api'
import type { MetricsHealth } from '../../components/nodes/NodeCard'

export function classifyMetricsHealth(
  facets: FacetMetricsSummary[] | null,
  error: string | null,
): MetricsHealth | null {
  if (error) return 'unreachable'
  if (!facets || facets.length === 0) return 'stale'

  const ages = facets
    .map((f) => f.last_sample_age_secs)
    .filter((v): v is number => v !== null && Number.isFinite(v))

  if (ages.length === 0) return 'stale'

  const minAge = Math.min(...ages)
  const FRESH_THRESHOLD_SECS = 30
  return minAge <= FRESH_THRESHOLD_SECS ? 'fresh' : 'stale'
}

export function renderMetricsBadge(health: MetricsHealth | null) {
  if (!health) return null

  const base =
    'inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-semibold'

  if (health === 'fresh') {
    return (
      <span className={`${base} bg-emerald-500/10 text-emerald-300`}>
        Metrics: fresh
      </span>
    )
  }

  if (health === 'stale') {
    return (
      <span className={`${base} bg-amber-500/10 text-amber-300`}>
        Metrics: stale
      </span>
    )
  }

  return (
    <span className={`${base} bg-rose-500/10 text-rose-300`}>
      Metrics: unreachable
    </span>
  )
}
