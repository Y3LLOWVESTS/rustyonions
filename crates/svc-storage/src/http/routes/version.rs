//! RO:WHAT — /version handler for svc-storage.
//! RO:WHY  — Operational introspection endpoint.

use axum::{response::IntoResponse, Json};
use serde::Serialize;

use crate::version::version_string;

#[derive(Serialize)]
struct VersionDto {
    version: String,
}

pub async fn handler() -> impl IntoResponse {
    Json(VersionDto {
        version: version_string(),
    })
}
