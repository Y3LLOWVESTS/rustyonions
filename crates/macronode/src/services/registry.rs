//! RO:WHAT — In-memory registry of Macronode managed services.
//! RO:WHY  — Give `/api/v1/status` and the supervisor a shared place to
//!           track which services exist and their coarse health.
//! RO:INVARIANTS —
//!   - Registry is in-memory only; macronode owns no persistent data.
//!   - Service names are small static strings; no user input here.

#![allow(dead_code)]

use std::collections::BTreeMap;

/// Coarse status for a composed service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Present but not yet doing real work (today's stubs).
    Stub,
    /// In the process of starting up.
    Starting,
    /// Fully running and healthy.
    Running,
    /// In the process of draining for shutdown.
    Draining,
    /// Failed permanently; requires operator intervention.
    Failed,
}

/// Simple registry mapping service name → status.
#[derive(Debug, Default)]
pub struct ServiceRegistry {
    inner: BTreeMap<&'static str, ServiceStatus>,
}

impl ServiceRegistry {
    /// Update the status for a named service.
    pub fn set_status(&mut self, name: &'static str, status: ServiceStatus) {
        self.inner.insert(name, status);
    }

    /// Fetch the status for a named service, if known.
    #[must_use]
    pub fn get_status(&self, name: &'static str) -> Option<ServiceStatus> {
        self.inner.get(name).copied()
    }

    /// Take a snapshot of all service statuses for serialization or logging.
    #[must_use]
    pub fn snapshot(&self) -> BTreeMap<&'static str, ServiceStatus> {
        self.inner.clone()
    }
}
