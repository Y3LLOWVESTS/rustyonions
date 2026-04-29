//! Router assembly + core admin plane.
//!
//! RO:WHAT — Compose `svc-gateway` HTTP routes and route-scoped middleware.
//! RO:WHY — Keep edge/admin/dev/app/WEB3 routes explicit and bounded.
//! RO:INTERACTS — health, ready, metrics, dev, app, `paid_storage` routes, and gateway layers.
//! RO:INVARIANTS — correlation IDs on product paths; body caps on dev routes; paid estimate is read-only.
//! RO:METRICS — prewarms and applies HTTP metrics to health/app/paid routes.
//! RO:CONFIG — `SVC_GATEWAY_DEV_ROUTES`, `SVC_GATEWAY_DEV_METRICS`, upstream base URLs.
//! RO:SECURITY — skips ambient authority; proxy routes forward selected headers only.
//! RO:TEST — `app_proxy.rs`, `paid_storage_estimate_proxy.rs`, `smoke.rs`.

use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub mod app;
pub mod dev;
pub mod health;
mod metrics;
pub mod paid_storage;
pub mod ready;

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

pub fn build_router(state: &AppState) -> Router {
    // Ensure readiness sampler is ticking.
    crate::observability::readiness::ensure_started();

    // Prewarm metric label series so dashboards light up right away.
    crate::observability::http_metrics::prewarm();

    // --- /healthz: correlation + request metrics (outermost) ---
    let health_with_layers = Router::new()
        .route("/healthz", get(health::handler))
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

    // --- /dev/*: body cap + rate limit; optionally add http_metrics when benching ---
    let dev_routes = if dev::enabled() {
        let dev_base = Router::new()
            .route("/dev/echo", post(dev::echo_post))
            .route("/dev/rl", get(dev::burst_ok))
            // inner: functional guards
            .route_layer(axum::middleware::from_fn(
                crate::layers::body_caps::body_cap_mw,
            ))
            .route_layer(axum::middleware::from_fn(
                crate::layers::rate_limit::rate_limit_mw, // lock-free RL
            ));

        // If enabled, make http_metrics the outermost layer on /dev/*
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

    // --- /app/*: app-plane proxy to omnigate with correlation + metrics ---
    let app_routes = Router::new()
        // App-plane proxy: /app/* → omnigate /v1/app/*
        .nest("/app", app::router())
        .route_layer(axum::middleware::from_fn(crate::layers::corr::mw))
        .route_layer(axum::middleware::from_fn(
            crate::observability::http_metrics::mw,
        ));

    // --- /paid/*: WEB3 paid preflight routes to omnigate with correlation + metrics ---
    let paid_routes = Router::new()
        // Paid storage estimate: /paid/o/estimate → omnigate /v1/paid/o/estimate
        .nest("/paid", paid_storage::router())
        .route_layer(axum::middleware::from_fn(crate::layers::corr::mw))
        .route_layer(axum::middleware::from_fn(
            crate::observability::http_metrics::mw,
        ));

    Router::new()
        .merge(health_with_layers)
        .merge(ready_with_guards)
        .merge(dev_routes)
        .merge(app_routes)
        .merge(paid_routes)
        .route("/metrics", get(metrics::get_metrics))
        .with_state(state.clone())
}
