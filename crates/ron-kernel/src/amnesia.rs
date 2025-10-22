//! RO:WHAT — Amnesia mode: single source of truth + metrics hook.
//! RO:WHY  — Pillar 1 (Kernel): central flag for RAM-first ops; SEC/RES concern.
//! RO:INTERACTS — metrics::exporter::Metrics (amnesia gauge), config::watcher/apply, readiness (orthogonal).
//! RO:INVARIANTS — lock-free reads; coherent under races; never gates readiness; updates metrics atomically.
//! RO:METRICS/LOGS — metrics.amnesia_mode (0/1 or label on="true|false") kept in sync on set().
//! RO:CONFIG — toggled by ConfigUpdated (from watcher); env RON_AMNESIA may also flip it.
//! RO:SECURITY — no secrets; boolean only.
//! RO:TEST HOOKS — unit: toggle coherency; integ: watcher flip updates gauge.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::metrics::exporter::Metrics;

/// Amnesia flag with atomic semantics and metrics synchronization.
#[derive(Clone, Debug)]
pub struct Amnesia(Arc<AtomicBool>);

impl Amnesia {
    /// Create with initial state.
    pub fn new(initial: bool) -> Self {
        Self(Arc::new(AtomicBool::new(initial)))
    }

    /// Read current state (lock-free).
    #[inline]
    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    /// Set state and synchronize the exported gauge.
    ///
    /// Never blocks; safe to call from config apply or env poller.
    pub fn set(&self, on: bool, metrics: &Metrics) {
        self.0.store(on, Ordering::Relaxed);
        metrics.set_amnesia(on);
    }
}
