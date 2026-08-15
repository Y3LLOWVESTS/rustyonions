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
        )
        .route(
            "/v1/index/explore",
            get(
                routes::explore_discovery::
                    explore_discovery,
            ),
        )
        .route(
            "/v1/index/creators/:username/publications",
            get(
                routes::creator_publications::
                    list_creator_publications,
            ),
        )
        .route(
            "/v1/index/creators/:username/publications/:publication_id",
            put(
                routes::creator_publications::
                    put_creator_publication,
            )
            .get(
                routes::creator_publications::
                    get_creator_publication,
            ),
        )
        .route(
            "/v1/index/publication-relations",
            put(
                routes::publication_relations::
                    put_publication_relation,
            )
            .get(
                routes::publication_relations::
                    list_publication_relations,
            ),
        )
        .route(
            "/v1/index/site-publications",
            put(
                routes::site_publications::
                    put_site_publication,
            )
            .get(
                routes::site_publications::
                    list_site_publications,
            ),
        );

    Router::new()
        .nest("/", api)
        .layer(middleware::trace_layer::layer())
        .layer(middleware::body_limits::layer(MAX_BODY_BYTES))
}
