//! RO:WHAT — Router assembly for Micronode.
//! RO:WHY  — Central composition point for routes and layers.
//! RO:INTERACTS — config::schema::Config, http::{admin,routes,kv}, facets, layers,
//!                limits, state::AppState.
//! RO:INVARIANTS — Compose routers with state=(), then attach AppState once at the end.
//! RO:SECURITY — SecurityLayer (extract) + RequireAuthLayer (enforce) for KV & Facets.
//! RO:TEST — Covered by integration tests (admin parity, kv_roundtrip, guard_behavior, concurrency,
//!           facets, auth_gate).

use crate::{
    config::schema::Config,
    http::{admin, kv, routes},
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
use http::Error as HttpError;
use std::{convert::Infallible, path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;
use tower_http::trace::TraceLayer;

pub fn build_router(cfg: Config) -> (Router, AppState) {
    let st = AppState::new(cfg.clone());

    // Prewarm metrics.
    http_metrics::prewarm();

    // --- Admin plane ---
    let admin_routes = Router::new()
        .route("/healthz", get(admin::healthz))
        .route("/readyz", get(admin::readyz))
        .route("/version", get(admin::version))
        .route("/metrics", get(admin::metrics));

    // --- Dev plane (guarded) ---
    let dev = if st.cfg.server.dev_routes {
        let echo_conc = Arc::new(Semaphore::new(256));
        Router::new().route(
            "/dev/echo",
            post(routes::dev::echo)
                .layer::<_, HttpError>(ConcurrencyLayer::new(echo_conc))
                .layer(BodyCapLayer::new(HTTP_BODY_CAP_BYTES))
                .layer::<_, Infallible>(middleware::from_fn(decode_guard::guard))
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
            .layer(RequireAuthLayer::new(st.cfg.security.mode))
            .layer::<_, HttpError>(ConcurrencyLayer::new(kv_conc.clone()))
            .layer(BodyCapLayer::new(HTTP_BODY_CAP_BYTES))
            .layer::<_, Infallible>(middleware::from_fn(decode_guard::guard))
            .layer(SecurityLayer::new()),
    );

    // Compose top-level router core.
    let mut router = Router::new().merge(admin_routes).nest("/v1", api_v1).merge(dev);

    // --- Facets plane (manifest-driven if enabled) ---
    if st.cfg.facets.enabled {
        if let Some(dir) = st.cfg.facets.dir.clone() {
            let p = PathBuf::from(dir);
            match crate::facets::loader::load_facets(&p) {
                Ok(reg) => {
                    router = crate::facets::mount_with_registry(router, reg, st.cfg.security.mode);
                }
                Err(e) => {
                    // Loader error: make readiness reflect truth.
                    tracing::error!("facet loader failed: {e}");
                    st.probes.set_deps_ok(false);
                    router = crate::facets::mount(router); // keep meta + demo ping for visibility
                }
            }
        } else {
            tracing::warn!("facets.enabled=true but facets.dir not set");
            st.probes.set_deps_ok(false);
            router = crate::facets::mount(router);
        }
    } else {
        // Disabled => keep demo + empty meta for operator sanity.
        router = crate::facets::mount(router);
    }

    // Attach state + global observability.
    let router = router
        .with_state(st.clone())
        .layer(http_metrics::layer())
        .layer(TraceLayer::new_for_http());

    (router, st)
}
