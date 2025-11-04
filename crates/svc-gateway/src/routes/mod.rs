//! Router assembly + core admin plane.
//! RO:ORDER  Keep layers minimal; apply correlation + HTTP metrics to `/healthz`,
//!           request-timeout + concurrency cap to `/readyz`, and body cap / rate limit
//!           only to dev routes.

use crate::state::AppState;
use axum::{routing::{get, post}, Router};

pub mod health;
pub mod ready;
mod metrics;
pub mod dev;
pub mod version; // /version returns name/version/build timestamp (no SHA)

/// Build the router with admin-plane routes.
/// Takes `&AppState` to match the current bin; clones internally for `with_state`.
pub fn build_router(state: &AppState) -> Router {
    // Start readiness sampler (idempotent).
    crate::observability::readiness::ensure_started();

    // /healthz with correlation + http_metrics (observability)
    let health_with_layers = Router::new()
        .route("/healthz", get(health::handler))
        .route_layer(axum::middleware::from_fn(crate::layers::corr::mw))
        .route_layer(axum::middleware::from_fn(
            crate::observability::http_metrics::mw,
        ));

    // /readyz with a small request timeout and concurrency cap
    let ready_with_guards = Router::new()
        .route("/readyz", get(ready::handler))
        .route_layer(axum::middleware::from_fn(
            crate::layers::timeouts::ready_timeout_mw,
        ))
        .route_layer(axum::middleware::from_fn(
            crate::layers::concurrency::ready_concurrency_mw,
        ));

    // Optional dev subrouter (POST /dev/echo, GET /dev/rl) with body-size cap and rate-limit
    let dev_routes = if dev::enabled() {
        Router::new()
            .route("/dev/echo", post(dev::echo_post))
            .route_layer(axum::middleware::from_fn(
                crate::layers::body_caps::body_cap_mw,
            ))
            .route("/dev/rl", get(dev::burst_ok))
            .route_layer(axum::middleware::from_fn(
                crate::layers::rate_limit::rate_limit_mw,
            ))
    } else {
        Router::new()
    };

    Router::new()
        .merge(health_with_layers)
        .merge(ready_with_guards)
        .merge(dev_routes)
        .route("/version", get(version::handler))
        .route("/metrics", get(metrics::get_metrics))
        .with_state(state.clone())
}
