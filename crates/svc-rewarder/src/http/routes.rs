//! RO:WHAT — Axum router construction for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/RES. Defines the stable HTTP service surface.
//! RO:INTERACTS — http::handlers, RewarderState, Axum routing.
//! RO:INVARIANTS — stable /rewarder/v1-style epoch paths; health/ready/metrics always exposed.
//! RO:METRICS — route handlers update metrics.
//! RO:CONFIG — bind/config is handled by main/bootstrap, not this file.
//! RO:SECURITY — write/inspect scopes are enforced by handlers.
//! RO:TEST — integration/http_compute.rs and manual curl smoke.

use axum::routing::{get, post};
use axum::Router;

use crate::http::handlers;
use crate::http::RewarderState;

/// Build the svc-rewarder HTTP router.
pub fn router(state: RewarderState) -> Router {
    Router::new()
        .route("/healthz", get(handlers::healthz))
        .route("/readyz", get(handlers::readyz))
        .route("/metrics", get(handlers::metrics))
        .route("/version", get(handlers::version))
        .route(
            "/rewarder/epochs/:epoch_id/compute",
            post(handlers::compute_epoch),
        )
        .route("/rewarder/epochs/:epoch_id", get(handlers::get_epoch))
        .route(
            "/rewarder/epochs/:epoch_id/settlement",
            get(handlers::get_settlement),
        )
        .with_state(state)
}
