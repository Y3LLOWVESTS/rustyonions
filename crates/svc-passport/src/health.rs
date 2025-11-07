//! Health/Readiness wrappers.

use ron_kernel::metrics::{health::HealthState, readiness::Readiness};

#[derive(Clone)]
pub struct Health {
    pub health: HealthState,
    pub ready: Readiness,
}

impl Health {
    pub fn new() -> Self {
        let health = HealthState::new();
        let ready = Readiness::new(health.clone());
        Self { health, ready }
    }
}

// Clippy asked for this; also makes it easy to construct.
impl Default for Health {
    fn default() -> Self {
        Self::new()
    }
}
