//! RO:WHAT — Admin plane endpoints (/metrics, /healthz, /readyz, /version) + readiness gates.
//! RO:WHY  — Truthful SLO surfaces and a simple flip-to-ready mechanism.

use std::sync::Arc;

use axum::{routing::get, Router};
use http::StatusCode;
use ron_kernel::HealthState;

use crate::build_info::BuildInfo;
use crate::observability::metrics::RegistryMetrics;

/// Shared admin-plane state.
#[derive(Clone)]
pub struct AdminState {
    pub health: Arc<HealthState>,
    pub build: BuildInfo,
    pub metrics: RegistryMetrics,
}

// Helpers to flip gates — call from main after init.
pub fn set_services_ok(health: &Arc<HealthState>, ok: bool) {
    health.set("services_ok", ok);
}
pub fn set_queues_ok(health: &Arc<HealthState>, ok: bool) {
    health.set("queues_ok", ok);
}

pub fn admin_router(state: AdminState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/version", get(version))
        .route("/metrics", get(metrics))
        .with_state(state)
}

async fn healthz() -> impl axum::response::IntoResponse {
    (StatusCode::OK, "")
}

async fn readyz(
    axum::extract::State(st): axum::extract::State<AdminState>,
) -> impl axum::response::IntoResponse {
    let snap = st.health.snapshot();
    let services_ok = snap.get("services_ok").copied().unwrap_or(false);
    let queues_ok = snap.get("queues_ok").copied().unwrap_or(false);
    let ready = services_ok && queues_ok;
    let degraded = !(services_ok && queues_ok);

    let body = serde_json::json!({
        "ready": ready,
        "degraded": degraded,
        "services_ok": services_ok,
        "queues_ok": queues_ok
    });

    if ready {
        (StatusCode::OK, axum::Json(body))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, axum::Json(body))
    }
}

async fn version(
    axum::extract::State(st): axum::extract::State<AdminState>,
) -> impl axum::response::IntoResponse {
    // Return an owned clone to avoid borrowing state across the response.
    axum::Json(st.build.clone())
}

async fn metrics(
    axum::extract::State(st): axum::extract::State<AdminState>,
) -> impl axum::response::IntoResponse {
    // Use the instance method (fixes E0061 after making gather_text non-static).
    let body = st.metrics.gather_text();
    (StatusCode::OK, body)
}
