// crates/svc-admin/src/dto/bench.rs
//
// RO:WHAT — Benchmark DTOs for the node admin-plane contract.
// RO:WHY  — svc-admin proxies benchmarks and renders results in the dashboard.
// RO:INVARIANTS —
//   - camelCase for SPA friendliness.
//   - DTOs are forward-compatible (extra / missing fields tolerated where safe).
//   - Matches macronode v1 “admin_plane” suite, but leaves room for future suites.

#![forbid(unsafe_code)]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchRunReqDto {
    /// Suite name. v1 supports: "admin_plane".
    pub suite: String,
    /// Total runtime (seconds). Node enforces caps.
    pub duration_secs: u64,
    /// Worker count. Node enforces caps.
    pub concurrency: u32,
    /// Placeholder for future suites (e.g., storage PUT/GET payload size).
    #[serde(default)]
    pub payload_size: u64,
    /// Determinism seed (optional). If 0, node may pick its own seed.
    #[serde(default)]
    pub seed: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchRunRespDto {
    pub run_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchRunStatusDto {
    pub run_id: String,
    /// queued | running | done | failed
    pub status: String,
    /// 0.0 .. 1.0
    pub progress: f32,
    /// Free-form phase label (warming, running, done, failed, etc.).
    pub phase: String,
    /// When the run actually started (may be None while queued).
    pub started_at: Option<String>,
    /// Last status update time (ISO-ish string).
    pub updated_at: String,
    /// Optional error string if status == "failed".
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchEndpointResultDto {
    pub name: String,
    pub method: String,
    pub path: String,

    pub requests: u64,
    pub errors: u64,
    pub rps: f64,

    // NOTE:
    // Field identifiers are snake_case, but `rename_all = "camelCase"`
    // makes the JSON keys `p50Ms`, `p95Ms`, `p99Ms`. This matches macronode.
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

/// Optional “scenario-style” results for future suites.
/// Not used by the current macronode `admin_plane` harness, but we allow it
/// so newer nodes can return richer aggregated results without breaking
/// older svc-admin builds.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchScenarioResultDto {
    pub name: String,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub p50_latency_ms: Option<f64>,
    #[serde(default)]
    pub p95_latency_ms: Option<f64>,
    #[serde(default)]
    pub p99_latency_ms: Option<f64>,
    #[serde(default)]
    pub throughput_ops_per_sec: Option<f64>,
    #[serde(default)]
    pub throughput_bytes_per_sec: Option<f64>,
    #[serde(default)]
    pub error_rate: Option<f64>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchRunResultDto {
    pub run_id: String,
    pub suite: String,
    pub started_at: String,
    pub ended_at: String,

    /// v1 macronode shape — per-endpoint metrics.
    /// `#[serde(default)]` keeps us tolerant if future nodes omit this and
    /// only send scenarios.
    #[serde(default)]
    pub results: Vec<BenchEndpointResultDto>,

    /// Free-form notes (e.g., explanation of the suite, caveats).
    #[serde(default)]
    pub notes: Vec<String>,

    /// Optional future/god-tier shape: aggregated scenario results.
    /// Empty for current macronode `admin_plane` runs.
    #[serde(default)]
    pub scenarios: Vec<BenchScenarioResultDto>,
}
