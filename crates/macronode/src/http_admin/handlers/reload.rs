//! RO:WHAT — `/api/v1/reload` handler (MVP stub).
//! RO:WHY  — Placeholder for config hot-reload; currently just 202.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
struct ReloadBody<'a> {
    status: &'a str,
}

pub async fn handler() -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(ReloadBody {
            status: "reload not yet implemented",
        }),
    )
}
