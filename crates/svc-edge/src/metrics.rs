//! Edge metrics (Prometheus).
//!
//! RO:WHAT
//! - Default-registry counters/histograms so we can tick from anywhere.
//! - Request accounting: requests_total{route,method,status}
//! - Rejects: edge_rejects_total{reason}
//! - Latency: edge_request_latency_seconds_bucket{route,method}
//! - Amnesia posture gauge.
//!
//! RO:USAGE
//! - In handlers: call `record_request(route, method, status, secs)`.
//! - In admission rejections: `inc_reject(reason)`.
//! - At startup: `seed_from_health(..., amnesia)`.

use once_cell::sync::Lazy;
use prometheus::{
    opts, register_histogram_vec, register_int_counter_vec, register_int_gauge, Encoder, HistogramVec,
    IntCounterVec, IntGauge, TextEncoder,
};
use std::time::Duration;

use crate::HealthState;

static HTTP_REQS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        opts!("edge_requests_total", "HTTP requests by route/method/status"),
        &["route", "method", "status"]
    )
    .unwrap()
});

static HTTP_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    // 1ms..10s range buckets (prometheus default-ish).
    register_histogram_vec!(
        "edge_request_latency_seconds",
        "Request latency seconds by route/method",
        &["route", "method"],
        vec![
            0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0
        ]
    )
    .unwrap()
});

static REJECTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        opts!("edge_rejects_total", "Admission rejects"),
        &["reason"]
    )
    .unwrap()
});

static AMNESIA: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(opts!("amnesia_mode", "Amnesia posture (1/0)")).unwrap()
});

/// Thin handle for metrics (future: attach more state here if needed).
#[derive(Clone, Debug)]
pub struct EdgeMetrics;

impl EdgeMetrics {
    /// Create and register all edge metrics in the default Prometheus registry.
    ///
    /// Touches each static to ensure registration has occurred before use.
    pub fn new() -> Self {
        let _ = &*HTTP_REQS;
        let _ = &*HTTP_LATENCY;
        let _ = &*REJECTS;
        let _ = &*AMNESIA;
        Self
    }
}

/// Seed gauges derived from initial health/config (currently just amnesia).
///
/// Call once at startup, after `EdgeMetrics::new()`.
pub fn seed_from_health(
    _health: std::sync::Arc<HealthState>,
    _metrics: &EdgeMetrics,
    amnesia: bool,
) {
    AMNESIA.set(if amnesia { 1 } else { 0 });
}

/// Record a single HTTP request outcome (increments counters and observes latency).
///
/// * `route`  — stable route label (e.g., `"/edge/assets/*path"`).
/// * `method` — HTTP method (e.g., `"GET"`, `"POST"`).
/// * `status` — final HTTP status code (e.g., `200`, `503`).
/// * `dur`    — wall time spent handling the request.
pub fn record_request(route: &str, method: &str, status: u16, dur: Duration) {
    HTTP_REQS
        .with_label_values(&[route, method, &status.to_string()])
        .inc();
    HTTP_LATENCY
        .with_label_values(&[route, method])
        .observe(dur.as_secs_f64());
}

/// Increment a reject counter for an admission-layer decision (e.g., `"timeout"`, `"busy"`).
pub fn inc_reject(reason: &str) {
    REJECTS.with_label_values(&[reason]).inc();
}

/// Render the Prometheus exposition text for `/metrics` (default registry).
///
/// This helper is used by the HTTP handler to return the encoded metrics body.
pub fn render() -> (axum::http::StatusCode, String) {
    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&metric_families, &mut buf).unwrap();
    (
        axum::http::StatusCode::OK,
        String::from_utf8(buf).unwrap_or_default(),
    )
}
