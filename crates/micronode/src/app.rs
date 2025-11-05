// crates/micronode/src/app.rs
//! RO:WHAT — Router assembly for Micronode.
//! RO:WHY  — Central composition point for routes and layers.
//! RO:INVARIANTS — outer: tracing; per-route caps first. No ambient authority.

use crate::{
    config::schema::Config,
    http::{admin, kv, routes},
    layers::{self, body_cap::BodyCapLayer, concurrency::ConcurrencyLayer},
    limits::HTTP_BODY_CAP_BYTES,
    state::AppState,
};
use axum::{
    middleware,
    routing::get, // method chaining handles put/delete on the route builder
    Router,
};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::Semaphore;
use tower_http::trace::TraceLayer;

pub fn build_router(cfg: Config) -> (Router, AppState) {
    let st = AppState::new(cfg.clone());

    // In-memory storage is available, so deps_ok is truthful right now.
    st.probes.set_deps_ok(true);

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
                .layer::<_, Infallible>(BodyCapLayer::new(HTTP_BODY_CAP_BYTES))
                .layer::<_, Infallible>(ConcurrencyLayer::new(echo_conc)),
        )
    } else {
        Router::new()
    };

    // --- Feature routes ---
    let api_v1 = Router::new().route("/ping", get(routes::ping));

    // --- KV routes (/kv/{bucket}/{key}) with strict guards ---
    let kv_conc = Arc::new(Semaphore::new(256));
    let kv_routes = Router::new().route(
        "/:bucket/:key",
        get(kv::get_kv)
            .put(kv::put_kv)
            .delete(kv::del_kv)
            .layer::<_, Infallible>(middleware::from_fn(layers::decode_guard::guard))
            .layer::<_, Infallible>(BodyCapLayer::new(HTTP_BODY_CAP_BYTES))
            .layer::<_, Infallible>(ConcurrencyLayer::new(kv_conc)),
    );

    let router = Router::new()
        .merge(admin_routes)
        .nest("/v1", api_v1)
        .nest("/kv", kv_routes)
        .merge(dev)
        .with_state(st.clone())
        .layer(TraceLayer::new_for_http());

    (router, st)
}
