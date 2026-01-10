// crates/micronode/src/http/admin.rs
//! Minimal "admin plane" endpoints for micronode.
//!
//! IMPORTANT:
//! - Keep these endpoints truthful.
//! - Prometheus scrape should expose BOTH:
//!     (1) default registry metrics (http middleware, etc.)
//!     (2) ron-kernel registry metrics (svc-admin freshness counters, etc.)

use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use prometheus::{Encoder, TextEncoder};

use crate::state::AppState;

fn prometheus_content_type() -> HeaderValue {
    HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8")
}

/// Liveness: process is running.
pub async fn healthz(State(st): State<AppState>) -> impl IntoResponse {
    if st.health.all_ready() {
        (StatusCode::OK, "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "degraded")
    }
}

/// Readiness: truthful readiness gate.
pub async fn readyz(State(st): State<AppState>) -> impl IntoResponse {
    let snap = st.probes.snapshot();
    if snap.required_ready() {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    }
}

/// Version info.
pub async fn version() -> impl IntoResponse {
    (StatusCode::OK, env!("CARGO_PKG_VERSION"))
}

/// Prometheus scrape endpoint.
/// Exposes BOTH the default registry and the ron-kernel registry.
pub async fn metrics(State(st): State<AppState>) -> impl IntoResponse {
    let encoder = TextEncoder::new();

    // Gather from the ron-kernel registry first (includes ron_facet_requests_total, etc.)
    let mut metric_families = st.metrics.registry.gather();

    // Also include default registry metrics (http middleware, etc.)
    metric_families.extend(prometheus::gather());

    let mut buf = Vec::with_capacity(16 * 1024);
    if encoder.encode(&metric_families, &mut buf).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "encode failed").into_response();
    }

    let mut resp: Response = buf.into_response();
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, prometheus_content_type());
    resp
}
