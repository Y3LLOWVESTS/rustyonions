// crates/svc-admin/ui/src/types/admin-api.ts

export type UiConfigDto = {
  title: string
  subtitle?: string
  read_only: boolean
  default_theme: 'light' | 'dark' | 'system'
  default_locale: string
}

export type MeResponse = {
  id: string
  display_name: string
  roles: string[]
  login_url?: string
}

export type NodeSummary = {
  id: string
  display_name: string
  profile: string
  // Labels / tags from config (env, region, etc.) will be added later.
}

export type PlaneStatus = {
  name: string
  health: 'healthy' | 'degraded' | 'down'
  ready: boolean
  restart_count: number
}

export type AdminStatusView = {
  node_id: string
  display_name: string
  profile: string
  version: string
  planes: PlaneStatus[]
}

/**
 * Facet metrics summary as exposed by `/api/nodes/{id}/metrics/facets`.
 *
 * This mirrors `dto::metrics::FacetMetricsSummary` on the Rust side:
 * - `rps` is requests per second over the recent window.
 * - `error_rate` is a 0.0–1.0 fraction.
 * - `p95_latency_ms` / `p99_latency_ms` are latency percentiles in ms.
 */
export type FacetMetricsSummary = {
  facet: string
  rps: number
  error_rate: number
  p95_latency_ms: number
  p99_latency_ms: number
}
