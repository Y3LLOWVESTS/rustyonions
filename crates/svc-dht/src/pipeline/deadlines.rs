//! RO:WHAT — Deadline budgeting for composite operations
//! RO:WHY — Ensure hedging/fanout stays within the caller budget; Concerns: PERF/RES

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct DeadlineBudget {
    start: Instant,
    total: Duration,
}

impl DeadlineBudget {
    pub fn new(total: Duration) -> Self {
        Self { start: Instant::now(), total }
    }
    pub fn remaining(&self) -> Duration {
        let spent = self.start.elapsed();
        if spent >= self.total {
            Duration::from_millis(0)
        } else {
            self.total - spent
        }
    }
    pub fn total(&self) -> Duration {
        self.total
    }
    pub fn spent(&self) -> Duration {
        self.start.elapsed()
    }
}
