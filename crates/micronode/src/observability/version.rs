//! RO:WHAT — /version payload.
//! RO:WHY  — Build provenance for ops.

use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
struct VersionResp<'a> {
    name: &'a str,
    version: &'a str,
    built_at_unix: u64,
}

pub async fn handler() -> impl IntoResponse {
    let built =
        option_env!("MICRONODE_BUILD_UNIX").and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    Json(VersionResp {
        name: "micronode",
        version: env!("CARGO_PKG_VERSION"),
        built_at_unix: built,
    })
}
