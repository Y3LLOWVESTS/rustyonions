//! Retry and backoff helpers for ron-app-sdk.
//!
//! RO:WHAT — Exponential backoff schedule helpers driven by `RetryCfg`.
//! RO:WHY  — Central place for retry math so all planes behave consistently.
//! RO:INTERACTS — Used by edge/storage/mailbox/index planes and examples.
//! RO:INVARIANTS — Pure functions; no I/O; no sleeping; jitter kept within
//!                 bounds (currently deterministic, ready for real jitter).

use std::time::Duration;

use crate::config::{Jitter, RetryCfg};

/// Compute the base (non-jittered) delay for a given attempt index.
///
/// `attempt` is zero-based: 0 → first retry, 1 → second, etc.
pub fn base_delay(cfg: &RetryCfg, attempt: u32) -> Duration {
    if attempt == 0 {
        return cfg.base;
    }

    let factor = cfg.factor.max(1.0) as f64;
    let base_ms = cfg.base.as_millis() as f64;
    let pow = factor.powi(attempt as i32);

    let mut ms = (base_ms * pow).round();
    let cap_ms = cfg.cap.as_millis() as f64;

    if ms > cap_ms {
        ms = cap_ms;
    }

    Duration::from_millis(ms as u64)
}

/// Apply jitter to a base delay.
///
/// For now `Jitter::Full` does not introduce randomness yet; it simply
/// returns the base delay. This keeps the implementation deterministic.
/// In a later revision we can introduce true full jitter using `rand`
/// once the dependency is wired and property tests are in place.
pub fn apply_jitter(base: Duration, jitter: Jitter) -> Duration {
    match jitter {
        Jitter::None => base,
        Jitter::Full => base,
    }
}

/// Iterator over retry delays according to the given configuration.
///
/// This does **not** sleep; callers are responsible for awaiting between
/// iterations.
pub fn backoff_schedule<'a>(cfg: &'a RetryCfg) -> impl Iterator<Item = Duration> + 'a {
    (0..cfg.max_attempts).map(move |attempt| {
        let base = base_delay(cfg, attempt);
        apply_jitter(base, cfg.jitter)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_is_monotonic_and_capped() {
        let cfg = RetryCfg::default();
        let mut last = Duration::ZERO;
        for d in backoff_schedule(&cfg) {
            assert!(d >= last);
            assert!(d <= cfg.cap);
            last = d;
        }
    }
}
