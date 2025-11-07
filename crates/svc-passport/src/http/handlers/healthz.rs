//! RO:WHAT — GET /healthz (passthrough to ron-kernel health).
use axum::response::IntoResponse;
use ron_kernel::metrics::health::HealthState;
use std::sync::Arc;

pub async fn healthz(h: Arc<HealthState>) -> impl IntoResponse {
    if h.all_healthy() {
        "ok"
    } else {
        "degraded"
    }
}
