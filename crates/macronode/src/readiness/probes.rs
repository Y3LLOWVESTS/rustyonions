// crates/macronode/src/readiness/probes.rs

//! RO:WHAT — In-process readiness probes and snapshot type.
//! RO:WHY  — Cheap, concurrency-friendly source of truth for `/readyz` and
//!           future status endpoints.
//!
//! RO:INVARIANTS —
//!   - All flags are atomic booleans with Release/Acquire semantics.
//!   - `required_ready()` encodes the essential gates for reporting
//!     `ready == true` in truthful mode.
//!   - Per-service bits (index/overlay/mailbox/dht) are tracked but do not
//!     gate readiness yet; they are surfaced in JSON only.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct ReadyProbes {
    // Essential gates
    listeners_bound: AtomicBool,
    cfg_loaded: AtomicBool,
    metrics_bound: AtomicBool,
    deps_ok: AtomicBool,
    gateway_bound: AtomicBool,

    // Per-service bits (informational for now)
    index_bound: AtomicBool,
    overlay_bound: AtomicBool,
    mailbox_bound: AtomicBool,
    dht_bound: AtomicBool,
}

impl ReadyProbes {
    /// Construct a fresh probe set with all gates set to `false`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            listeners_bound: AtomicBool::new(false),
            cfg_loaded: AtomicBool::new(false),
            metrics_bound: AtomicBool::new(false),
            deps_ok: AtomicBool::new(false),
            gateway_bound: AtomicBool::new(false),
            index_bound: AtomicBool::new(false),
            overlay_bound: AtomicBool::new(false),
            mailbox_bound: AtomicBool::new(false),
            dht_bound: AtomicBool::new(false),
        }
    }

    // --- Essential gate setters ---

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

    pub fn set_gateway_bound(&self, v: bool) {
        self.gateway_bound.store(v, Ordering::Release);
    }

    // --- Per-service setters (informational) ---

    pub fn set_index_bound(&self, v: bool) {
        self.index_bound.store(v, Ordering::Release);
    }

    pub fn set_overlay_bound(&self, v: bool) {
        self.overlay_bound.store(v, Ordering::Release);
    }

    pub fn set_mailbox_bound(&self, v: bool) {
        self.mailbox_bound.store(v, Ordering::Release);
    }

    pub fn set_dht_bound(&self, v: bool) {
        self.dht_bound.store(v, Ordering::Release);
    }

    /// Take a consistent snapshot for use by HTTP handlers / metrics.
    #[must_use]
    pub fn snapshot(&self) -> ReadySnapshot {
        ReadySnapshot {
            listeners_bound: self.listeners_bound.load(Ordering::Acquire),
            cfg_loaded: self.cfg_loaded.load(Ordering::Acquire),
            metrics_bound: self.metrics_bound.load(Ordering::Acquire),
            deps_ok: self.deps_ok.load(Ordering::Acquire),
            gateway_bound: self.gateway_bound.load(Ordering::Acquire),
            index_bound: self.index_bound.load(Ordering::Acquire),
            overlay_bound: self.overlay_bound.load(Ordering::Acquire),
            mailbox_bound: self.mailbox_bound.load(Ordering::Acquire),
            dht_bound: self.dht_bound.load(Ordering::Acquire),
        }
    }
}

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
    pub gateway_bound: bool,
    pub index_bound: bool,
    pub overlay_bound: bool,
    pub mailbox_bound: bool,
    pub dht_bound: bool,
}

impl ReadySnapshot {
    /// Essential readiness gates for reporting `"ready": true`.
    ///
    /// Deliberately *does not* include per-service bits yet. Once the
    /// non-core planes are wired and stable, we can tighten this gate.
    #[must_use]
    pub fn required_ready(&self) -> bool {
        self.listeners_bound && self.cfg_loaded && self.deps_ok && self.gateway_bound
    }
}
