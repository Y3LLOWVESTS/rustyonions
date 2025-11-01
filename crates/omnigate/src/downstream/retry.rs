//! RO:WHAT   Retry policy & jittered backoff helpers for outbound HTTP.
//! RO:WHY    Make transient failures tolerable without thundering herd.
//! RO:INVARS  Budgeted attempts; exponential w/ full jitter; never retry 4xx.

use rand::{rngs::StdRng, Rng, SeedableRng};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,   // total, including first try
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(50),
            max_delay: Duration::from_millis(600),
        }
    }
}

pub fn full_jitter_backoff(attempt: u32, base: Duration, max: Duration, rng: &mut StdRng) -> Duration {
    let exp = base.saturating_mul(1u32.saturating_shl(attempt.saturating_sub(1).min(10)));
    let cap = std::cmp::min(exp, max);
    let nanos = rng.gen_range(0..=cap.as_nanos() as u128);
    Duration::from_nanos(nanos as u64)
}

pub fn new_rng() -> StdRng { StdRng::from_entropy() }
