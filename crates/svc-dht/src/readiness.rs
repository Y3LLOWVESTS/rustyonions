//! RO:WHAT — Readiness gate: flips when bootstrap quorum + min-fill thresholds met
//! RO:WHY — Prevents thundering herd; Concerns: RES/PERF
//! RO:INTERACTS — bootstrap, peer::table, /readyz
//! RO:INVARIANTS — set ready last; fail-closed on writes
//! RO:TEST — readiness_bootstrap.rs

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct ReadyGate {
    ready: AtomicBool,
}
impl ReadyGate {
    pub fn new() -> Self {
        Self { ready: AtomicBool::new(false) }
    }
    pub fn set_ready(&self) {
        self.ready.store(true, Ordering::Release);
    }
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
}

pub type SharedReady = Arc<ReadyGate>;
