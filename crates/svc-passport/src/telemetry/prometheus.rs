//! RO:WHAT — Expose default Prometheus registry + simple health handler.
use axum::{http::StatusCode, response::IntoResponse};
use prometheus::{Encoder, TextEncoder};
use ron_kernel::metrics::health::HealthState;
use std::sync::Arc;

pub async fn metrics_handler() -> impl IntoResponse {
    let mf = prometheus::default_registry().gather();
    let mut buf = Vec::new();
    TextEncoder::new().encode(&mf, &mut buf).unwrap();
    (StatusCode::OK, buf)
}

pub async fn healthz_handler(h: Arc<HealthState>) -> impl IntoResponse {
    if h.all_healthy() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
