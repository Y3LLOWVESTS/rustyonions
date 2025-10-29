//! Placeholder middleware (future: enforce ready before heavy ops).

use axum::response::Response;
use std::sync::Arc;

use crate::readiness::Readiness;

#[allow(dead_code)]
pub async fn require_ready(_ready: Arc<Readiness>) -> Response {
    // stub: in future, check readiness and short-circuit with 503
    Response::new(axum::body::Body::empty())
}
