//! RO:WHAT — GET /healthz (passthrough to ron-kernel health).
use axum::response::IntoResponse;
use ron_kernel::metrics::health::HealthState;

pub async fn healthz(h: HealthState) -> impl IntoResponse {
    if h.all_ready() {
        "ok"
    } else {
        "degraded"
    }
}
