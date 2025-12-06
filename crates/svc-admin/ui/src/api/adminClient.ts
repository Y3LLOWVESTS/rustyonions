// crates/svc-admin/ui/src/api/adminClient.ts
//
// RO:WHAT — Thin fetch wrapper for svc-admin backend APIs.
// RO:WHY  — Keep all HTTP paths and DTO wiring in one place so routes
//           and components stay dumb and testable.
// RO:INTERACTS — `types/admin-api.ts`, Rust `router.rs` JSON contracts.
// RO:INVARIANTS — All paths are relative to the same origin; errors
//                 surface as thrown Error instances.

import type {
  UiConfigDto,
  MeResponse,
  NodeSummary,
  AdminStatusView,
  FacetMetricsSummary,
  NodeActionResponse
} from '../types/admin-api'

const base = ''

async function getJson<T>(path: string): Promise<T> {
  const rsp = await fetch(base + path)
  if (!rsp.ok) {
    throw new Error(`Request failed: ${rsp.status}`)
  }
  return rsp.json() as Promise<T>
}

async function postJson<T>(path: string): Promise<T> {
  const rsp = await fetch(base + path, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    }
  })

  if (!rsp.ok) {
    // For now we just surface the HTTP status; we can refine this once
    // the backend adds richer error bodies.
    throw new Error(`Request failed: ${rsp.status}`)
  }

  return rsp.json() as Promise<T>
}

export const adminClient = {
  getUiConfig: () => getJson<UiConfigDto>('/api/ui-config'),

  getMe: () => getJson<MeResponse>('/api/me'),

  getNodes: () => getJson<NodeSummary[]>('/api/nodes'),

  getNodeStatus: (id: string) =>
    getJson<AdminStatusView>(`/api/nodes/${encodeURIComponent(id)}/status`),

  getNodeFacetMetrics: (id: string) =>
    getJson<FacetMetricsSummary[]>(
      `/api/nodes/${encodeURIComponent(id)}/metrics/facets`
    ),

  reloadNode: (id: string) =>
    postJson<NodeActionResponse>(
      `/api/nodes/${encodeURIComponent(id)}/reload`
    ),

  shutdownNode: (id: string) =>
    postJson<NodeActionResponse>(
      `/api/nodes/${encodeURIComponent(id)}/shutdown`
    )
}
