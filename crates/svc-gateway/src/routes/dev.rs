use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use crate::state::AppState;

pub async fn echo_post(State(_state): State<AppState>, body: Bytes) -> Response {
    let len = body.len();
    let payload = String::from_utf8_lossy(&body).to_string();
    let obj = serde_json::json!({ "len": len, "echo": payload });
    (StatusCode::OK, Json(obj)).into_response()
}

// Simple GET to exercise rate limit
pub async fn burst_ok() -> Response {
    (StatusCode::OK, "ok").into_response()
}

pub fn enabled() -> bool {
    matches!(std::env::var("SVC_GATEWAY_DEV_ROUTES").as_deref(), Ok("1"))
}
