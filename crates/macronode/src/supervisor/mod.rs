//! RO:WHAT — Process supervisor scaffold for Macronode.
//! RO:WHY  — Central place to coordinate service startup/shutdown.
//! RO:INVARIANTS —
//!   - crash policy + backoff (future slice)
//!   - graceful shutdown orchestration (future slice)
//!   - health reporting to readiness/admin planes (future slice)

mod shutdown;

use std::sync::Arc;

use crate::{errors::Result, readiness::ReadyProbes, services};

pub use shutdown::ShutdownToken;

/// Macronode process supervisor (MVP).
///
/// For now this is a thin wrapper that kicks off service startup. Future
/// revisions will give it:
///   - crash policy + backoff
///   - graceful shutdown orchestration
///   - health reporting to readiness/admin planes.
#[derive(Debug)]
pub struct Supervisor {
    probes: Arc<ReadyProbes>,
    shutdown: ShutdownToken,
}

impl Supervisor {
    /// Construct a new supervisor handle.
    pub fn new(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> Self {
        Supervisor { probes, shutdown }
    }

    /// Start all managed services.
    ///
    /// Today this just delegates to `services::spawn_all()`; as we add more
    /// moving parts we will keep this as the single entrypoint for runtime
    /// service wiring.
    pub async fn start(&self) -> Result<()> {
        services::spawn_all(self.probes.clone(), self.shutdown.clone()).await
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Supervisor {
            probes: Arc::new(ReadyProbes::new()),
            shutdown: ShutdownToken::new(),
        }
    }
}
