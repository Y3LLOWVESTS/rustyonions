// crates/svc-admin/ui/src/routes/node-list/helpers.ts
//
// RO:WHAT — Helpers for NodeListPage view-model (health + summaries + metrics freshness).
// RO:WHY  — Keep the route file thin and keep logic shared with future polling/refetch.

import type { AdminStatusView, PlaneStatus, FacetMetricsSummary } from '../../types/admin-api'
import type { NodeStatusSummary, MetricsHealth } from '../../components/nodes/NodeCard'

type Health = 'healthy' | 'degraded' | 'down'

export function deriveOverallHealth(planes: PlaneStatus[]): Health {
  if (!planes.length) return 'degraded'
  if (planes.some((p) => p.health === 'down')) return 'down'
  if (planes.some((p) => p.health === 'degraded')) return 'degraded'
  return 'healthy'
}

export function buildSummary(status: AdminStatusView | null): NodeStatusSummary | undefined {
  if (!status) return undefined

  const planeCount = status.planes.length
  const readyCount = status.planes.filter((p) => p.ready).length
  const totalRestarts = status.planes.reduce((sum, p) => sum + (p.restart_count ?? 0), 0)

  return {
    overallHealth: deriveOverallHealth(status.planes),
    planeCount,
    readyCount,
    totalRestarts,
  }
}

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
