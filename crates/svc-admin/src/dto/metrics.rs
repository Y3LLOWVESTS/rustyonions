use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetMetricsSummary {
    pub facet: String,
    pub rps: f64,
    pub error_rate: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
}
