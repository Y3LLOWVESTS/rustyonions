// crates/svc-admin/ui/src/types/admin-api.ts
//
// WHAT: Shared TypeScript DTO definitions for the svc-admin SPA.
// WHY:  Keep the UI strictly aligned with the Rust-side DTOs in
//       `crates/svc-admin/src/dto/*` and the documented JSON contracts.
//
// NOTE:
//   - UiConfigDto mirrors `dto::ui::UiConfigDto` (camelCase via serde).
//   - MeResponse mirrors `dto::me::MeResponse` (camelCase via serde).
//   - Node / metrics types mirror `dto::node` and `dto::metrics`.
//
// COMPAT POLICY (important):
//   Rust DTOs sometimes use snake_case (no rename_all) and sometimes camelCase (rename_all="camelCase").
//   To prevent UI breakage while endpoints are rolling out, we allow harmless alias fields as OPTIONAL
//   (e.g. restart_count vs restartCount vs restarts). Prefer the canonical field when present.

export type UiDevConfig = {
  enableAppPlayground: boolean
}

export type UiConfigDto = {
  defaultTheme: string
  availableThemes: string[]
  defaultLanguage: string
  availableLanguages: string[]
  readOnly: boolean
  dev: UiDevConfig
}

export type MeResponse = {
  subject: string
  displayName: string
  roles: string[]
  authMode: string
  // Optional and may be null when no interactive login is available.
  loginUrl?: string | null
}

// ---- Node listing / status ----------------------------------------------
//
// Mirrors `dto::node::NodeSummary` on the Rust side.
// NOTE: JSON field names are snake_case here (no rename_all on the Rust struct).

export type NodeSummary = {
  id: string
  display_name: string
  profile: string | null

  // Optional compat aliases (safe to ignore if absent).
  displayName?: string
}

// Mirrors `dto::node::PlaneStatus`.
export type PlaneStatus = {
  name: string
  health: 'healthy' | 'degraded' | 'down'
  ready: boolean

  // Canonical (snake_case) used by current Rust DTO.
  restart_count: number

  // Compat aliases (optional) — older/other emitters may use these.
  restarts?: number
  restartCount?: number

  // Optional timing hints some emitters may include (safe for UI).
  age_s?: number
  ageSecs?: number
  age_ms?: number
  ageMs?: number
  sample_age_secs?: number
  last_sample_age_secs?: number
}

// Mirrors `dto::node::AdminStatusView`.
export type AdminStatusView = {
  id: string
  display_name: string
  profile: string | null
  version: string | null

  // Best-effort uptime (seconds). Optional for older nodes.
  uptime_seconds?: number | null

  planes: PlaneStatus[]

  // Optional in older nodes; UI treats missing as "dev allow".
  capabilities?: string[] | null

  // Compat aliases (optional).
  displayName?: string
  uptimeSeconds?: number | null
}

// ---- Facet metrics DTO ---------------------------------------------------
//
// Mirrors `dto::metrics::FacetMetricsSummary`.
// Historically this has been snake_case on the wire.

export type FacetMetricsSummary = {
  facet: string
  rps: number
  error_rate: number
  p95_latency_ms: number
  p99_latency_ms: number
  last_sample_age_secs: number | null

  // Compat aliases (optional) — safe.
  errorRate?: number
  p95LatencyMs?: number
  p99LatencyMs?: number
  lastSampleAgeSecs?: number | null
}

// ---- Node actions DTO ----------------------------------------------------
//
// Mirrors `dto::node::NodeActionResponse`.

export type NodeActionResponse = {
  node_id: string
  action: string
  accepted: boolean
  message?: string | null

  // Compat alias (optional)
  nodeId?: string
}

// ---- Storage / Databases (read-only) -------------------------------------

export type NodeCapability = 'storage.readonly.v1' | string

export type StorageSummaryDto = {
  fsType: string
  mount: string

  totalBytes: number
  usedBytes: number
  freeBytes: number

  ioReadBps: number | null
  ioWriteBps: number | null
}

export type DatabaseHealth = 'ok' | 'degraded' | 'error'

export type DatabaseEntryDto = {
  name: string
  engine: string
  sizeBytes: number

  mode: string
  owner: string

  health: DatabaseHealth

  notes?: string | null
  worldReadable?: boolean
  worldWritable?: boolean
}

export type DatabaseDetailDto = {
  name: string
  engine: string
  sizeBytes: number

  mode: string
  owner: string
  health: DatabaseHealth

  pathAlias: string

  fileCount: number
  lastCompaction: string | null

  approxKeys: number | null
  warnings: string[]
}

// ---- App Playground (dev-only, read-only MVP) ----------------------------

export type PlaygroundExampleDto = {
  id: string
  title: string
  description: string
  manifestToml: string
}

export type PlaygroundValidateManifestReq = {
  manifestToml: string
}

export type PlaygroundValidateManifestResp = {
  ok: boolean
  errors: string[]
  warnings: string[]
  parsed?: unknown | null
}

// ---- System summary ------------------------------------------------------

export type SystemSummaryDto = {
  updatedAt: string
  cpuPercent?: number | null

  // NEW: optional basic CPU facts (safe for rollout)
  cpuCores?: number | null
  cpuThreads?: number | null

  ramTotalBytes: number
  ramUsedBytes: number

  netRxBps?: number | null
  netTxBps?: number | null
}

// ---- Benchmarks (node-executed; bounded) ---------------------------------

export type BenchRunReq = {
  suite: string
  durationSecs: number
  concurrency: number
  payloadSize: number
  seed: number
  limits?: {
    maxDurationSecs?: number
    maxConcurrency?: number
    maxPayloadSize?: number
  }
}

export type BenchRunResp = {
  runId: string
}

export type BenchRunState = 'queued' | 'running' | 'done' | 'failed'

export type BenchRunStatusDto = {
  runId: string
  status: BenchRunState
  progress: number
  phase: string
  startedAt?: string | null
  endedAt?: string | null
  error?: string | null
  partial?: unknown | null
}

export type BenchScenarioResultDto = {
  name: string
  ok: boolean
  p50LatencyMs?: number | null
  p95LatencyMs?: number | null
  p99LatencyMs?: number | null
  throughputOpsPerSec?: number | null
  throughputBytesPerSec?: number | null
  errorRate?: number | null
  notes?: string[] | null
}

export type BenchRunResultDto = {
  runId: string
  suite: string
  nodeId?: string | null
  startedAt: string
  endedAt: string
  scenarios: BenchScenarioResultDto[]
  env?: {
    service?: string | null
    version?: string | null
    gitSha?: string | null
    profile?: string | null
    host?: string | null
    os?: string | null
    arch?: string | null
  } | null
}
