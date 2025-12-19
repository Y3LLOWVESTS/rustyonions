// crates/svc-admin/ui/src/routes/node-storage/helpers.ts
//
// RO:WHAT — Small helpers (endpoint detection + health/metrics derivations).
// RO:WHY  — Keep the route file thin and avoid duplicating heuristics.

import type { FacetMetricsSummary } from '../../types/admin-api'

type FetchErr = Error & { status?: number }

export function isMissingEndpoint(err: unknown): boolean {
  const e = err as FetchErr
  const s = e && typeof e.status === 'number' ? e.status : undefined
  if (s === 404 || s === 405 || s === 501) return true

  // Fallback heuristic: handleResponse embeds status code into the message.
  const msg = e?.message ?? ''
  return (
    msg.includes(' 404 ') ||
    msg.includes(' 405 ') ||
    msg.includes(' 501 ') ||
    msg.toLowerCase().includes('not found') ||
    msg.toLowerCase().includes('not implemented')
  )
}

export function deriveOverallHealthOrNull(
  planes: Array<{ health: 'healthy' | 'degraded' | 'down' }> | undefined,
): 'healthy' | 'degraded' | 'down' | null {
  if (!planes || planes.length === 0) return null
  if (planes.some((p) => p.health === 'down')) return 'down'
  if (planes.some((p) => p.health === 'degraded')) return 'degraded'
  return 'healthy'
}

export function computeMetricsHealth(
  facets: FacetMetricsSummary[] | null,
  error: string | null,
): 'fresh' | 'stale' | 'unreachable' {
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
