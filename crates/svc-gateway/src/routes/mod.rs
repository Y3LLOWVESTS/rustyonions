//! Router assembly + core admin plane.
//!
//! RO:ORDER  Keep layers minimal; apply correlation + HTTP metrics to `/healthz` and the
//!           app-plane `/app/*` subtree, request-timeout + concurrency cap to `/readyz`,
//!           and body cap / rate limit only to dev routes. Optionally add `http_metrics`
//!           to dev routes when `SVC_GATEWAY_DEV_METRICS` is truthy for benching visibility.

use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub mod app;
pub mod dev;
pub mod health;
mod metrics;
pub mod ready;

/// Return true if `SVC_GATEWAY_DEV_METRICS` is set to a truthy value.
/// Accepted values (case-insensitive): "1", "true", "yes", "on".
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

    Router::new()
        .merge(health_with_layers)
        .merge(ready_with_guards)
        .merge(dev_routes)
        .merge(app_routes)
        .route("/metrics", get(metrics::get_metrics))
        .with_state(state.clone())
}
