//! `/version` endpoint (no SHA), exposes crate name, semver, and build timestamp.

use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
struct VersionDto<'a> {
    name: &'a str,
    version: &'a str,
    built_at_unix: u64,
}

pub async fn handler() -> impl IntoResponse {
    let version = env!("CARGO_PKG_VERSION");
    let built_at_unix = option_env!("SVC_GATEWAY_BUILD_TS")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    Json(VersionDto {
        name: "svc-gateway",
        version,
        built_at_unix,
    })
}
