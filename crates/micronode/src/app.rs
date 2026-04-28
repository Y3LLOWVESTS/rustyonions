// crates/micronode/src/app.rs

//! RO:WHAT — Router assembly for Micronode.
//! RO:WHY  — Central composition point for routes and layers.
//! RO:INTERACTS — config::schema::Config, http::{admin,admin_api,routes,kv}, facets, layers,
//!                limits, state::AppState.
//! RO:INVARIANTS — Compose routers with state=(), then attach AppState once at the end.

use crate::{
    config::schema::Config,
    http::{admin, admin_api, kv, routes},
    layers::{
        body_cap::BodyCapLayer,
        concurrency::ConcurrencyLayer,
        decode_guard,
        security::{RequireAuthLayer, SecurityLayer},
    },
    limits::HTTP_BODY_CAP_BYTES,
    observability::http_metrics,
    state::AppState,
};
use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;
use tower_http::trace::TraceLayer;

pub fn build_router(cfg: Config) -> (Router, AppState) {
    // Build state once. Router stays `state=()` until the final `.with_state(...)`.
    let st = AppState::new(cfg.clone());

    // Freeze commonly-used config values so we don't thread `st` everywhere.
    let dev_routes_enabled = cfg.server.dev_routes;
    let security_mode = cfg.security.mode.clone();
    let facets_cfg = cfg.facets.clone();

    // Prewarm metrics.
    http_metrics::prewarm();

    // --- Admin plane (basic) ---
    let admin_routes = Router::new()
        .route("/healthz", get(admin::healthz))
        .route("/readyz", get(admin::readyz))
        .route("/version", get(admin::version))
        .route("/metrics", get(admin::metrics));

    // --- Admin API (svc-admin contract) ---
    let admin_api_routes = Router::new()
        .route("/api/v1/status", get(admin_api::status))
        .route("/api/v1/system/summary", get(admin_api::system_summary))
        .route("/api/v1/storage/summary", get(admin_api::storage_summary));

    // --- Dev plane (guarded) ---
    let dev = if dev_routes_enabled {
        let echo_conc = Arc::new(Semaphore::new(256));
        Router::new().route(
            "/dev/echo",
            post(routes::dev::echo)
                // axum 0.7 MethodRouter::layer needs a concrete NewError; pick axum::http::Error
                .layer::<_, axum::http::Error>(ConcurrencyLayer::new(echo_conc))
                .layer(BodyCapLayer::new(HTTP_BODY_CAP_BYTES))
                .layer(middleware::from_fn(decode_guard::guard))
                .layer(SecurityLayer::new()),
        )
    } else {
        Router::new()
    };

    // --- API v1 (public) ---
    let kv_conc = Arc::new(Semaphore::new(256));
    let api_v1 = Router::new().route("/ping", get(routes::ping)).route(
        "/kv/:bucket/:key",
        put(kv::put_kv)
            .delete(kv::delete_kv)
            .get(kv::get_kv)
            .layer(RequireAuthLayer::new(security_mode.clone()))
            // axum 0.7 MethodRouter::layer needs a concrete NewError; pick axum::http::Error
            .layer::<_, axum::http::Error>(ConcurrencyLayer::new(kv_conc))
            .layer(BodyCapLayer::new(HTTP_BODY_CAP_BYTES))
            .layer(middleware::from_fn(decode_guard::guard))
            .layer(SecurityLayer::new()),
    );

    // Compose top-level router core.
    let mut router =
        Router::new().merge(admin_routes).merge(admin_api_routes).nest("/v1", api_v1).merge(dev);

    // --- Facets plane (manifest-driven if enabled) ---
    if facets_cfg.enabled {
        if let Some(dir) = facets_cfg.dir.clone() {
            let p = PathBuf::from(dir);
            match crate::facets::loader::load_facets(&p) {
                Ok(reg) => {
                    router = crate::facets::mount_with_registry(router, reg, security_mode.clone());
                }
                Err(e) => {
                    tracing::error!("facet loader failed: {e}");
                    st.probes.set_deps_ok(false);
                    router = crate::facets::mount(router);
                }
            }
        } else {
            tracing::warn!("facets.enabled=true but facets.dir not set");
            st.probes.set_deps_ok(false);
            router = crate::facets::mount(router);
        }
    } else {
        router = crate::facets::mount(router);
    }

    // Attach state + global observability (ONLY ONCE).
    let router = router
        .with_state(st.clone())
        .layer(http_metrics::layer())
        .layer(TraceLayer::new_for_http());

    (router, st)
}
