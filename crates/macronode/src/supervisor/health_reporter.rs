//! RO:WHAT — Health reporting helper for Macronode supervisor.
//! RO:WHY  — Provide a small adapter object that can fan supervisor state
//!           into readiness probes or structured status maps.
//! RO:INVARIANTS —
//!   - This module is pure; it does not spawn tasks or own timers.

#![allow(dead_code)]

use std::collections::BTreeMap;

/// High-level status of a logical service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceHealth {
    Stub,
    Starting,
    Running,
    Draining,
    Failed,
}

impl ServiceHealth {
    #[must_use]
    pub const fn is_healthy(self) -> bool {
        matches!(self, ServiceHealth::Running | ServiceHealth::Draining)
    }
}

/// Aggregated view of service health used for `/api/v1/status`.
#[derive(Debug, Default)]
pub struct HealthSnapshot {
    services: BTreeMap<&'static str, ServiceHealth>,
}

impl HealthSnapshot {
    /// Record or update the health for a named service.
    pub fn set_service(&mut self, name: &'static str, health: ServiceHealth) {
        self.services.insert(name, health);
    }

    /// Immutable view of the underlying map for serialization or logging.
    #[must_use]
    pub fn services(&self) -> &BTreeMap<&'static str, ServiceHealth> {
        &self.services
    }
}
