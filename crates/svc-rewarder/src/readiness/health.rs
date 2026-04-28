//! RO:WHAT — Health and readiness gate state for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/PERF. Service must degrade before economic correctness is at risk.
//! RO:INTERACTS — http handlers, metrics, main bootstrap.
//! RO:INVARIANTS — readiness is truthful; missing keys are stable; no lock across await.
//! RO:METRICS — readyz_degraded gauges updated by state changes.
//! RO:CONFIG — config_loaded gate flips after config validation.
//! RO:SECURITY — readiness leaks only dependency class, not credentials.
//! RO:TEST — integration/readiness.rs.

use parking_lot::RwLock;
use serde::Serialize;

/// Readiness snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HealthSnapshot {
    /// Config has been loaded and validated.
    pub config_loaded: bool,
    /// Ledger/wallet egress path is healthy.
    pub ledger_ok: bool,
    /// Policy registry path is healthy.
    pub policy_registry_ok: bool,
    /// Internal queue/backpressure state is healthy.
    pub queue_ok: bool,
}

impl Default for HealthSnapshot {
    fn default() -> Self {
        Self {
            config_loaded: false,
            ledger_ok: true,
            policy_registry_ok: true,
            queue_ok: true,
        }
    }
}

/// Thread-safe readiness state.
#[derive(Debug, Default)]
pub struct HealthState {
    inner: RwLock<HealthSnapshot>,
}

impl HealthState {
    /// New degraded state until config_loaded is set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mutate snapshot synchronously.
    pub fn set(&self, f: impl FnOnce(&mut HealthSnapshot)) {
        let mut guard = self.inner.write();
        f(&mut guard);
    }

    /// Copy current snapshot.
    #[must_use]
    pub fn snapshot(&self) -> HealthSnapshot {
        self.inner.read().clone()
    }

    /// True when all readiness gates are green.
    #[must_use]
    pub fn all_ready(&self) -> bool {
        let s = self.inner.read();
        s.config_loaded && s.ledger_ok && s.policy_registry_ok && s.queue_ok
    }

    /// Stable missing/degraded keys.
    #[must_use]
    pub fn missing(&self) -> Vec<&'static str> {
        let s = self.inner.read();
        let mut missing = Vec::new();
        if !s.config_loaded {
            missing.push("config_loaded");
        }
        if !s.ledger_ok {
            missing.push("ledger_ok");
        }
        if !s.policy_registry_ok {
            missing.push("policy_registry_ok");
        }
        if !s.queue_ok {
            missing.push("queue_ok");
        }
        missing
    }
}
