// crates/svc-admin/ui/src/api/adminClient.ts
//
// RO:WHAT — Thin HTTP client for the svc-admin backend API.
// RO:WHY  — Centralize fetch + error shaping + JSON parsing for SPA routes.
// RO:INVARIANTS —
//   - All methods are read-only unless explicitly named as an action.
//   - Errors include `status` when available so callers can classify 404/501/etc.
//   - No implicit retries here (UI controls fetch cadence).
//   - Dev-only request logging is passive and bounded (ring buffer).
//
// NOTE: This file is a drop-in replacement for the common pattern used in svc-admin UI.

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
  SystemSummaryDto,
} from '../types/admin-api'

export type HttpError = Error & { status?: number; body?: string }

function makeHttpError(message: string, status?: number, body?: string): HttpError {
  const err = new Error(message) as HttpError
  if (typeof status === 'number') err.status = status
  if (typeof body === 'string') err.body = body
  return err
}

export function isHttpError(err: unknown): err is HttpError {
  return !!err && typeof err === 'object' && ('status' in err || 'body' in err)
}

async function readTextSafe(r: Response): Promise<string> {
  try {
    return await r.text()
  } catch {
    return ''
  }
}

/**
 * Dev-only request log (bounded ring buffer).
 * WHY: During macronode integration, seeing the last N API calls, status codes,
 *      and timings prevents “it looks the same” / “is it broken?” loops.
 */
export type HttpLogEntry = {
  id: string
  at: string // ISO timestamp
  method: string
  path: string
  status?: number
  ok?: boolean
  duration_ms: number
  error?: string
  body_snippet?: string
}

type Listener = (entries: HttpLogEntry[]) => void

const MAX_LOG = 80
let logEntries: HttpLogEntry[] = []
const listeners = new Set<Listener>()

function pushLog(entry: HttpLogEntry) {
  logEntries = [entry, ...logEntries].slice(0, MAX_LOG)
  for (const fn of listeners) {
    try {
      fn(logEntries)
    } catch {
      // no-op
    }
  }
}

function nowIso(): string {
  return new Date().toISOString()
}

function randomId(): string {
  // Enough uniqueness for a dev ring buffer.
  return Math.random().toString(16).slice(2) + Math.random().toString(16).slice(2)
}

export const httpLog = {
  getEntries(): HttpLogEntry[] {
    return logEntries
  },
  subscribe(fn: Listener): () => void {
    listeners.add(fn)
    // push current immediately for convenience
    try {
      fn(logEntries)
    } catch {
      // no-op
    }
    return () => {
      listeners.delete(fn)
    }
  },
  clear(): void {
    logEntries = []
    for (const fn of listeners) {
      try {
        fn(logEntries)
      } catch {
        // no-op
      }
    }
  },
}

function buildHeaders(init?: RequestInit): HeadersInit {
  const headers: Record<string, string> = {
    Accept: 'application/json',
  }

  // Only set Content-Type if we actually have a body (or caller explicitly set it).
  // This keeps GETs "simple" and avoids surprises if we ever go cross-origin.
  const hasBody = typeof init?.body !== 'undefined' && init?.body !== null
  if (hasBody) headers['Content-Type'] = 'application/json'

  return {
    ...headers,
    ...(init?.headers ?? {}),
  }
}

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const method = init?.method ?? 'GET'
  const started = performance.now()
  const id = randomId()
  let logged = false

  try {
    const r = await fetch(path, {
      ...init,
      headers: buildHeaders(init),
    })

    const duration_ms = Math.max(0, performance.now() - started)

    if (!r.ok) {
      const body = await readTextSafe(r)
      pushLog({
        id,
        at: nowIso(),
        method,
        path,
        status: r.status,
        ok: false,
        duration_ms,
        error: `${method} ${path} → ${r.status}`,
        body_snippet: body.slice(0, 600),
      })
      logged = true
      throw makeHttpError(`${method} ${path} → ${r.status}`, r.status, body)
    }

    // Some endpoints might return empty body; keep it strict and fail loudly.
    try {
      const json = (await r.json()) as T
      pushLog({
        id,
        at: nowIso(),
        method,
        path,
        status: r.status,
        ok: true,
        duration_ms,
      })
      logged = true
      return json
    } catch (e: any) {
      const body = await readTextSafe(r)
      pushLog({
        id,
        at: nowIso(),
        method,
        path,
        status: r.status,
        ok: false,
        duration_ms,
        error: `Failed to parse JSON from ${path}: ${String(e)}`,
        body_snippet: body.slice(0, 600),
      })
      logged = true
      throw makeHttpError(`Failed to parse JSON from ${path}: ${String(e)}`, r.status, body)
    }
  } catch (e: any) {
    const duration_ms = Math.max(0, performance.now() - started)

    // Only log here if we didn't already log a non-2xx or JSON parse error above.
    // This path is intended for network/CORS/abort failures.
    if (!logged) {
      pushLog({
        id,
        at: nowIso(),
        method,
        path,
        ok: false,
        duration_ms,
        error: e?.message ? String(e.message) : 'Request failed',
      })
    }

    throw e
  }
}

/**
 * Request JSON, but treat "missing endpoint" as null.
 * Useful for capability rollout (e.g. /system/summary not present on older nodes).
 */
async function requestMaybeJson<T>(path: string, init?: RequestInit): Promise<T | null> {
  try {
    return await requestJson<T>(path, init)
  } catch (e: any) {
    if (isHttpError(e)) {
      const s = e.status
      if (s === 404 || s === 405 || s === 501) return null
    }
    throw e
  }
}

async function requestVoid(path: string, init?: RequestInit): Promise<void> {
  const method = init?.method ?? 'GET'
  const started = performance.now()
  const id = randomId()
  let logged = false

  try {
    const r = await fetch(path, {
      ...init,
      headers: buildHeaders(init),
    })

    const duration_ms = Math.max(0, performance.now() - started)

    if (!r.ok) {
      const body = await readTextSafe(r)
      pushLog({
        id,
        at: nowIso(),
        method,
        path,
        status: r.status,
        ok: false,
        duration_ms,
        error: `${method} ${path} → ${r.status}`,
        body_snippet: body.slice(0, 600),
      })
      logged = true
      throw makeHttpError(`${method} ${path} → ${r.status}`, r.status, body)
    }

    pushLog({
      id,
      at: nowIso(),
      method,
      path,
      status: r.status,
      ok: true,
      duration_ms,
    })
    logged = true
  } catch (e: any) {
    const duration_ms = Math.max(0, performance.now() - started)
    if (!logged) {
      pushLog({
        id,
        at: nowIso(),
        method,
        path,
        ok: false,
        duration_ms,
        error: e?.message ? String(e.message) : 'Request failed',
      })
    }
    throw e
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

  // ---- System (capability rollout; may be missing) -----------------------

  async getNodeSystemSummary(id: string): Promise<SystemSummaryDto | null> {
    return requestMaybeJson<SystemSummaryDto>(
      `/api/nodes/${encodeURIComponent(id)}/system/summary`,
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

  async getNodeStorageDatabaseDetail(id: string, name: string): Promise<DatabaseDetailDto> {
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
    return requestVoid('/healthz', { method: 'GET' })
  },
}
