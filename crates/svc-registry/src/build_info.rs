//! RO:WHAT — Canonical build metadata struct and constructor.
//! RO:WHY  — Shared across admin/version and anywhere else needing service build info.
//! RO:INTERACTS — Used by `observability::endpoints::AdminState` and `/version` handler.

use serde::Serialize;

/// Service build metadata returned by `/version`.
#[derive(Clone, Debug, Serialize)]
pub struct BuildInfo {
    pub service: &'static str,
    pub version: &'static str,
    /// Build fingerprint (prefer git SHA; fall back to BLAKE3 or "unknown").
    pub commit: &'static str,
    pub schema: serde_json::Value,
    pub deprecations: Vec<String>,
}

/// Construct the current build info.
/// Priority: VERGEN_GIT_SHA (or GIT_COMMIT_SHA) → RON_BUILD_B3 → "unknown".
pub fn build_info() -> BuildInfo {
    // Prefer standard git SHA if present (via `vergen`, CI export, or custom build.rs).
    let commit = option_env!("VERGEN_GIT_SHA")
        .or(option_env!("GIT_COMMIT_SHA"))
        // Optional: your BLAKE3 fallback from CI/build.rs
        .or(option_env!("RON_BUILD_B3"))
        .unwrap_or("unknown");

    BuildInfo {
        service: "svc-registry",
        version: env!("CARGO_PKG_VERSION"),
        commit,
        schema: serde_json::json!({
            "registry": "1.0.0"
        }),
        deprecations: Vec::new(),
    }
}
