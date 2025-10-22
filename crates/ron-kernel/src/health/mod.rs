/*!
Health state — liveness vs readiness (degrade-first).

- Uses `parking_lot::RwLock` (faster, no poisoning).
- `all_ready()` governs `/readyz` (true → 200, false → 503 + `Retry-After: 1`).
- `missing()` returns a stable list for `/readyz` body.
*/

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Snapshot of coarse-grained health used by kernel/demo routes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthSnapshot {
    /// True when essential services are running (supervisors satisfied).
    pub services_ok: bool,
    /// True when configuration load completed and is valid.
    pub config_loaded: bool,
    /// Current amnesia posture (observable; not part of readiness truth).
    pub amnesia: bool,
}

/// Mutable health state with cheap readers and exclusive writers.
#[derive(Debug, Default)]
pub struct HealthState {
    inner: RwLock<HealthSnapshot>,
}

impl HealthState {
    /// Construct a new health state wrapped in `Arc`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: RwLock::new(HealthSnapshot::default()),
        })
    }

    /// Mutate the snapshot in place.
    pub fn set(&self, f: impl FnOnce(&mut HealthSnapshot)) {
        let mut guard = self.inner.write();
        f(&mut guard);
    }

    /// Obtain a cheap cloned snapshot.
    pub fn snapshot(&self) -> HealthSnapshot {
        self.inner.read().clone()
    }

    /// Readiness policy: *both* services and config must be OK.
    pub fn all_ready(&self) -> bool {
        let s = self.inner.read();
        s.services_ok && s.config_loaded
    }

    /// Names of components that currently prevent readiness.
    pub fn missing(&self) -> Vec<String> {
        let s = self.inner.read();
        let mut out = Vec::new();
        if !s.services_ok {
            out.push("services".to_string());
        }
        if !s.config_loaded {
            out.push("config".to_string());
        }
        out
    }
}
