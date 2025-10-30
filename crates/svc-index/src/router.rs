//! RO:WHAT — HTTP router (routes + middleware).
//! RO:WHY  — Keep as Router<Arc<AppState>>; main.rs injects state via .with_state(...).
//! RO:INVARIANTS — Handlers use State<Arc<AppState>>.

use std::sync::Arc;

use axum::{routing::get, Router};

use crate::{
    constants::MAX_BODY_BYTES,
    http::{middleware, routes},
    state::AppState,
};

pub fn build_router() -> Router<Arc<AppState>> {
    let api = Router::new()
        .route("/healthz", get(routes::health::healthz))
        .route("/readyz", get(routes::health::readyz))
        .route("/version", get(routes::version::version))
        .route("/metrics", get(routes::metrics::metrics))
        // Generic key resolver: supports "name:*" or "b3:*"
        .route("/resolve/:key", get(routes::resolve::resolve))
        .route("/providers/:cid", get(routes::providers::providers));

    Router::new()
        .nest("/", api)
        .layer(middleware::trace_layer::layer())
        .layer(middleware::body_limits::layer(MAX_BODY_BYTES))
}
