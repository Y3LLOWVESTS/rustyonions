//! RO:WHAT — Router assembly for Micronode.
//! RO:WHY  — Central composition point for routes and layers.
//! RO:INTERACTS — config::schema::Config, http::{admin,routes,kv}, layers, limits, state::AppState.
//! RO:INVARIANTS — Outer trace layer; HTTP metrics just inside; per-route caps first.
//! RO:METRICS — HTTP metrics/trace via http_metrics + tower-http TraceLayer.
//! RO:CONFIG — Reads cfg.server.bind + cfg.server.dev_routes; storage engine later.
//! RO:SECURITY — No auth at router level; capability/policy live in layers/handlers.
//! RO:TEST — Covered by integration tests hitting /healthz,/readyz,/version,/metrics,/v1/kv.

use crate::{
    config::schema::Config,
    http::{admin, kv, routes},
    layers::{self, body_cap::BodyCapLayer, concurrency::ConcurrencyLayer},
    limits::HTTP_BODY_CAP_BYTES,
    observability::http_metrics,
    state::AppState,
};
use axum::{middleware, routing::get, Router};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::Semaphore;
use tower_http::trace::TraceLayer;

pub fn build_router(cfg: Config) -> (Router, AppState) {
    let st = AppState::new(cfg.clone());

    // Prewarm HTTP metrics so /metrics exposes micronode_http_* families immediately.
    http_metrics::prewarm();

    // --- Admin plane ---
    let admin_routes = Router::new()
        .route("/healthz", get(admin::healthz))
        .route("/readyz", get(admin::readyz))
        .route("/version", get(admin::version))
        .route("/metrics", get(admin::metrics));

    // --- Dev plane (guarded route) ---
    let dev = if st.cfg.server.dev_routes {
        let echo_conc = Arc::new(Semaphore::new(256)); // default per-route cap

        Router::new().route(
            "/dev/echo",
            axum::routing::post(routes::dev::echo)
                // Order matters: decode policy -> body cap -> concurrency
                .layer::<_, Infallible>(middleware::from_fn(layers::decode_guard::guard))
                .layer(BodyCapLayer::new(HTTP_BODY_CAP_BYTES))
                .layer(ConcurrencyLayer::new(echo_conc)),
        )
    } else {
        Router::new()
    };

    // --- Feature routes (v1 API) ---

    // Concurrency cap for KV operations; sized for small-node defaults.
    let kv_conc = Arc::new(Semaphore::new(256));

    let api_v1 = Router::new().route("/ping", get(routes::ping)).route(
        "/kv/:bucket/:key",
        axum::routing::get(kv::get_kv)
            .put(kv::put_kv)
            .delete(kv::delete_kv)
            // Same guard stack as dev echo: decode + body cap + concurrency.
            .layer::<_, Infallible>(middleware::from_fn(layers::decode_guard::guard))
            .layer(BodyCapLayer::new(HTTP_BODY_CAP_BYTES))
            .layer(ConcurrencyLayer::new(kv_conc)),
    );

    let router = Router::new()
        .merge(admin_routes)
        .nest("/v1", api_v1)
        .merge(dev)
        .with_state(st.clone())
        // Observability stack: metrics inner, tracing outer (so spans wrap metrics).
        .layer(http_metrics::layer())
        .layer(TraceLayer::new_for_http());

    (router, st)
}
