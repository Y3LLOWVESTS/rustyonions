//! RO:WHAT   Small latency helpers for downstream timing (standalone).
//! RO:WHY    Keep measurement logic trivial to test/mock.

use std::time::{Duration, Instant};

pub struct Timer(Instant);

impl Timer {
    pub fn start() -> Self { Self(Instant::now()) }
    pub fn stop(self) -> Duration { self.0.elapsed() }
}
