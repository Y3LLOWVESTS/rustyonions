//! RO:WHAT — Health & readiness handles via ron-kernel.
//! RO:WHY  — Truthful /healthz and /readyz.
//! RO:INVARIANTS — Ready only after config + keys loaded.

use ron_kernel::metrics::{health::HealthState, readiness::Readiness};
use std::sync::Arc;

#[derive(Clone)]
pub struct Health {
    pub health: Arc<HealthState>,
    pub ready: Readiness,
}

impl Health {
    pub fn new() -> Self {
        let health = Arc::new(HealthState::new());
        let ready = Readiness::new(health.clone());
        Self { health, ready }
    }
}
