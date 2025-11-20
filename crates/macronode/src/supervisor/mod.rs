//! RO:WHAT — Process supervisor scaffold for Macronode.
//! RO:WHY  — Central place to coordinate service startup/shutdown.
//! RO:INVARIANTS —
//!   - crash policy + backoff (future slice)
//!   - graceful shutdown orchestration (future slice)
//!   - health reporting to readiness/admin planes (future slice)

#![allow(dead_code)]

mod backoff;
mod crash_policy;
mod health_reporter;
mod lifecycle;
mod shutdown;

use std::sync::Arc;

use crate::{errors::Result, readiness::ReadyProbes, services};

pub use shutdown::ShutdownToken;

use backoff::Backoff;
use crash_policy::CrashPolicy;
use health_reporter::HealthSnapshot;
use lifecycle::LifecycleState;

/// Macronode process supervisor (MVP).
///
/// Currently minimal: only boots services. Future slices will add:
///   - crash detection + restart
///   - backoff control
///   - graceful shutdown coordination
///   - health reporting
#[derive(Debug)]
pub struct Supervisor {
    /// Shared readiness probes used by admin plane and readiness endpoints.
    probes: Arc<ReadyProbes>,
    /// Cooperative shutdown token shared with all managed services.
    shutdown: ShutdownToken,
    /// Coarse lifecycle state.
    lifecycle: LifecycleState,
    /// Aggregated view of per-service health.
    health: HealthSnapshot,
    /// Policy controlling restart aggressiveness.
    crash_policy: CrashPolicy,
    /// Exponential backoff state used between restarts.
    backoff: Backoff,
}

impl Supervisor {
    /// Construct a new supervisor handle.
    pub fn new(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> Self {
        let crash_policy = CrashPolicy::new(5, std::time::Duration::from_secs(60));
        let backoff = Backoff::new(
            std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(30),
        );

        Supervisor {
            probes,
            shutdown,
            lifecycle: LifecycleState::Starting,
            health: HealthSnapshot::default(),
            crash_policy,
            backoff,
        }
    }

    /// Start all managed services.
    pub async fn start(&self) -> Result<()> {
        services::spawn_all(self.probes.clone(), self.shutdown.clone()).await
    }

    // ------------------------------------------------------------
    //  No-op future API — does *nothing* yet.
    //  This lets future restart logic drop in without rewriting tests.
    // ------------------------------------------------------------

    /// Placeholder: record a crash event (future slice).
    pub fn record_crash(&mut self, _service: &'static str) {
        // no-op
    }

    /// Placeholder: compute restart delay (future slice).
    pub fn restart_delay(&mut self, _service: &'static str) -> Option<std::time::Duration> {
        // no-op: always allow restart with 0 delay
        Some(std::time::Duration::from_secs(0))
    }

    /// Placeholder: reset backoff state (future slice).
    pub fn reset_backoff(&mut self) {
        // no-op
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Supervisor::new(Arc::new(ReadyProbes::new()), ShutdownToken::new())
    }
}
