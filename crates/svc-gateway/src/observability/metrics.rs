//! Prometheus registry + golden metrics wiring.
//! Dashboard hints & names aligned with docs. :contentReference[oaicite:9]{index=9}

use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, HistogramVec,
    IntCounterVec, IntGauge,
};

/// Handles to all gateway metrics registered in the global Prometheus registry.
#[derive(Clone)]
pub struct MetricsHandles {
    /// Total HTTP requests, partitioned by `route`, `method`, and `status`.
    pub http_reqs: IntCounterVec,
    /// Request latency histogram (seconds), partitioned by `route` and `method`.
    pub http_lat: HistogramVec,
    /// Current number of in-flight requests across the gateway.
    pub inflight: IntGauge,
    /// Count of rejected requests by `reason` (e.g., `rate_limit`, `body_cap`, `timeout`).
    pub rejected: IntCounterVec,

    pub ready_inflight_current: IntGauge,
    pub ready_error_rate_pct: IntGauge,
    pub ready_queue_saturated: IntGauge,
}

/// Register metrics; returns handles.
///
/// # Errors
///
/// Returns an error if metrics of the same name are already registered.
pub fn register() -> anyhow::Result<MetricsHandles> {
    let http_reqs = register_int_counter_vec!(
        "http_requests_total",
        "HTTP requests",
        &["route", "method", "status"]
    )?;
    let http_lat = register_histogram_vec!(
        "request_latency_seconds",
        "Request latencies",
        &["route", "method"]
    )?;
    let inflight = register_int_gauge!("inflight_requests", "In-flight requests")?;
    let rejected = register_int_counter_vec!(
        "rejected_total",
        "Rejected by reason (e.g., rate_limit, body_cap, timeout)",
        &["reason"]
    )?;

    // Readiness gauges (carry-over names). :contentReference[oaicite:10]{index=10}
    let ready_inflight_current =
        register_int_gauge!("ready_inflight_current", "Current inflight across gateway")?;
    let ready_error_rate_pct =
        register_int_gauge!("ready_error_rate_pct", "Observed 429/503 % over window")?;
    let ready_queue_saturated =
        register_int_gauge!("ready_queue_saturated", "Queue saturated indicator")?;

    Ok(MetricsHandles {
        http_reqs,
        http_lat,
        inflight,
        rejected,
        ready_inflight_current,
        ready_error_rate_pct,
        ready_queue_saturated,
    })
}
