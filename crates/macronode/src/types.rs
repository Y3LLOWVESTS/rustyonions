//! RO:WHAT — Shared runtime types for Macronode.
//! RO:WHY  — Keep main/http modules thin by centralizing state and build info.

use std::{sync::Arc, time::Instant};

use crate::{config::Config, readiness::ReadyProbes};

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub probes: Arc<ReadyProbes>,
    pub started_at: Instant,
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
