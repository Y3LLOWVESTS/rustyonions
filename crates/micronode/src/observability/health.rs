//! RO:WHAT — /healthz handler adapter (thin wrapper if we need custom shape later).
//! RO:WHY  — Keep admin.rs simple.
//! RO:INVARIANTS — Truthful.

use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
struct Health {
    ok: bool,
}

pub async fn handler() -> impl IntoResponse {
    Json(Health { ok: true })
}
