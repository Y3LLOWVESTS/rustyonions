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

// ---- UI config DTO -------------------------------------------------------
//
// Rust (`dto::ui::UiConfigDto`):
//
// #[serde(rename_all = "camelCase")]
// pub struct UiConfigDto {
//     pub default_theme: String,
//     pub available_themes: Vec<String>,
//     pub default_language: String,
//     pub available_languages: Vec<String>,
//     pub read_only: bool,
//     pub dev: UiDevDto,
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct UiDevDto {
//     pub enable_app_playground: bool,
// }

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

// ---- /api/me DTO ---------------------------------------------------------
//
// Rust (`dto::me::MeResponse`):
//
// #[serde(rename_all = "camelCase")]
// pub struct MeResponse {
//     pub subject: String,
//     pub display_name: String,
//     pub roles: Vec<String>,
//     pub auth_mode: String,
//     pub login_url: Option<String>,
// }

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
  // Optional profile hint, e.g. "macronode" / "micronode".
  profile: string | null
  // Labels / tags from config (env, region, etc.) may be added later.
}

// Mirrors `dto::node::PlaneStatus`.

export type PlaneStatus = {
  name: string
  health: 'healthy' | 'degraded' | 'down'
  ready: boolean
  restart_count: number
}

// Mirrors `dto::node::AdminStatusView`.

export type AdminStatusView = {
  id: string
  display_name: string
  profile: string | null
  version: string | null

  // Best-effort uptime (seconds). Optional for older nodes.
  // Rust: AdminStatusView.uptime_seconds: Option<u64>
  uptime_seconds?: number | null

  planes: PlaneStatus[]

  // Optional in older nodes; UI treats missing as "dev allow".
  capabilities?: string[] | null
}

// ---- Facet metrics DTO ---------------------------------------------------
//
// Mirrors `dto::metrics::FacetMetricsSummary`.

export type FacetMetricsSummary = {
  facet: string
  rps: number
  error_rate: number
  p95_latency_ms: number
  p99_latency_ms: number
  last_sample_age_secs: number | null
}

// ---- Node actions DTO ----------------------------------------------------
//
// Mirrors `dto::node::NodeActionResponse`.

export type NodeActionResponse = {
  node_id: string
  action: string
  accepted: boolean
  message?: string | null
}

// ---- Storage / Databases (read-only) -------------------------------------
//
// NOTE (design):
// - These DTOs are intentionally “curated facts” and NOT a remote file browser.
// - Paths should be reported as aliases (e.g. "data/db") rather than raw paths.
// - Permissions should be represented safely (mode + derived flags), not ACLs.
// - svc-admin will capability-gate these screens via "storage.readonly.v1".
//
// These DTOs are consumed by svc-admin SPA via:
//   - GET /api/nodes/:id/storage/summary
//   - GET /api/nodes/:id/storage/databases
//   - GET /api/nodes/:id/storage/databases/:name

export type NodeCapability = 'storage.readonly.v1' | string

export type StorageSummaryDto = {
  // Filesystem type (e.g. "ext4", "apfs", "ntfs") if known.
  fsType: string
  // Mount name or alias ("/", "data", etc.).
  mount: string

  totalBytes: number
  usedBytes: number
  freeBytes: number

  // Optional I/O rates; may be null when not supported/available.
  ioReadBps: number | null
  ioWriteBps: number | null
}

export type DatabaseHealth = 'ok' | 'degraded' | 'error'

export type DatabaseEntryDto = {
  name: string
  engine: string
  sizeBytes: number

  // Permissions are safe/collapsed facts.
  // mode is a string like "0750".
  mode: string
  owner: string

  health: DatabaseHealth

  // Optional operator-facing notes; may be absent.
  notes?: string | null

  // Optional derived warning flags (additive, safe).
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

  // Alias only; do not leak raw absolute paths.
  pathAlias: string

  fileCount: number
  lastCompaction: string | null

  // Optional, only if cheap.
  approxKeys: number | null

  // Safe warning strings for UI banners.
  warnings: string[]
}

// ---- App Playground (dev-only, read-only MVP) ----------------------------
//
// Rust router (svc-admin backend):
//   GET  /api/playground/examples
//   POST /api/playground/manifest/validate
//
// These are *svc-admin-local* helpers (not node execution).
// They are hidden unless UiConfigDto.dev.enableAppPlayground is true.

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
  // Parsed TOML rendered as JSON value for inspection (may be null on parse error).
  parsed?: unknown | null
}

/// system summary
export type SystemSummaryDto = {
  updatedAt: string
  cpuPercent?: number | null
  ramTotalBytes: number
  ramUsedBytes: number
  netRxBps?: number | null
  netTxBps?: number | null
}
