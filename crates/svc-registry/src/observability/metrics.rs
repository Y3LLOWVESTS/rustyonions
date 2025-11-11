//! RO:WHAT — Prometheus metrics for svc-registry.
//! RO:WHY  — Standardized counters/gauges/histograms with bounded label cardinality.
//! RO:INVARIANTS — label keys fixed; route labels reflect templates, not raw paths.
//!
//! Note: registration is made idempotent with a process-global OnceLock so tests
//! can construct multiple RegistryMetrics without "AlreadyReg" panics.

use prometheus::{
    opts, register_gauge, register_histogram_vec, register_int_counter, register_int_counter_vec,
    Encoder, Gauge, HistogramVec, IntCounter, IntCounterVec, TextEncoder,
};
use std::sync::OnceLock;

#[derive(Clone)]
pub struct RegistryMetrics {
    /// HTTP request totals by method/route/status.
    pub requests_total: IntCounterVec,
    /// HTTP request latency histogram by route.
    pub request_latency_seconds: HistogramVec,

    /// Write-path outcomes: {outcome="ok|error"}.
    pub registry_commits_total: IntCounterVec,
    /// Current head version (gauge).
    pub registry_head_version: Gauge,

    /// SSE lifecycle counters.
    pub registry_sse_clients_connected_total: IntCounter,
    pub registry_sse_clients_disconnected_total: IntCounter,
}

static METRICS_ONCE: OnceLock<RegistryMetrics> = OnceLock::new();

impl Default for RegistryMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryMetrics {
    /// Return a clone of the singleton metrics set; first caller registers.
    pub fn new() -> Self {
        METRICS_ONCE
            .get_or_init(|| Self::register_all())
            .clone()
    }

    /// Increment commit-success counter.
    pub fn inc_commit_ok(&self) {
        self.registry_commits_total
            .with_label_values(&["ok"])
            .inc();
    }

    /// Increment commit-error counter.
    pub fn inc_commit_err(&self) {
        self.registry_commits_total
            .with_label_values(&["error"])
            .inc();
    }

    /// Update the head-version gauge.
    pub fn set_head_version(&self, v: u64) {
        self.registry_head_version.set(v as f64);
    }

    /// Record an SSE **connect** event.
    pub fn sse_client_connected(&self) {
        self.registry_sse_clients_connected_total.inc();
    }

    /// Record an SSE **disconnect** (hooked post-beta when we have an on-close signal).
    #[allow(dead_code)]
    pub fn sse_client_disconnected(&self) {
        self.registry_sse_clients_disconnected_total.inc();
    }

    /// Gather current metrics into Prometheus text exposition format.
    pub fn gather_text(&self) -> String {
        let mf = prometheus::gather();
        let mut buf = Vec::with_capacity(64 * 1024);
        let encoder = TextEncoder::new();
        let _ = encoder.encode(&mf, &mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    }

    // ---- internals ----

    /// Construct and register all collectors on the default registry once.
    #[allow(clippy::expect_used)]
    fn register_all() -> Self {
        // HTTP
        let requests_total = register_int_counter_vec!(
            opts!(
                "requests_total",
                "HTTP request totals by method/route/status"
            ),
            &["method", "route", "status"]
        )
        .expect("register requests_total");

        let request_latency_seconds = register_histogram_vec!(
            "request_latency_seconds",
            "HTTP request latency (seconds) by route",
            &["route"]
        )
        .expect("register request_latency_seconds");

        // Registry
        let registry_commits_total = register_int_counter_vec!(
            opts!(
                "registry_commits_total",
                "Registry commit outcomes (ok|error)"
            ),
            &["outcome"]
        )
        .expect("register registry_commits_total");

        let registry_head_version =
            register_gauge!("registry_head_version", "Current registry head version")
                .expect("register registry_head_version");

        // SSE lifecycle
        let registry_sse_clients_connected_total = register_int_counter!(
            "registry_sse_clients_connected_total",
            "SSE clients connected (lifetime)"
        )
        .expect("register registry_sse_clients_connected_total");

        let registry_sse_clients_disconnected_total = register_int_counter!(
            "registry_sse_clients_disconnected_total",
            "SSE clients disconnected (lifetime)"
        )
        .expect("register registry_sse_clients_disconnected_total");

        Self {
            requests_total,
            request_latency_seconds,
            registry_commits_total,
            registry_head_version,
            registry_sse_clients_connected_total,
            registry_sse_clients_disconnected_total,
        }
    }
}
