//! GET /o/{addr} with Range (separate route for clarity)

use axum::{extract::Path, response::IntoResponse};

pub async fn get_range(Path(addr): Path<String>) -> impl IntoResponse {
    // MVP: range not implemented yet
    (
        http::StatusCode::NOT_IMPLEMENTED,
        format!("range read stub for {}", addr),
    )
}
