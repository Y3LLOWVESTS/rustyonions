// crates/svc-admin/ui/src/api/adminClient.ts
//
// RO:WHAT - Thin fetch wrapper for svc-admin backend APIs.
// RO:WHY  - Keep all HTTP paths and DTO wiring in one place so routes
//           and components stay simple and testable.
// RO:INTERACTS - types/admin-api.ts, Rust router.rs JSON contracts.

import type {
  UiConfigDto,
  MeResponse,
  NodeSummary,
  AdminStatusView,
  FacetMetricsSummary,
  NodeActionResponse,

  // Storage (Slice 3)
  StorageSummaryDto,
  DatabaseEntryDto,
  DatabaseDetailDto,
} from '../types/admin-api'

// Base URL strategy:
//
// - In dev, we want to use *relative* URLs ("/api/...") so Vite can proxy
//   to the backend and we avoid CORS completely.
// - In production, the SPA is normally served by svc-admin itself, so
//   relative URLs still work (same origin).
// - If you *really* want to point at a remote svc-admin, set
//   VITE_SVC_ADMIN_BASE_URL and we'll prefix with that.
const RAW_BASE_URL: string =
  (import.meta as any).env?.VITE_SVC_ADMIN_BASE_URL ??
  (import.meta as any).env?.VITE_API_BASE_URL ??
  ''

function buildUrl(path: string): string {
  if (!RAW_BASE_URL) return path
  const base = RAW_BASE_URL.replace(/\/+$/, '')
  return `${base}${path}`
}

type FetchError = Error & { status?: number; statusText?: string }

async function handleResponse<T>(res: Response): Promise<T> {
  if (!res.ok) {
    let bodyText = ''
    try {
      bodyText = await res.text()
    } catch {
      // ignore secondary errors
    }

    const msg =
      bodyText && bodyText.length < 1024
        ? `Request failed: ${res.status} ${res.statusText} - ${bodyText}`
        : `Request failed: ${res.status} ${res.statusText}`

    const err = new Error(msg) as FetchError
    err.status = res.status
    err.statusText = res.statusText
    throw err
  }

  const text = await res.text()
  if (!text) return undefined as unknown as T
  return JSON.parse(text) as T
}

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(buildUrl(path), init)
  return handleResponse<T>(res)
}

export const adminClient = {
  // --- Core config / identity ---------------------------------------------

  async getUiConfig(): Promise<UiConfigDto> {
    return fetchJson<UiConfigDto>('/api/ui-config')
  },

  async getMe(): Promise<MeResponse> {
    return fetchJson<MeResponse>('/api/me')
  },

  // --- Nodes listing / status ---------------------------------------------

  async getNodes(): Promise<NodeSummary[]> {
    return fetchJson<NodeSummary[]>('/api/nodes')
  },

  async getNodeStatus(id: string): Promise<AdminStatusView> {
    return fetchJson<AdminStatusView>(
      `/api/nodes/${encodeURIComponent(id)}/status`,
    )
  },

  async getNodeFacetMetrics(id: string): Promise<FacetMetricsSummary[]> {
    return fetchJson<FacetMetricsSummary[]>(
      `/api/nodes/${encodeURIComponent(id)}/metrics/facets`,
    )
  },

  // --- Node storage (read-only) -------------------------------------------
  //
  // These endpoints may return 404/501 until node/admin-plane support exists.
  // Callers should be prepared to fall back to mock data.

  async getNodeStorageSummary(id: string): Promise<StorageSummaryDto> {
    return fetchJson<StorageSummaryDto>(
      `/api/nodes/${encodeURIComponent(id)}/storage/summary`,
    )
  },

  async getNodeDatabases(id: string): Promise<DatabaseEntryDto[]> {
    return fetchJson<DatabaseEntryDto[]>(
      `/api/nodes/${encodeURIComponent(id)}/storage/databases`,
    )
  },

  async getNodeDatabaseDetail(
    id: string,
    name: string,
  ): Promise<DatabaseDetailDto> {
    return fetchJson<DatabaseDetailDto>(
      `/api/nodes/${encodeURIComponent(id)}/storage/databases/${encodeURIComponent(
        name,
      )}`,
    )
  },

  // --- Node control actions -----------------------------------------------

  async reloadNode(id: string): Promise<NodeActionResponse> {
    return fetchJson<NodeActionResponse>(
      `/api/nodes/${encodeURIComponent(id)}/reload`,
      { method: 'POST' },
    )
  },

  async shutdownNode(id: string): Promise<NodeActionResponse> {
    return fetchJson<NodeActionResponse>(
      `/api/nodes/${encodeURIComponent(id)}/shutdown`,
      { method: 'POST' },
    )
  },

  // --- Dev-only: synthetic crash proxy ------------------------------------

  async debugCrashNode(
    id: string,
    service?: string,
  ): Promise<NodeActionResponse> {
    const payload: { service?: string } = {}
    if (service) payload.service = service

    return fetchJson<NodeActionResponse>(
      `/api/nodes/${encodeURIComponent(id)}/debug/crash`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      },
    )
  },
}
