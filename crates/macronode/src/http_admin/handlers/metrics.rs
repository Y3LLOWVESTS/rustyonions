//! RO:WHAT — `/metrics` handler (Prometheus text).
//! RO:WHY  — Single scrape surface for admin metrics.

use crate::observability::metrics::encode_prometheus;
use axum::{http::StatusCode, response::IntoResponse};

pub async fn handler() -> impl IntoResponse {
    let body = encode_prometheus();
    (StatusCode::OK, body)
}
