//! RO:WHAT — Public and dev routes for Micronode.

use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

// --- /v1/ping ---
pub async fn ping() -> Json<Value> {
    Json(json!({ "pong": true }))
}

// --- /dev/echo ---
// Echoes JSON payload deterministically. Requires Content-Type: application/json
// and (by policy) Content-Length (enforced by BodyCap layer).
pub mod dev {
    use super::*;
    use axum::response::IntoResponse;

    /// Echo JSON back; reject non-JSON early.
    pub async fn echo(Json(body): Json<Value>) -> impl IntoResponse {
        (StatusCode::OK, Json(body))
    }
}
