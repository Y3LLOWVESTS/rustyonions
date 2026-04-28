//! RO:WHAT — Liveness and readiness HTTP handlers for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: RES/GOV. Operators need truthful health and degrade-first readiness.
//! RO:INTERACTS — readiness::ReadinessGate, metrics, supervisor/main.
//! RO:INVARIANTS — /healthz is always liveness; /readyz returns 503 when writes must shed.
//! RO:METRICS — request counts are recorded by handler-level guard.
//! RO:CONFIG — none.
//! RO:SECURITY — no secrets or account data in health bodies.
//! RO:TEST — covered by route smoke once integration tests are wired.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Serialize;

use crate::routes::WalletState;

/// Liveness response body.
#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HealthResponse {
    /// Service name.
    pub service: &'static str,
    /// Liveness flag.
    pub ok: bool,
}

/// GET /healthz.
pub async fn healthz(State(state): State<WalletState>) -> impl IntoResponse {
    let _guard = state.metrics.begin_request();
    state.metrics.inc_success();

    Json(HealthResponse {
        service: "svc-wallet",
        ok: true,
    })
}

/// GET /readyz.
pub async fn readyz(State(state): State<WalletState>) -> impl IntoResponse {
    let _guard = state.metrics.begin_request();
    let snapshot = state.readiness.snapshot();

    if snapshot.ready {
        state.metrics.inc_success();
        (StatusCode::OK, Json(snapshot)).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::RETRY_AFTER, "1")],
            Json(snapshot),
        )
            .into_response()
    }
}
