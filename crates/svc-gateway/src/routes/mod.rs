//! Router assembly + core admin/product plane.
//!
//! RO:WHAT — Compose `svc-gateway` HTTP routes and route-scoped middleware.
//! RO:WHY — Keep edge/admin/dev/app/WEB3 routes explicit and bounded.
//! RO:INTERACTS — `health`, `ready`, `metrics`, `dev`, `objects`, `app`, `paid_storage`, and `product` routes.
//! RO:INVARIANTS — correlation IDs on product/object paths; gateway stays proxy-only; no wallet/ledger mutation.
//! RO:METRICS — prewarms and applies HTTP metrics to health/app/paid/product/object routes.
//! RO:CONFIG — `SVC_GATEWAY_DEV_ROUTES`, `SVC_GATEWAY_DEV_METRICS`, upstream base URLs.
//! RO:SECURITY — skips ambient authority; proxy routes forward selected headers only.
//! RO:TEST — `app_proxy.rs`, `paid_storage_*_proxy.rs`, `product_routes_proxy.rs`, `smoke.rs`.

use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub mod app;
pub mod dev;
pub mod health;
mod metrics;
pub mod objects;
pub mod objects_range;
pub mod paid_storage;
pub mod product;
pub mod ready;
pub mod version;

/// Return true if `SVC_GATEWAY_DEV_METRICS` is set to a truthy value.
///
/// Accepted values, case-insensitive: `1`, `true`, `yes`, `on`.
fn dev_metrics_enabled() -> bool {
    match std::env::var("SVC_GATEWAY_DEV_METRICS") {
        Ok(v) => {
            let s = v.trim().to_ascii_lowercase();
            matches!(s.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => false,
    }
}

/// Build the public `svc-gateway` router.
pub fn build_router(state: &AppState) -> Router {
    // Ensure readiness sampler is ticking.
    crate::observability::readiness::ensure_started();

    // Prewarm metric label series so dashboards light up right away.
    crate::observability::http_metrics::prewarm();

    // --- /healthz and /version: correlation + request metrics ---
    let health_with_layers = Router::new()
        .route("/healthz", get(health::handler))
        .route("/version", get(version::handler))
        .route_layer(axum::middleware::from_fn(crate::layers::corr::mw))
        .route_layer(axum::middleware::from_fn(
            crate::observability::http_metrics::mw,
        ));

    // --- /readyz: guarded with timeout + concurrency cap ---
    let ready_with_guards = Router::new()
        .route("/readyz", get(ready::handler))
        .route_layer(axum::middleware::from_fn(
            crate::layers::timeouts::ready_timeout_mw,
        ))
        .route_layer(axum::middleware::from_fn(
            crate::layers::concurrency::ready_concurrency_mw,
        ));

    // --- /dev/*: body cap + rate limit; optionally add HTTP metrics when benching ---
    let dev_routes = if dev::enabled() {
        let dev_base = Router::new()
            .route("/dev/echo", post(dev::echo_post))
            .route("/dev/rl", get(dev::burst_ok))
            .route_layer(axum::middleware::from_fn(
                crate::layers::body_caps::body_cap_mw,
            ))
            .route_layer(axum::middleware::from_fn(
                crate::layers::rate_limit::rate_limit_mw,
            ));

        if dev_metrics_enabled() {
            dev_base.route_layer(axum::middleware::from_fn(
                crate::observability::http_metrics::mw,
            ))
        } else {
            dev_base
        }
    } else {
        Router::new()
    };

    // --- /o/:addr: raw object reads proxied directly to svc-storage ---
    let object_routes = Router::new()
        .route(
            "/o/:addr",
            get(objects::get_object).head(objects::head_object),
        )
        .route_layer(axum::middleware::from_fn(crate::layers::corr::mw))
        .route_layer(axum::middleware::from_fn(
            crate::observability::http_metrics::mw,
        ));

    // --- /app/*: app-plane proxy to omnigate with correlation + metrics ---
    let app_routes = Router::new()
        .nest("/app", app::router())
        .route_layer(axum::middleware::from_fn(crate::layers::corr::mw))
        .route_layer(axum::middleware::from_fn(
            crate::observability::http_metrics::mw,
        ));

    // --- /paid/*: WEB3 paid storage routes to omnigate with correlation + metrics ---
    let paid_routes = Router::new()
        .nest("/paid", paid_storage::router())
        .route_layer(axum::middleware::from_fn(crate::layers::corr::mw))
        .route_layer(axum::middleware::from_fn(
            crate::observability::http_metrics::mw,
        ));

    // --- WEB3_2 product routes: crab/b3/assets/sites to omnigate ---
    let product_routes = product::router()
        .route_layer(axum::middleware::from_fn(crate::layers::corr::mw))
        .route_layer(axum::middleware::from_fn(
            crate::observability::http_metrics::mw,
        ));

    Router::new()
        .merge(health_with_layers)
        .merge(ready_with_guards)
        .merge(dev_routes)
        .merge(object_routes)
        .merge(app_routes)
        .merge(paid_routes)
        .merge(product_routes)
        .route("/metrics", get(metrics::get_metrics))
        .with_state(state.clone())
}
