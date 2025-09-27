//! Golden metrics & health/readiness scaffolding.

#[derive(Debug, Default)]
pub struct Metrics;

#[derive(Debug, Default)]
pub struct HealthState;

impl Metrics {
    pub fn new() -> Self { Self::default() }
    pub fn health(&self) -> &HealthState { &HealthState::default() }
}

