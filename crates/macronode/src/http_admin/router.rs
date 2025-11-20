//! RO:WHAT — Router builder for Macronode admin plane.

use std::sync::Arc;

use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};

use crate::{
    http_admin::middleware::{auth, rate_limit, request_id, timeout},
    readiness::{self, ReadyProbes},
    types::AppState,
};

pub fn build_router(state: AppState) -> Router {
    let probes: Arc<ReadyProbes> = state.probes.clone();

    let base = Router::new()
        .route(
            "/version",
            get(crate::http_admin::handlers::version::handler),
        )
        .route(
            "/healthz",
            get(crate::http_admin::handlers::healthz::handler),
        )
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
        .with_state(state);

    // Middleware stack:
    base.layer(from_fn(rate_limit::layer))
        .layer(from_fn(auth::layer)) // only applies to guarded paths
        .layer(from_fn(timeout::layer))
        .layer(from_fn(request_id::layer))
}
