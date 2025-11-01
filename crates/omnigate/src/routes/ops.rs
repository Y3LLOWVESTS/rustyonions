//! RO:WHAT — Ops/administration endpoints (version, health, ready).
//! RO:WHY  — Keep admin plane consistent and DTO-stable.
//! RO:INVARIANTS — Shapes match types::dto; no secret/PII in responses.

use crate::types::VersionResponse;
use axum::{response::IntoResponse, Json};

/// GET /versionz (or /ops/version if routed) — returns service version and optional git short hash.
/// Wire shape: VersionResponse { version: String, git: Option<String> }.
pub async fn versionz() -> impl IntoResponse {
    // Prefer compile-time embed from build.rs; fall back to runtime env (CI can export it).
    let git = option_env!("GIT_COMMIT_SHORT")
        .map(|s| s.to_string())
        .or_else(|| std::env::var("GIT_COMMIT_SHORT").ok());

    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        git,
    })
}

/// Back-compat shim so existing router entries calling `routes::ops::version` still work.
pub async fn version() -> impl IntoResponse {
    versionz().await
}
