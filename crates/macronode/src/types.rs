// crates/macronode/src/types.rs

//! RO:WHAT — Shared runtime types for Macronode.
//! RO:WHY  — Keep main/http modules thin by centralizing state and build info.

use std::{sync::Arc, time::Instant};

use crate::{config::Config, readiness::ReadyProbes, supervisor::ShutdownToken};

#[derive(Clone)]
pub struct AppState {
    /// Effective runtime config for this macronode process.
    pub cfg: Arc<Config>,
    /// Shared readiness probes used by `/readyz` and `/api/v1/status`.
    pub probes: Arc<ReadyProbes>,
    /// Timestamp when the node finished its initial bootstrap.
    pub started_at: Instant,
    /// Cooperative shutdown token shared with the supervisor and services.
    ///
    /// NOTE: We currently trigger shutdown via a blunt `process::exit(0)` in
    /// the `/api/v1/shutdown` handler, so this field is not yet read. We keep
    /// it here so the CLI wiring stays aligned with the future graceful
    /// shutdown design.
    #[allow(dead_code)]
    pub shutdown: ShutdownToken,
}

/// Build-time info used by `/version`.
///
/// We keep this minimal for now; once a build script is in place
/// we can plumb git SHA, build timestamp, and rustc/msrv versions.
pub struct BuildInfo {
    pub service: &'static str,
    pub version: &'static str,
    pub git_sha: &'static str,
    pub build_ts: &'static str,
    pub rustc: &'static str,
    pub msrv: &'static str,
}

impl BuildInfo {
    pub fn current() -> Self {
        Self {
            service: "macronode",
            version: env!("CARGO_PKG_VERSION"),
            git_sha: "unknown",
            build_ts: "unknown",
            rustc: "unknown",
            msrv: "1.80.0",
        }
    }
}
