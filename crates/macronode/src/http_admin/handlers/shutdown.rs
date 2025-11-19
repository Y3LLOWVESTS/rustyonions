//! RO:WHAT — `/api/v1/shutdown` handler (MVP stub).
//! RO:WHY  — Placeholder for graceful shutdown wiring; currently just 202.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
struct ShutdownBody<'a> {
    status: &'a str,
}

pub async fn handler() -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(ShutdownBody {
            status: "shutdown not yet implemented",
        }),
    )
}
