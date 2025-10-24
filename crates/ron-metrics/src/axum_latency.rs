//! RO:WHAT — Axum middleware to observe request latency into ron-metrics.
//! RO:WHY  — Zero-touch per-request timing for Axum apps.
//! RO:INVARIANTS — no locks across .await; low overhead.
//! RO:USAGE — let app = axum_latency::attach(router, metrics.clone());

use std::time::Instant;

use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::{self, Next},
    response::Response,
    Router,
};
use crate::Metrics;

/// Attach a latency middleware to the given Router that records per-request
/// latency into `request_latency_seconds`.
///
/// ```ignore
/// let router = Router::new()
///     .route("/ping", get(|| async { "pong" }));
/// let router = ron_metrics::axum_latency::attach(router, metrics.clone());
/// ```
pub fn attach(router: Router, metrics: Metrics) -> Router {
    router.layer(middleware::from_fn_with_state(metrics, track_latency))
}

async fn track_latency(
    State(metrics): State<Metrics>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let started = Instant::now();
    let resp = next.run(req).await;
    metrics.observe_request(started.elapsed().as_secs_f64());
    resp
}
