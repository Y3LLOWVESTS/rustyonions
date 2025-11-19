//! RO:WHAT — Router builder for Macronode admin HTTP surface.
//! RO:WHY  — Single place to wire endpoints, middleware, and state.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    readiness::{self, ReadyProbes},
    types::AppState,
};

pub fn build_router(state: AppState) -> Router {
    let probes: Arc<ReadyProbes> = state.probes.clone();

    Router::new()
        .route(
            "/version",
            get(crate::http_admin::handlers::version::handler),
        )
        .route(
            "/healthz",
            get(crate::http_admin::handlers::healthz::handler),
        )
        // `/readyz` uses the shared probes directly.
        .route(
            "/readyz",
            get(move || {
                let probes = probes.clone();
                readiness::handler(probes)
            }),
        )
        .route(
            "/metrics",
            get(crate::http_admin::handlers::metrics::handler),
        )
        .route(
            "/api/v1/status",
            get(crate::http_admin::handlers::status::handler),
        )
        .route(
            "/api/v1/reload",
            post(crate::http_admin::handlers::reload::handler),
        )
        .route(
            "/api/v1/shutdown",
            post(crate::http_admin::handlers::shutdown::handler),
        )
        .with_state(state)
}
