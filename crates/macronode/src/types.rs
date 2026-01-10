//! RO:WHAT — Shared runtime types for Macronode.
//! RO:WHY  — Keep main/http modules thin by centralizing state and build info.
//! RO:INVARIANTS —
//!   - AppState is cheap to clone (Arc-backed).
//!   - Handlers must not hold locks across .await.
//!   - BuildInfo is stable, small, and safe to expose publicly.

#![forbid(unsafe_code)]

use std::{sync::Arc, time::Instant};

use crate::{bench::BenchManager, bus::NodeBus, config::Config, readiness::ReadyProbes};

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub probes: Arc<ReadyProbes>,
    /// Intra-node event bus used for KernelEvent traffic (config updates,
    /// health changes, crash notices, etc.).
    pub bus: NodeBus,
    pub started_at: Instant,

    /// Node-executed benchmark manager (bounded, safe loadgen).
    pub bench: Arc<BenchManager>,
}

#[derive(Clone)]
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
            git_sha: option_env!("RON_GIT_SHA").unwrap_or("unknown"),
            build_ts: option_env!("RON_BUILD_TS").unwrap_or("unknown"),
            rustc: option_env!("RON_RUSTC").unwrap_or("unknown"),
            msrv: "1.80.0",
        }
    }
}
