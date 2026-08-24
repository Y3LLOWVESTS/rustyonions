//! Prometheus registry + golden metrics wiring.
//! Dashboard hints & names aligned with docs. :contentReference[oaicite:9]{index=9}

use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, HistogramOpts,
    HistogramVec, IntCounterVec, IntGauge, Opts,
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

/// Construct gateway metric handles without registering them in the
/// process-global Prometheus registry.
///
/// RO:WHY
/// `CrabNode` composes `svc-gateway` and `Omnigate` in one process. Both
/// historical services own some generic Prometheus names, so embedded
/// composition must not re-register the gateway's legacy state handles
/// globally.
///
/// The canonical standalone `svc-gateway` binary continues to call
/// [`register`], preserving its existing metrics contract.
///
/// # Errors
///
/// Returns an error if a local collector cannot be constructed.
pub fn unregistered() -> anyhow::Result<MetricsHandles> {
    let http_reqs = IntCounterVec::new(
        Opts::new("http_requests_total", "HTTP requests"),
        &["route", "method", "status"],
    )?;

    let http_lat = HistogramVec::new(
        HistogramOpts::new("request_latency_seconds", "Request latencies"),
        &["route", "method"],
    )?;

    let inflight = IntGauge::new("inflight_requests", "In-flight requests")?;

    let rejected = IntCounterVec::new(
        Opts::new(
            "rejected_total",
            "Rejected by reason (e.g., rate_limit, body_cap, timeout)",
        ),
        &["reason"],
    )?;

    let ready_inflight_current =
        IntGauge::new("ready_inflight_current", "Current inflight across gateway")?;

    let ready_error_rate_pct =
        IntGauge::new("ready_error_rate_pct", "Observed 429/503 % over window")?;

    let ready_queue_saturated =
        IntGauge::new("ready_queue_saturated", "Queue saturated indicator")?;

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

/// Register the gateway metric handles in the process-global Prometheus
/// registry.
///
/// # Errors
///
/// Returns an error if any metric cannot be registered, including when an
/// incompatible metric with the same name is already present.
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

#[cfg(test)]
mod tests {
    use super::unregistered;

    #[test]
    fn unregistered_handles_can_coexist_without_global_registration() {
        let first = unregistered().expect("first local gateway metric set");

        let second = unregistered().expect("second local gateway metric set");

        first
            .http_reqs
            .with_label_values(&["healthz", "GET", "200"])
            .inc();

        assert_eq!(
            first
                .http_reqs
                .with_label_values(&["healthz", "GET", "200",])
                .get(),
            1,
        );

        assert_eq!(
            second
                .http_reqs
                .with_label_values(&["healthz", "GET", "200",])
                .get(),
            0,
            "local metric handles must not alias through a global collector",
        );
    }
}
