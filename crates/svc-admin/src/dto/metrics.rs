// crates/svc-admin/src/dto/metrics.rs
//
// RO:WHAT — DTOs for facet metrics responses.
// RO:WHY  — Bridge between internal facet metrics store and JSON contracts
//           consumed by the svc-admin SPA.
// RO:INTERACTS — crate::metrics::facet::FacetMetrics, ui `FacetMetricsSummary`
// RO:INVARIANTS — Field names are snake_case → JSON keys as-is (no rename_all).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetMetricsSummary {
    /// Facet name, e.g. "admin.status" or "gateway.app".
    pub facet: String,
    /// Approximate requests per second over the recent window.
    pub rps: f64,
    /// Approximate error rate (0.0–1.0) over the recent window.
    pub error_rate: f64,
    /// Approximate p95 latency in milliseconds (currently stubbed).
    pub p95_latency_ms: f64,
    /// Approximate p99 latency in milliseconds (currently stubbed).
    pub p99_latency_ms: f64,
    /// Seconds since the last successful sample for this facet, as seen
    /// by the svc-admin sampler. `None` means “no samples yet”.
    pub last_sample_age_secs: Option<f64>,
}
