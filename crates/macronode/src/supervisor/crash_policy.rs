//! RO:WHAT — Crash policy helper for Macronode supervisor.
//! RO:WHY  — Provide a single place to decide whether a given crash history
//!           should result in another restart or giving up on a worker.
//! RO:INVARIANTS —
//!   - Decisions are pure and deterministic.
//!   - The policy never panics on empty or small histories.

#![allow(dead_code)]

use std::time::{Duration, Instant};

/// Result of asking the policy what to do about recent crashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashDecision {
    /// Supervisor should attempt another restart after the given delay.
    RestartAfter(Duration),
    /// Supervisor should stop restarting this worker.
    GiveUp,
}

/// Crash policy based on "max restarts in a rolling window".
#[derive(Debug, Clone, Copy)]
pub struct CrashPolicy {
    /// Maximum number of restarts allowed within `window` before giving up.
    pub max_restarts: usize,
    /// Rolling time window to consider.
    pub window: Duration,
}

impl CrashPolicy {
    /// Construct a policy with the given limits.
    #[must_use]
    pub const fn new(max_restarts: usize, window: Duration) -> Self {
        Self {
            max_restarts,
            window,
        }
    }

    /// Decide what to do given the timestamps of recent crashes, ordered
    /// from oldest to newest.
    ///
    /// The caller is responsible for trimming the history occasionally;
    /// this function just consumes the slice it is given.
    #[must_use]
    pub fn decide(
        &self,
        now: Instant,
        crash_times: &[Instant],
        backoff_delay: Duration,
    ) -> CrashDecision {
        if crash_times.is_empty() {
            return CrashDecision::RestartAfter(backoff_delay);
        }

        // Count how many crashes are within the rolling window.
        let window_start = now.saturating_duration_since(self.window);
        let recent = crash_times
            .iter()
            .filter(|&&t| t >= window_start)
            .count();

        if recent > self.max_restarts {
            CrashDecision::GiveUp
        } else {
            CrashDecision::RestartAfter(backoff_delay)
        }
    }
}
