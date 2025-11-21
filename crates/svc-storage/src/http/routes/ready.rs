use axum::{extract::State, response::IntoResponse};
use http::StatusCode;

use crate::http::extractors::AppState;

pub async fn handler(State(_app): State<AppState>) -> impl IntoResponse {
    // If the HTTP server is bound and AppState is wired, report ready.
    // Deeper readiness gates (quotas, residency, etc.) can be added later.
    StatusCode::OK
}
