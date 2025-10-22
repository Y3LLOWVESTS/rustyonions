//! RO:WHAT — Crash-only supervision with jittered exponential backoff and labeled restart metrics.
//! RO:WHY  — RESilience: children may crash; we restart with backoff and publish ServiceCrashed.
//! RO:INTERACTS — metrics::Metrics (service_restarts_total{service}); bus::Bus (ServiceCrashed).
//! RO:INVARIANTS — jittered backoff (100ms→30s cap), intensity cap optional; no lock across .await.

use crate::{events::KernelEvent, metrics::Metrics};
use anyhow::Result;
use rand::{rng, Rng};
use std::future::Future;
use tokio::time::{sleep, Duration};

fn jitter_ms(base: u64) -> u64 {
    if base <= 1 {
        return 1;
    }
    let mut r = rng();
    let half = base / 2;
    half + r.random_range(0..half.max(1))
}

/// Supervise an async child factory. On error, increments labeled restart metric and emits ServiceCrashed.
/// The `spawn` closure should create a fresh future each attempt.
pub async fn supervise_with_backoff<Fut, Spawn>(
    service: &str,
    metrics: Metrics,
    bus: crate::bus::Bus,
    mut spawn: Spawn,
) -> !
where
    Fut: Future<Output = Result<()>> + Send + 'static,
    Spawn: FnMut() -> Fut + Send + 'static,
{
    let service_name = service.to_string();
    let mut backoff = crate::internal::constants::SUP_BACKOFF_MS_START;

    loop {
        let res = spawn().await;
        if let Err(err) = res {
            // Publish crash and count a restart.
            let _ = bus.publish(KernelEvent::ServiceCrashed {
                service: service_name.clone(),
                reason: err.to_string(),
            });
            metrics
                .service_restarts_total
                .with_label_values(&[&service_name])
                .inc();
        }
        // Sleep with jitter (cap at 30s).
        backoff = (backoff.saturating_mul(2)).min(crate::internal::constants::SUP_BACKOFF_MS_CAP);
        let sleep_ms = jitter_ms(backoff);
        sleep(Duration::from_millis(sleep_ms)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::jitter_ms;

    #[test]
    fn jitter_is_within_bounds_and_nonzero() {
        // Basic sanity: jitter must be at least 1 and not exceed base.
        for &base in &[2, 4, 8, 16, 32, 64, 128] {
            let j = jitter_ms(base);
            assert!(j >= 1, "jitter must be >=1");
            assert!(j <= base, "jitter must be <= base (got {} for {})", j, base);
        }
        // Base 1 -> clamped to 1.
        assert_eq!(jitter_ms(1), 1);
        assert_eq!(jitter_ms(0), 1);
    }
}
