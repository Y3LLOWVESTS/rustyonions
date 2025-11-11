//! Response helpers & error mapping (to be extended as write plane lands).
use axum::{response::IntoResponse, Json};
use http::StatusCode;

pub fn err(status: StatusCode, code: &str, message: &str, corr_id: &str) -> impl IntoResponse {
    let body = serde_json::json!({
        "error": { "code": code, "message": message, "corr_id": corr_id }
    });
    (status, Json(body))
}
