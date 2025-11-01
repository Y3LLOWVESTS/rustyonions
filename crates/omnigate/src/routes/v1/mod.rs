//! RO:WHAT — v1 public surface (ping, basic read-only checks).
//! RO:WHY  — DTO-stable, tiny confidence checks for clients.
//! RO:INVARIANTS — Never leak internals; ping shape matches PingResponse.

use crate::types::PingResponse;
use axum::{routing::get, Json, Router};

pub fn router() -> Router {
    Router::new().route("/ping", get(ping))
}

/// Public so the top-level router in lib.rs can reference it directly.
pub async fn ping() -> Json<PingResponse> {
    // Current DTO is `{ ok: bool }` — no timestamp field.
    Json(PingResponse { ok: true })
}
