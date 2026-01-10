// crates/svc-admin/ui/src/api/adminClient.ts
//
// RO:WHAT — Thin HTTP client for the svc-admin backend API.
// RO:WHY  — Centralize fetch + error shaping + JSON parsing for SPA routes.
// RO:INVARIANTS —
//   - All methods are read-only unless explicitly named as an action.
//   - Errors include `status` when available so callers can classify 404/501/etc.
//   - No implicit retries here (UI controls fetch cadence).
//   - Dev-only request logging is passive and bounded (ring buffer).
//   - Session cookies MUST be included for local auth.

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
  NetAccountingDto,
  BenchRunReq,
  BenchRunResp,
  BenchRunStatusDto,
  BenchRunResultDto,
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

export type HttpLogEntry = {
  id: string
  at: string
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
  return Math.random().toString(16).slice(2) + Math.random().toString(16).slice(2)
}

export const httpLog = {
  getEntries(): HttpLogEntry[] {
    return logEntries
  },
  subscribe(fn: Listener): () => void {
    listeners.add(fn)
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

  const hasBody = typeof init?.body !== 'undefined' && init?.body !== null
  if (hasBody) headers['Content-Type'] = 'application/json'

  return {
    ...headers,
    ...(init?.headers ?? {}),
  }
}

function parseJsonStrict<T>(text: string, path: string): T {
  if (!text || text.trim().length === 0) {
    throw new Error(`Empty JSON body from ${path}`)
  }
  return JSON.parse(text) as T
}

function withSession(init?: RequestInit): RequestInit {
  // Critical for local cookie-session auth (and also safe for ingress/passport modes).
  // Ensures cookies are sent + Set-Cookie responses are honored.
  return {
    ...(init ?? {}),
    credentials: init?.credentials ?? 'include',
  }
}

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const method = init?.method ?? 'GET'
  const started = performance.now()
  const id = randomId()
  let logged = false

  try {
    const r = await fetch(path, {
      ...withSession(init),
      headers: buildHeaders(init),
    })

    const duration_ms = Math.max(0, performance.now() - started)
    const bodyText = await readTextSafe(r)

    if (!r.ok) {
      pushLog({
        id,
        at: nowIso(),
        method,
        path,
        status: r.status,
        ok: false,
        duration_ms,
        error: `${method} ${path} → ${r.status}`,
        body_snippet: bodyText.slice(0, 600),
      })
      logged = true
      throw makeHttpError(`${method} ${path} → ${r.status}`, r.status, bodyText)
    }

    try {
      const json = parseJsonStrict<T>(bodyText, path)
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
      pushLog({
        id,
        at: nowIso(),
        method,
        path,
        status: r.status,
        ok: false,
        duration_ms,
        error: `Failed to parse JSON from ${path}: ${String(e)}`,
        body_snippet: bodyText.slice(0, 600),
      })
      logged = true
      throw makeHttpError(`Failed to parse JSON from ${path}: ${String(e)}`, r.status, bodyText)
    }
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

async function requestMaybeJson<T>(path: string, init?: RequestInit): Promise<T | null> {
  return requestMaybeJsonWithMissingStatuses<T>(path, init, [404, 405, 501])
}

async function requestMaybeJsonWithMissingStatuses<T>(
  path: string,
  init: RequestInit | undefined,
  missingStatuses: number[],
): Promise<T | null> {
  try {
    return await requestJson<T>(path, init)
  } catch (e: any) {
    if (isHttpError(e)) {
      const s = e.status
      if (typeof s === 'number' && missingStatuses.includes(s)) return null
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
      ...withSession(init),
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

type LoginRequest = {
  username: string
  password: string
}

export const adminClient = {
  // ---- Auth --------------------------------------------------------------

  async login(username: string, password: string): Promise<MeResponse> {
    const req: LoginRequest = { username, password }
    return requestJson<MeResponse>('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify(req),
    })
  },

  async logout(): Promise<void> {
    return requestVoid('/api/auth/logout', { method: 'POST', body: JSON.stringify({}) })
  },

  async authMe(): Promise<MeResponse> {
    return requestJson<MeResponse>('/api/auth/me', { method: 'GET' })
  },

  // ---- UI/meta -----------------------------------------------------------

  async getUiConfig(): Promise<UiConfigDto> {
    return requestJson<UiConfigDto>('/api/ui-config')
  },

  // Legacy/current API has /api/me in some modes; local auth uses /api/auth/me.
  // This method tries local-auth first and falls back.
  async getMe(): Promise<MeResponse> {
    const local = await requestMaybeJson<MeResponse>('/api/auth/me', { method: 'GET' })
    if (local) return local
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
    return requestJson<FacetMetricsSummary[]>(`/api/nodes/${encodeURIComponent(id)}/metrics/facets`)
  },

  // ---- System (capability rollout; may be missing) -----------------------

  async getNodeSystemSummary(id: string): Promise<SystemSummaryDto | null> {
    return requestMaybeJson<SystemSummaryDto>(`/api/nodes/${encodeURIComponent(id)}/system/summary`)
  },

  async getNodeSystemNetAccounting(id: string): Promise<NetAccountingDto | null> {
    return requestMaybeJson<NetAccountingDto>(`/api/nodes/${encodeURIComponent(id)}/system/net/accounting`)
  },

  // ---- Storage (read-only) ----------------------------------------------

  async getNodeStorageSummary(id: string): Promise<StorageSummaryDto> {
    return requestJson<StorageSummaryDto>(`/api/nodes/${encodeURIComponent(id)}/storage/summary`)
  },

  async getNodeStorageDatabases(id: string): Promise<DatabaseEntryDto[]> {
    return requestJson<DatabaseEntryDto[]>(`/api/nodes/${encodeURIComponent(id)}/storage/databases`)
  },

  async getNodeStorageDatabaseDetail(id: string, name: string): Promise<DatabaseDetailDto> {
    return requestJson<DatabaseDetailDto>(
      `/api/nodes/${encodeURIComponent(id)}/storage/databases/${encodeURIComponent(name)}`,
    )
  },

  // ---- Benchmarks (capability rollout; may be missing) -------------------

  async runNodeBench(id: string, req: BenchRunReq): Promise<BenchRunResp | null> {
    return requestMaybeJson<BenchRunResp>(`/api/nodes/${encodeURIComponent(id)}/bench/run`, {
      method: 'POST',
      body: JSON.stringify(req),
    })
  },

  async getNodeBenchRunStatus(id: string, runId: string): Promise<BenchRunStatusDto | null> {
    return requestMaybeJsonWithMissingStatuses<BenchRunStatusDto>(
      `/api/nodes/${encodeURIComponent(id)}/bench/runs/${encodeURIComponent(runId)}`,
      { method: 'GET' },
      [405, 501],
    )
  },

  async getNodeBenchRunResult(id: string, runId: string): Promise<BenchRunResultDto | null> {
    return requestMaybeJsonWithMissingStatuses<BenchRunResultDto>(
      `/api/nodes/${encodeURIComponent(id)}/bench/runs/${encodeURIComponent(runId)}/result`,
      { method: 'GET' },
      [405, 501],
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
    return requestJson<NodeActionResponse>(`/api/nodes/${encodeURIComponent(id)}/debug/crash`, {
      method: 'POST',
      body: JSON.stringify({ service: service ?? null }),
    })
  },

  // ---- Playground (dev-only, read-only MVP) ------------------------------

  async getPlaygroundExamples(): Promise<PlaygroundExampleDto[]> {
    return requestJson<PlaygroundExampleDto[]>('/api/playground/examples')
  },

  async validatePlaygroundManifest(manifestToml: string): Promise<PlaygroundValidateManifestResp> {
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
