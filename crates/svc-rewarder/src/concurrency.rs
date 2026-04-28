//! RO:WHAT — Bounded concurrency primitives for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/PERF. Compute and IO must be bounded before heavy work.
//! RO:INTERACTS — http::RewarderState and future worker topology.
//! RO:INVARIANTS — semaphore bounds are non-zero; no unbounded queues.
//! RO:METRICS — queue/backpressure metrics are updated by handlers/readiness.
//! RO:CONFIG — concurrency.compute_workers, io_inflight, work_queue_max.
//! RO:SECURITY — fail-closed under overload.
//! RO:TEST — readiness/backpressure integration later.

use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::config::ConcurrencyConfig;

/// Shared concurrency gates.
#[derive(Debug, Clone)]
pub struct ConcurrencyGates {
    compute: Arc<Semaphore>,
    io: Arc<Semaphore>,
}

impl ConcurrencyGates {
    /// Build from validated config.
    #[must_use]
    pub fn new(cfg: &ConcurrencyConfig) -> Self {
        Self {
            compute: Arc::new(Semaphore::new(cfg.compute_workers.max(1))),
            io: Arc::new(Semaphore::new(cfg.io_inflight.max(1))),
        }
    }

    /// Compute semaphore.
    #[must_use]
    pub fn compute(&self) -> Arc<Semaphore> {
        Arc::clone(&self.compute)
    }

    /// IO semaphore.
    #[must_use]
    pub fn io(&self) -> Arc<Semaphore> {
        Arc::clone(&self.io)
    }
}
