use axum::response::IntoResponse;
use http::StatusCode;

use crate::http::extractors::AppState;
use axum::Extension;

pub async fn handler(_: Extension<AppState>) -> impl IntoResponse {
    StatusCode::OK
}
