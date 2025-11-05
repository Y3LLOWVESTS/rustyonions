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
    pub fn new() -> Self {
        Self {
            listeners_bound: AtomicBool::new(false),
            cfg_loaded: AtomicBool::new(false),
            metrics_bound: AtomicBool::new(false),
            deps_ok: AtomicBool::new(false),
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

// Clippy: new_without_default — provide a Default that delegates to new().
impl Default for ReadyProbes {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadySnapshot {
    pub listeners_bound: bool,
    pub cfg_loaded: bool,
    pub metrics_bound: bool,
    pub deps_ok: bool,
}

impl ReadySnapshot {
    /// REQUIRED probes for 200 OK. Adjust here if you want stricter gates.
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
