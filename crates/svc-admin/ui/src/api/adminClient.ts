// crates/svc-admin/ui/src/api/adminClient.ts
//
// RO:WHAT — Thin HTTP client for the svc-admin backend API.
// RO:WHY  — Centralize fetch + error shaping + JSON parsing for SPA routes.
// RO:INVARIANTS —
//   - All methods are read-only unless explicitly named as an action.
//   - Errors include `status` when available so callers can classify 404/501/etc.
//   - No implicit retries here (UI controls fetch cadence).
//
// NOTE: If your project already has additional helpers/types here, keep them;
// this file is drop-in replacement for the common pattern used in svc-admin UI.

import type {
  UiConfigDto,
  MeResponse,
  NodeSummary,
  AdminStatusView,
  FacetMetricsSummary,
  NodeActionResponse,
  StorageSummaryDto,
  DatabaseEntryDto,
  DatabaseDetailDto,
  PlaygroundExampleDto,
  PlaygroundValidateManifestReq,
  PlaygroundValidateManifestResp,
} from '../types/admin-api'

type HttpError = Error & { status?: number; body?: string }

function makeHttpError(message: string, status?: number, body?: string): HttpError {
  const err = new Error(message) as HttpError
  if (typeof status === 'number') err.status = status
  if (typeof body === 'string') err.body = body
  return err
}

async function readTextSafe(r: Response): Promise<string> {
  try {
    return await r.text()
  } catch {
    return ''
  }
}

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const r = await fetch(path, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
  })

  if (!r.ok) {
    const body = await readTextSafe(r)
    throw makeHttpError(`${init?.method ?? 'GET'} ${path} → ${r.status}`, r.status, body)
  }

  // Some endpoints might return empty body; keep it strict and fail loudly.
  try {
    return (await r.json()) as T
  } catch (e: any) {
    const body = await readTextSafe(r)
    throw makeHttpError(`Failed to parse JSON from ${path}: ${String(e)}`, r.status, body)
  }
}

async function requestVoid(path: string, init?: RequestInit): Promise<void> {
  const r = await fetch(path, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
  })

  if (!r.ok) {
    const body = await readTextSafe(r)
    throw makeHttpError(`${init?.method ?? 'POST'} ${path} → ${r.status}`, r.status, body)
  }
}

export const adminClient = {
  // ---- UI/meta -----------------------------------------------------------

  async getUiConfig(): Promise<UiConfigDto> {
    return requestJson<UiConfigDto>('/api/ui-config')
  },

  async getMe(): Promise<MeResponse> {
    return requestJson<MeResponse>('/api/me')
  },

  // ---- Nodes -------------------------------------------------------------

  async getNodes(): Promise<NodeSummary[]> {
    return requestJson<NodeSummary[]>('/api/nodes')
  },

  async getNodeStatus(id: string): Promise<AdminStatusView> {
    return requestJson<AdminStatusView>(`/api/nodes/${encodeURIComponent(id)}/status`)
  },

  async getNodeFacetMetrics(id: string): Promise<FacetMetricsSummary[]> {
    return requestJson<FacetMetricsSummary[]>(
      `/api/nodes/${encodeURIComponent(id)}/metrics/facets`,
    )
  },

  // ---- Storage (read-only) ----------------------------------------------

  async getNodeStorageSummary(id: string): Promise<StorageSummaryDto> {
    return requestJson<StorageSummaryDto>(
      `/api/nodes/${encodeURIComponent(id)}/storage/summary`,
    )
  },

  async getNodeStorageDatabases(id: string): Promise<DatabaseEntryDto[]> {
    return requestJson<DatabaseEntryDto[]>(
      `/api/nodes/${encodeURIComponent(id)}/storage/databases`,
    )
  },

  async getNodeStorageDatabaseDetail(
    id: string,
    name: string,
  ): Promise<DatabaseDetailDto> {
    return requestJson<DatabaseDetailDto>(
      `/api/nodes/${encodeURIComponent(id)}/storage/databases/${encodeURIComponent(name)}`,
    )
  },

  // ---- Node actions (mutating; backend will gate) -------------------------

  async reloadNode(id: string): Promise<NodeActionResponse> {
    return requestJson<NodeActionResponse>(`/api/nodes/${encodeURIComponent(id)}/reload`, {
      method: 'POST',
      body: JSON.stringify({}),
    })
  },

  async shutdownNode(id: string): Promise<NodeActionResponse> {
    return requestJson<NodeActionResponse>(`/api/nodes/${encodeURIComponent(id)}/shutdown`, {
      method: 'POST',
      body: JSON.stringify({}),
    })
  },

  async debugCrashNode(id: string, service?: string | null): Promise<NodeActionResponse> {
    return requestJson<NodeActionResponse>(
      `/api/nodes/${encodeURIComponent(id)}/debug/crash`,
      {
        method: 'POST',
        body: JSON.stringify({ service: service ?? null }),
      },
    )
  },

  // ---- Playground (dev-only, read-only MVP) ------------------------------

  async getPlaygroundExamples(): Promise<PlaygroundExampleDto[]> {
    return requestJson<PlaygroundExampleDto[]>('/api/playground/examples')
  },

  async validatePlaygroundManifest(
    manifestToml: string,
  ): Promise<PlaygroundValidateManifestResp> {
    const req: PlaygroundValidateManifestReq = { manifestToml }
    return requestJson<PlaygroundValidateManifestResp>('/api/playground/manifest/validate', {
      method: 'POST',
      body: JSON.stringify(req),
    })
  },

  // ---- Misc --------------------------------------------------------------

  async ping(): Promise<void> {
    // Useful for smoke checks (optional).
    return requestVoid('/healthz', { method: 'GET' })
  },
}
