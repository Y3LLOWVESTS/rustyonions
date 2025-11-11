//! Edge metrics wrapper over the shared Prometheus registry.

use once_cell::sync::Lazy;
use prometheus::{
    register_gauge, register_histogram, register_int_counter_vec, Encoder, Gauge, Histogram,
    IntCounterVec, TextEncoder,
};
use ron_kernel::HealthState;
use std::sync::Arc;

static REQ_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "edge_request_latency_seconds",
        "Latency of handled edge requests (admin plane for now)"
    )
    .unwrap()
});

static REJECTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!("edge_rejects_total", "Rejects by reason", &["reason"]).unwrap()
});

static AMNESIA: Lazy<Gauge> =
    Lazy::new(|| register_gauge!("amnesia_mode", "Amnesia posture (1/0)").unwrap());

/// Handle to edge metrics (cheap clone).
#[derive(Clone, Default)]
pub struct EdgeMetrics;

impl EdgeMetrics {
    /// Create a new metrics handle.
    pub fn new() -> Self {
        Self
    }

    /// Observe a request latency in seconds.
    pub fn observe_req_latency(&self, secs: f64) {
        REQ_LATENCY.observe(secs);
    }

    /// Increment the rejects counter for a given reason label.
    pub fn inc_reject(&self, reason: &str) {
        REJECTS.with_label_values(&[reason]).inc();
    }

    /// Set the amnesia posture gauge to 1 (on) or 0 (off).
    pub fn set_amnesia(&self, on: bool) {
        AMNESIA.set(if on { 1.0 } else { 0.0 });
    }

    /// Render Prometheus exposition format into bytes using the global registry.
    pub fn gather() -> Vec<u8> {
        let mut buf = Vec::with_capacity(16 * 1024);
        let mf = prometheus::gather();
        let _ = TextEncoder::new().encode(&mf, &mut buf);
        buf
    }
}

/// Seed metric gauges from the health snapshot if needed.
///
/// This is a convenience for startup synchronization patterns.
pub fn seed_from_health(_health: Arc<HealthState>, metrics: &EdgeMetrics, amnesia: bool) {
    metrics.set_amnesia(amnesia);
}
