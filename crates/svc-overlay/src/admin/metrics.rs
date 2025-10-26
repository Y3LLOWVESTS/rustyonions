//! RO:WHAT
//!   Prometheus exporter for /metrics backed by the default registry.
//!
//! RO:WHY
//!   Self-initialize overlay metrics and build_info so the endpoint is never empty.
//!
//! RO:INTERACTS
//!   - Other modules bump overlay metrics via `overlay_metrics::*`.
//!
//! RO:INVARIANTS
//!   - Single global registry; encode errors return 500; no panics.

use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
};
use once_cell::sync::Lazy;
use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
    TextEncoder,
};
use tracing::warn;

static GLOBAL_REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

fn ensure_registered() {
    let _ = &*OVERLAY_SESSIONS_ACTIVE;
    let _ = &*OVERLAY_ACCEPT_LATENCY_SECONDS;
    let _ = &*BUILD_INFO;
    let _ = &*OVERLAY_HANDSHAKE_FAIL_TOTAL;
    let _ = &*OVERLAY_PEER_TX_DROPPED_TOTAL;
    let _ = &*OVERLAY_PEER_TX_DEPTH;
    let _ = &*OVERLAY_CONN_LIFETIME_SECONDS;
}

static OVERLAY_SESSIONS_ACTIVE: Lazy<IntGauge> = Lazy::new(|| {
    let g = IntGauge::with_opts(Opts::new(
        "overlay_sessions_active",
        "Current number of active overlay sessions",
    ))
    .expect("gauge");
    GLOBAL_REGISTRY
        .register(Box::new(g.clone()))
        .expect("register overlay_sessions_active");
    g
});

static OVERLAY_ACCEPT_LATENCY_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    let h = Histogram::with_opts(
        HistogramOpts::new(
            "overlay_accept_latency_seconds",
            "Time from accept to handshake start",
        )
        .buckets(vec![
            0.00005, 0.0001, 0.0002, 0.0005, 0.001, 0.002, 0.005, 0.01,
        ]),
    )
    .expect("histogram");
    GLOBAL_REGISTRY
        .register(Box::new(h.clone()))
        .expect("register overlay_accept_latency_seconds");
    h
});

static OVERLAY_CONN_LIFETIME_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    let h = Histogram::with_opts(HistogramOpts::new(
        "overlay_conn_lifetime_seconds",
        "Lifetime of a connection (from handshake ok to close)",
    ))
    .expect("histogram");
    GLOBAL_REGISTRY
        .register(Box::new(h.clone()))
        .expect("register overlay_conn_lifetime_seconds");
    h
});

static OVERLAY_HANDSHAKE_FAIL_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let c = IntCounterVec::new(
        Opts::new(
            "overlay_handshake_fail_total",
            "Handshake failures by reason",
        ),
        &["reason"],
    )
    .expect("counter");
    GLOBAL_REGISTRY
        .register(Box::new(c.clone()))
        .expect("register overlay_handshake_fail_total");
    c
});

static OVERLAY_PEER_TX_DROPPED_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    let g = IntGauge::with_opts(Opts::new(
        "overlay_peer_dropped_total",
        "Total frames dropped due to full per-peer TX queue (process lifetime)",
    ))
    .expect("gauge");
    GLOBAL_REGISTRY
        .register(Box::new(g.clone()))
        .expect("register overlay_peer_dropped_total");
    g
});

static OVERLAY_PEER_TX_DEPTH: Lazy<IntGauge> = Lazy::new(|| {
    let g = IntGauge::with_opts(Opts::new(
        "overlay_peer_queue_depth",
        "Current per-peer TX queue depth (most recent writer task)",
    ))
    .expect("gauge");
    GLOBAL_REGISTRY
        .register(Box::new(g.clone()))
        .expect("register overlay_peer_queue_depth");
    g
});

static BUILD_INFO: Lazy<IntGaugeVec> = Lazy::new(|| {
    let v = IntGaugeVec::new(
        Opts::new(
            "overlay_build_info",
            "Build info for svc-overlay (value is always 1)",
        ),
        &["version", "git"],
    )
    .expect("gauge vec");
    GLOBAL_REGISTRY
        .register(Box::new(v.clone()))
        .expect("register overlay_build_info");
    v
});

pub mod overlay_metrics {
    use super::*;
    pub fn ensure() {
        super::ensure_registered();
    }
    pub fn inc_sessions_active() {
        OVERLAY_SESSIONS_ACTIVE.inc();
    }
    pub fn dec_sessions_active() {
        OVERLAY_SESSIONS_ACTIVE.dec();
    }
    pub fn accept_latency_seconds(v: f64) {
        OVERLAY_ACCEPT_LATENCY_SECONDS.observe(v);
    }
    pub fn conn_lifetime_seconds(v: f64) {
        OVERLAY_CONN_LIFETIME_SECONDS.observe(v);
    }
    pub fn handshake_fail(reason: &'static str) {
        OVERLAY_HANDSHAKE_FAIL_TOTAL
            .with_label_values(&[reason])
            .inc();
    }
    pub fn set_build_info(version: &'static str, git: &'static str) {
        BUILD_INFO.with_label_values(&[version, git]).set(1);
    }
    pub fn set_peer_tx_depth(depth: usize) {
        OVERLAY_PEER_TX_DEPTH.set(depth as i64);
    }
    pub fn inc_peer_tx_dropped() {
        OVERLAY_PEER_TX_DROPPED_TOTAL.inc();
    }

    // NEW: lightweight getters for the sampler.
    pub fn get_peer_tx_depth() -> i64 {
        OVERLAY_PEER_TX_DEPTH.get()
    }
    pub fn get_sessions_active() -> i64 {
        OVERLAY_SESSIONS_ACTIVE.get()
    }
}

/// GET /metrics — Prometheus exposition format
pub async fn handle_metrics() -> impl IntoResponse {
    ensure_registered();

    let mut buf = Vec::with_capacity(64 * 1024);
    let encoder = TextEncoder::new();
    let metrics = prometheus::gather();
    if let Err(e) = encoder.encode(&metrics, &mut buf) {
        warn!(error=?e, "failed to encode prometheus metrics");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", "text/plain")],
            "encode failed",
        )
            .into_response();
    }

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE.as_str(), encoder.format_type())],
        buf,
    )
        .into_response()
}
