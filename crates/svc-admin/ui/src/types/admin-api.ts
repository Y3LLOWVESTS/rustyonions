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
//
// JSON example:
//
// {
//   "defaultTheme": "light",
//   "availableThemes": ["light", "dark"],
//   "defaultLanguage": "en-US",
//   "availableLanguages": ["en-US", "es-ES"],
//   "readOnly": true,
//   "dev": {
//     "enableAppPlayground": false
//   }
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
//
// JSON example:
//
// {
//   "subject": "dev-operator",
//   "displayName": "Dev Operator",
//   "roles": ["admin"],
//   "authMode": "none",
//   "loginUrl": null
// }

export type MeResponse = {
  subject: string
  displayName: string
  roles: string[]
  authMode: string
  loginUrl?: string
}

// ---- Node listing / status ----------------------------------------------
//
// Mirrors `dto::node::NodeSummary` on the Rust side.

export type NodeSummary = {
  id: string
  display_name: string
  profile: string
  // Labels / tags from config (env, region, etc.) will be added later.
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
  node_id: string
  display_name: string
  profile: string
  version: string
  planes: PlaneStatus[]
}

// ---- Facet metrics DTO ---------------------------------------------------
//
// Mirrors `dto::metrics::FacetMetricsSummary`.
//
// - `rps` is requests per second over the recent window.
// - `error_rate` is a 0.0–1.0 fraction.
// - `p95_latency_ms` / `p99_latency_ms` are latency percentiles in ms.

export type FacetMetricsSummary = {
  facet: string
  rps: number
  error_rate: number
  p95_latency_ms: number
  p99_latency_ms: number
}
