use std::future::Future;
use std::time::Duration;

use anyhow::Result;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Instant};
use tracing::{error, info, warn};

use crate::{Bus, Event};
use crate::cancel::Shutdown;

/// Exponential backoff with jitter (bounded).
#[derive(Debug, Clone)]
pub struct Backoff {
    pub base_ms: u64,     // default 200
    pub max_ms: u64,      // default 5000
    pub cap_pow2: u32,    // cap growth at 2^cap_pow2
    pub jitter_frac: f32, // +/- fraction (0.1 = 10%)
}

impl Default for Backoff {
    fn default() -> Self {
        Self { base_ms: 200, max_ms: 5_000, cap_pow2: 5, jitter_frac: 0.10 }
    }
}

impl Backoff {
    fn compute(&self, attempt: u32) -> Duration {
        let factor = 1u64 << attempt.min(self.cap_pow2);
        let raw = self.base_ms.saturating_mul(factor);
        let bounded = raw.min(self.max_ms);

        let jitter = (bounded as f32 * self.jitter_frac) as u64;
        let seed = Instant::now().elapsed().as_nanos() as u64;
        let jitter_val = (seed % (2 * jitter + 1)) as i64 - jitter as i64;
        let with_jitter = if jitter_val.is_negative() {
            bounded.saturating_sub(jitter_val.unsigned_abs())
        } else {
            bounded.saturating_add(jitter_val as u64)
        };
        Duration::from_millis(with_jitter.max(50))
    }
}

#[derive(Debug, Clone)]
pub struct SupervisorOptions {
    pub service_name: &'static str,
    pub backoff: Backoff,
}

impl SupervisorOptions {
    pub fn new(service_name: &'static str) -> Self {
        Self { service_name, backoff: Backoff::default() }
    }
}

/// Spawn a supervised async service. The `factory` creates a new service future each run.
/// If the task panics or returns Err, it is restarted with backoff until `shutdown` triggers.
pub fn spawn_supervised<F, Fut>(
    opts: SupervisorOptions,
    bus: Bus,
    shutdown: Shutdown,
    mut factory: F,
) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        let mut attempt: u32 = 0;

        loop {
            if shutdown.token().is_cancelled() {
                info!(service = opts.service_name, "supervisor exiting (shutdown)");
                break;
            }

            info!(service = opts.service_name, "starting service run");
            let run = tokio::spawn(factory());

            match run.await {
                Ok(Ok(())) => {
                    info!(service = opts.service_name, "service completed normally");
                    break;
                }
                Ok(Err(err)) => {
                    error!(service = opts.service_name, ?err, "service error; will restart");
                    bus.publish(Event::Restart {
                        service: opts.service_name,
                        reason: format!("error: {err:?}"),
                    });
                }
                Err(join_err) if join_err.is_panic() => {
                    error!(service = opts.service_name, ?join_err, "service panicked; will restart");
                    bus.publish(Event::Restart {
                        service: opts.service_name,
                        reason: "panic".to_string(),
                    });
                }
                Err(join_err) => {
                    error!(service = opts.service_name, ?join_err, "service join error; will restart");
                    bus.publish(Event::Restart {
                        service: opts.service_name,
                        reason: "join error".to_string(),
                    });
                }
            }

            let delay = opts.backoff.compute(attempt);
            warn!(service = opts.service_name, attempt, ?delay, "backing off before restart");
            sleep(delay).await;
            attempt = attempt.saturating_add(1);
        }
    })
}
