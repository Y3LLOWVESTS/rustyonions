//! RO:WHAT — HTTP router (routes + middleware).
//! RO:WHY  — Keep as Router<Arc<AppState>>; main.rs injects state via .with_state(...).
//! RO:INVARIANTS — Handlers use State<Arc<AppState>>; no business logic in router.

use std::sync::Arc;

use axum::{
    routing::{get, put},
    Router,
};

use crate::{
    constants::MAX_BODY_BYTES,
    http::{middleware, routes},
    state::AppState,
};

/// Build the svc-index HTTP router.
pub fn build_router() -> Router<Arc<AppState>> {
    let api = Router::new()
        .route("/healthz", get(routes::health::healthz))
        .route("/readyz", get(routes::health::readyz))
        .route("/version", get(routes::version::version))
        .route("/metrics", get(routes::metrics::metrics))
        // Generic key resolver: supports "name:*" or "b3:*".
        .route("/resolve/:key", get(routes::resolve::resolve))
        .route("/providers/:cid", get(routes::providers::providers))
        // WEB3_2 manifest pointer foundation.
        .route(
            "/v1/index/assets/:asset_cid/manifest",
            put(routes::index_manifests::put_asset_manifest)
                .get(routes::index_manifests::get_asset_manifest),
        )
        .route(
            "/v1/index/sites/:name/manifest",
            put(routes::index_manifests::put_site_manifest)
                .get(routes::index_manifests::get_site_manifest),
        );

    Router::new()
        .nest("/", api)
        .layer(middleware::trace_layer::layer())
        .layer(middleware::body_limits::layer(MAX_BODY_BYTES))
}
