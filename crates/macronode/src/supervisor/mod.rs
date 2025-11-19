//! RO:WHAT — Process supervisor scaffold for Macronode.
//! RO:WHY  — Central place to coordinate service startup/shutdown.
//! RO:INVARIANTS —
//!   - crash policy + backoff (future slice)
//!   - graceful shutdown orchestration (future slice)
//!   - health reporting to readiness/admin planes (future slice)

use crate::errors::Result;

/// Macronode process supervisor (MVP).
///
/// For now this is a thin wrapper that kicks off service startup. Future
/// revisions will give it:
///   - crash policy + backoff
///   - graceful shutdown orchestration
///   - health reporting to readiness/admin planes.
#[derive(Debug, Default)]
pub struct Supervisor;

impl Supervisor {
    /// Construct a new supervisor handle.
    pub fn new() -> Self {
        Supervisor
    }

    /// Start all managed services.
    ///
    /// Today this just delegates to `services::spawn_all()`; as we add more
    /// moving parts we will keep this as the single entrypoint for runtime
    /// service wiring.
    pub async fn start(&self) -> Result<()> {
        crate::services::spawn_all().await
    }
}
