//! RO:WHAT — Jittered exponential backoff with cap and reset.
//! RO:WHY  — Prevents thundering herds on crash-loops; RES concern.
//! RO:INTERACTS — lifecycle.rs (sleep scheduling), child.rs (restart cadence).
//! RO:INVARIANTS — Monotone until cap; jitter bounded; no panics on edge cases.
//! RO:METRICS/LOGS — None (observed externally by restart counters).
//! RO:CONFIG — init/max/factor/jitter ranges validated by config layer.
//! RO:SECURITY — N/A.
//! RO:TEST HOOKS — unit: sequence grows; jitter within bounds; reset works.

use rand::{rng, Rng};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Backoff {
    current: Duration,
    init: Duration,
    max: Duration,
    factor: f64,
    jitter: f64, // 0.2 => ±20%
}

impl Backoff {
    pub fn new(init: Duration, max: Duration, factor: f64, jitter: f64) -> Self {
        let init = if init.is_zero() { Duration::from_millis(100) } else { init };
        let max = if max < init { init } else { max };
        let factor = if factor < 1.0 { 1.0 } else { factor };
        let jitter = jitter.clamp(0.0, 1.0);
        Self {
            current: init,
            init,
            max,
            factor,
            jitter,
        }
    }

    pub fn next(&mut self) -> Duration {
        let base = self.current;
        // prepare next (monotone up to cap)
        let next = Duration::from_secs_f64((base.as_secs_f64() * self.factor).min(self.max.as_secs_f64()));
        self.current = next;

        // apply jitter to current sleep (the 'base' value)
        if self.jitter == 0.0 {
            return base;
        }
        let j = rng().random_range(-self.jitter..=self.jitter);
        let secs = base.as_secs_f64() * (1.0 + j);
        Duration::from_secs_f64(secs.clamp(self.init.as_secs_f64(), self.max.as_secs_f64()))
    }

    pub fn reset(&mut self) {
        self.current = self.init;
    }
}
