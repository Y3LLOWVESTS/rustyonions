//! RO:WHAT — Readiness probes and `/readyz` handler (truthful by default).
//! RO:WHY  — Operators need a machine-readable snapshot of liveness gates.
//! RO:INVARIANTS
//!   - Required probes: listeners_bound && cfg_loaded.
//!   - Optional probes: metrics_bound, deps_ok (storage/index/etc).
//!   - Dev override via MICRONODE_DEV_READY=1 returns 200 immediately.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub struct ReadyProbes {
    listeners_bound: AtomicBool,
    cfg_loaded: AtomicBool,
    metrics_bound: AtomicBool,
    deps_ok: AtomicBool, // placeholder: storage/index/queue/etc
}

impl ReadyProbes {
    /// Construct probes with a conservative-but-truthful baseline for the
    /// current Micronode profile.
    ///
    /// For the in-memory storage engine, `deps_ok` is effectively always true
    /// once the process is up: there is no fallible external dependency to
    /// gate on. We initialise `deps_ok` to true so that "truthful" mode
    /// reflects reality today.
    ///
    /// When we add a fallible engine (sled / overlay / remote index), that
    /// engine's open result should drive `set_deps_ok(false|true)` instead.
    pub fn new() -> Self {
        Self {
            listeners_bound: AtomicBool::new(false),
            cfg_loaded: AtomicBool::new(false),
            metrics_bound: AtomicBool::new(false),
            deps_ok: AtomicBool::new(true),
        }
    }

    // --- Setters (flip true when satisfied) ---

    pub fn set_listeners_bound(&self, v: bool) {
        self.listeners_bound.store(v, Ordering::Release);
    }

    pub fn set_cfg_loaded(&self, v: bool) {
        self.cfg_loaded.store(v, Ordering::Release);
    }

    pub fn set_metrics_bound(&self, v: bool) {
        self.metrics_bound.store(v, Ordering::Release);
    }

    pub fn set_deps_ok(&self, v: bool) {
        self.deps_ok.store(v, Ordering::Release);
    }

    // --- Snapshot & decision ---

    pub fn snapshot(&self) -> ReadySnapshot {
        ReadySnapshot {
            listeners_bound: self.listeners_bound.load(Ordering::Acquire),
            cfg_loaded: self.cfg_loaded.load(Ordering::Acquire),
            metrics_bound: self.metrics_bound.load(Ordering::Acquire),
            deps_ok: self.deps_ok.load(Ordering::Acquire),
        }
    }
}

// Clippy: new-without-default — keep `new()` as the semantic ctor and
// delegate `Default` to it.
impl Default for ReadyProbes {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadySnapshot {
    pub listeners_bound: bool,
    pub cfg_loaded: bool,
    pub metrics_bound: bool,
    pub deps_ok: bool,
}

impl ReadySnapshot {
    /// REQUIRED probes for 200 OK. Adjust here if you want stricter gates.
    ///
    /// Today we keep this minimal: Micronode is "ready" once it is listening
    /// and config has been successfully loaded. Optional probes such as
    /// metrics_bound and deps_ok are still included in the JSON payload for
    /// operators and dashboards but do not flip the readiness bit.
    pub fn required_ready(&self) -> bool {
        self.listeners_bound && self.cfg_loaded
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ReadyReport {
    ready: bool,
    probes: ReadySnapshot,
    mode: &'static str, // "dev-forced" or "truthful"
}

pub async fn handler(probes: Arc<ReadyProbes>) -> impl IntoResponse {
    // Dev override: force ready for local benches/smokes.
    if matches!(
        std::env::var("MICRONODE_DEV_READY").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("on") | Ok("ON")
    ) {
        let snap = probes.snapshot();
        let report = ReadyReport { ready: true, probes: snap, mode: "dev-forced" };
        return (StatusCode::OK, Json(report)).into_response();
    }

    let snap = probes.snapshot();
    let ok = snap.required_ready();
    let status = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };

    let report = ReadyReport { ready: ok, probes: snap, mode: "truthful" };

    (status, Json(report)).into_response()
}
