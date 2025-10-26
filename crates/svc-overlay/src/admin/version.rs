//! RO:WHAT
//!   Build metadata surface (type + handler) for /version.
//! RO:WHY
//!   Bootstrap expects `BuildInfo` and `admin::router(probe, ver)`.
//! RO:INVARIANTS
//!   - Safe if git/build envs are missing (fall back to "unknown").

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

/// Static build metadata carried in admin state.
#[derive(Clone, Copy, Serialize)]
pub struct BuildInfo {
    pub version: &'static str,
    pub git: &'static str,
    pub build: &'static str,
    pub features: &'static [&'static str],
}

/// Construct BuildInfo from compile-time env (fallbacks allowed).
pub fn current_build_info() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        git: option_env!("GIT_SHA").unwrap_or("unknown"),
        build: option_env!("BUILD_TS").unwrap_or("unknown"),
        features: &[
            // e.g., "tls", "quic", "pq", "amnesia"
        ],
    }
}

/// GET /version — returns the build info from AdminState.
pub async fn handle_version(State(state): State<crate::admin::AdminState>) -> impl IntoResponse {
    Json(state.build)
}
