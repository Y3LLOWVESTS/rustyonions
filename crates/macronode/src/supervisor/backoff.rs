//! RO:WHAT — Exponential backoff helper for service restarts.
//! RO:WHY  — Give the supervisor a small, testable primitive to decide how
//!           long to wait before restarting a crashed worker.
//! RO:INVARIANTS —
//!   - Backoff never panics; it clamps at `max_delay`.
//!   - All math is done with safe, bounded integers.

#![allow(dead_code)]

use std::time::Duration;

/// Simple exponential backoff policy.
///
/// This is intentionally small; more elaborate jitter/strategy can be added
/// later without changing call sites.
#[derive(Debug, Clone)]
pub struct Backoff {
    base_delay: Duration,
    max_delay: Duration,
    attempt: u32,
}

impl Backoff {
    /// Construct a new backoff policy with the given base and max delay.
    #[must_use]
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            base_delay,
            max_delay,
            attempt: 0,
        }
    }

    /// Reset the attempt counter back to zero.
    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    /// Compute the next delay.
    ///
    /// Roughly: `delay = base_delay * 2^attempt`, clamped at `max_delay`.
    /// The attempt counter is incremented after each call.
    #[must_use]
    pub fn next_delay(&mut self) -> Duration {
        // 2^attempt as a u32, clamped so we never shift by >= 32.
        let exp = self.attempt.min(31);
        let factor: u32 = 1u32.checked_shl(exp).unwrap_or(u32::MAX);

        let candidate = self.base_delay.saturating_mul(factor);
        self.attempt = self.attempt.saturating_add(1);

        if candidate > self.max_delay {
            self.max_delay
        } else {
            candidate
        }
    }
}
