//! RO:WHAT — Supervisor wrapper that restarts an async actor on failure.
//! RO:WHY  — Crash-only philosophy; Concerns: RES; host observes restarts.
//! RO:INTERACTS — config::SupervisionCfg (base/cap/jitter); tracing (optional).
//! RO:INVARIANTS — backoff grows to cap; cancellation cooperative.

use crate::config::RykerConfig;
use crate::supervisor::decorrelated_jitter;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

pub struct Supervisor {
    cfg: Arc<RykerConfig>,
}

impl Supervisor {
    pub fn new(cfg: Arc<RykerConfig>) -> Self {
        Self { cfg }
    }

    /// Spawn an actor future; if it returns Err or panics, it will be restarted
    /// with decorrelated jitter until cancel() is observed by the actor.
    pub fn spawn<F, Fut>(&self, mut make_actor: F) -> JoinHandle<()>
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let cfg = self.cfg.clone();
        tokio::spawn(async move {
            let mut backoff = cfg.supervisor.backoff_base_ms;
            let mut last_fail = Instant::now();
            loop {
                let res = make_actor().await;
                if res.is_ok() {
                    // Normal exit—do not restart.
                    break;
                }
                #[cfg(feature = "tracing")]
                tracing::warn!(target = "ryker", "actor failed; restarting");

                // Compute next delay with decorrelated jitter.
                let seed = last_fail.elapsed().as_millis() as u64;
                backoff = decorrelated_jitter(
                    cfg.supervisor.backoff_base_ms,
                    cfg.supervisor.backoff_cap_ms,
                    backoff,
                    seed,
                );
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                last_fail = Instant::now();
            }
        })
    }
}
