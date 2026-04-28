//! RO:WHAT — Readiness gate and snapshot for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: RES/ECON/GOV. Wallet writes must fail closed before dependency collapse.
//! RO:INTERACTS — routes/health, routes/metrics, middleware::shedder, supervisor/main.
//! RO:INVARIANTS — /healthz is liveness; /readyz is dependency truth; degraded writes shed before unsafe commits.
//! RO:METRICS — metrics.rs renders wallet_ready from this gate.
//! RO:CONFIG — no direct config reads; startup decides initial dependency state.
//! RO:SECURITY — no secrets or account data in readiness body.
//! RO:TEST — default_is_not_ready; mark_ready_sets_all_core_deps.

use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Serializable readiness snapshot for /readyz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessSnapshot {
    /// True when the service should accept reads/writes.
    pub ready: bool,
    /// Ledger dependency health.
    pub ledger_ok: bool,
    /// Policy dependency health.
    pub policy_loaded: bool,
    /// Accounting sink health.
    pub accounting_ok: bool,
    /// Whether write paths should be shed.
    pub shed_writes: bool,
    /// Human-readable missing dependency labels.
    pub missing: Vec<String>,
}

impl Default for ReadinessSnapshot {
    fn default() -> Self {
        Self {
            ready: false,
            ledger_ok: false,
            policy_loaded: false,
            accounting_ok: false,
            shed_writes: true,
            missing: vec![
                "ledger".to_string(),
                "policy".to_string(),
                "accounting".to_string(),
            ],
        }
    }
}

/// Cloneable readiness gate.
#[derive(Clone, Debug, Default)]
pub struct ReadinessGate {
    inner: Arc<RwLock<ReadinessSnapshot>>,
}

impl ReadinessGate {
    /// Build a new degraded gate.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return current readiness snapshot.
    pub fn snapshot(&self) -> ReadinessSnapshot {
        self.inner.read().clone()
    }

    /// True when service is ready.
    pub fn is_ready(&self) -> bool {
        self.snapshot().ready
    }

    /// Mark dev/local dependencies as ready.
    pub fn mark_ready(&self) {
        let mut guard = self.inner.write();
        guard.ledger_ok = true;
        guard.policy_loaded = true;
        guard.accounting_ok = true;
        guard.shed_writes = false;
        recompute_missing(&mut guard);
    }

    /// Mark the service degraded with a named missing dependency.
    pub fn mark_degraded(&self, missing: impl Into<String>) {
        let mut guard = self.inner.write();
        guard.ready = false;
        guard.shed_writes = true;
        let missing = missing.into();
        if !guard.missing.iter().any(|item| item == &missing) {
            guard.missing.push(missing);
        }
    }
}

fn recompute_missing(snapshot: &mut ReadinessSnapshot) {
    snapshot.missing.clear();
    if !snapshot.ledger_ok {
        snapshot.missing.push("ledger".to_string());
    }
    if !snapshot.policy_loaded {
        snapshot.missing.push("policy".to_string());
    }
    if !snapshot.accounting_ok {
        snapshot.missing.push("accounting".to_string());
    }
    snapshot.ready = snapshot.missing.is_empty() && !snapshot.shed_writes;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_ready() {
        let gate = ReadinessGate::new();
        assert!(!gate.is_ready());
        assert!(gate.snapshot().shed_writes);
    }

    #[test]
    fn mark_ready_sets_all_core_deps() {
        let gate = ReadinessGate::new();
        gate.mark_ready();
        let snapshot = gate.snapshot();
        assert!(snapshot.ready);
        assert!(snapshot.missing.is_empty());
    }
}
