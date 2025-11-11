//! Temporary API route(s) to exercise the admission chain.

use axum::{extract::State, body::Bytes, response::IntoResponse, Json};
use http::StatusCode;

use crate::state::AppState;

/// POST /echo — echos back the body length as JSON.
///
/// This is intentionally simple so we can validate body caps, timeouts,
/// RPS shaping, etc., without introducing domain behavior yet.
pub async fn echo(State(_state): State<AppState>, body: Bytes) -> impl IntoResponse {
    let len = body.len();
    let payload = serde_json::json!({ "ok": true, "len": len });
    (StatusCode::OK, Json(payload))
}
