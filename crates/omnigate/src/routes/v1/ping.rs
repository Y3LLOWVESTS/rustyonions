//! RO:WHAT   GET /v1/ping handler
//! RO:WHY    Minimal health style endpoint used by benches and smoke tests.

use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct PingResponse {
    pub ok: bool,
}

pub async fn handler() -> impl IntoResponse {
    Json(PingResponse { ok: true })
}
