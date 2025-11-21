use axum::{extract::State, response::IntoResponse};
use http::StatusCode;

use crate::http::extractors::AppState;

pub async fn handler(State(_app): State<AppState>) -> impl IntoResponse {
    // If the process is up and Axum can extract AppState, consider it healthy.
    StatusCode::OK
}
