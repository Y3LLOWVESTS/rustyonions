//! RO:WHAT — Axum middleware to count responses by status class (1xx..5xx).
//! RO:WHY  — Low-cardinality error-rate signal for SRE/alerts.
//! RO:USAGE — let app = axum_status::attach(router, metrics.clone());

use axum::{
    body::Body,
    http::Request,
    middleware::{self, Next},
    response::Response,
    Router,
};
use crate::Metrics;

pub fn attach(router: Router, metrics: Metrics) -> Router {
    router.layer(middleware::from_fn_with_state(metrics, count_status))
}

async fn count_status(
    axum::extract::State(metrics): axum::extract::State<Metrics>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let resp = next.run(req).await;
    let code = resp.status().as_u16();
    let class = match code {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "other",
    };
    metrics.observe_status_class(class);
    resp
}
