//! RO:WHAT — Crash policy helper for the macronode supervisor.
//! RO:WHY  — Decide whether we should restart a crashing service based on
//!           how many times it has crashed within a rolling time window.
//! RO:INVARIANTS —
//!   - We only look at crashes within a sliding window (e.g. last 60s).
//!   - If the number of recent crashes exceeds `max_restarts`, we stop
//!     restarting and mark the service as permanently failed.
//!   - This module is *pure* logic: it has no side effects and can be
//!     unit-tested independently.
//!
//! RO:INTERACTS —
//!   - Intended to be used by `Supervisor` when a service task exits.
//!   - `crash_times` should be maintained by the supervisor as a log of
//!     `Instant`s when each service crashed.
//!
//! RO:NOTES —
//!   - This is groundwork: at the moment the supervisor still only spawns
//!     services once. Wiring this into the actual restart loop is a
//!     future slice so we keep behavior identical for now.

use std::time::{Duration, Instant};

/// Simple crash policy: allow up to `max_restarts` crashes within a
/// rolling `window` duration.
///
/// Example:
///   - `max_restarts = 5`
///   - `window = 60s`
///
/// If a service crashes 6+ times within the last 60 seconds, we should
/// stop trying to restart it and surface a permanent failure upstream.
#[derive(Debug, Clone, Copy)]
pub struct CrashPolicy {
    max_restarts: usize,
    window: Duration,
}

impl CrashPolicy {
    /// Construct a new crash policy.
    ///
    /// * `max_restarts` — maximum allowed crashes within `window`
    /// * `window`       — time window we consider "recent"
    pub fn new(max_restarts: usize, window: Duration) -> Self {
        CrashPolicy {
            max_restarts,
            window,
        }
    }

    /// Maximum allowed restarts within the window.
    pub fn max_restarts(&self) -> usize {
        self.max_restarts
    }

    /// Rolling window length used when counting recent crashes.
    pub fn window(&self) -> Duration {
        self.window
    }

    /// Decide whether we should attempt another restart *now* given the
    /// crash history for a service.
    ///
    /// * `crash_times` — slice of `Instant`s when this service crashed.
    ///   The supervisor owns and maintains this log.
    /// * `now`         — current time, usually `Instant::now()`.
    ///
    /// Returns `true` if we are still within the allowed restart budget
    /// for the configured window, `false` if we should stop restarting.
    pub fn should_restart(&self, crash_times: &[Instant], now: Instant) -> bool {
        // Compute the beginning of the window. On the off chance that
        // `now < window` (very early in process lifetime), we treat the
        // window start as `now` so that every crash is considered "recent"
        // and we still honor `max_restarts`.
        let window_start = now.checked_sub(self.window).unwrap_or(now);

        // Count crashes that occurred within [window_start, now].
        let recent = crash_times
            .iter()
            .copied()
            .filter(|&t| t >= window_start)
            .count();

        recent <= self.max_restarts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_restarts_below_threshold() {
        let policy = CrashPolicy::new(3, Duration::from_secs(60));
        let now = Instant::now();

        // Two crashes in the window, threshold is three.
        let crashes = vec![now - Duration::from_secs(10), now - Duration::from_secs(20)];

        assert!(policy.should_restart(&crashes, now));
    }

    #[test]
    fn denies_restarts_above_threshold() {
        let policy = CrashPolicy::new(3, Duration::from_secs(60));
        let now = Instant::now();

        // Four crashes all within the last 60 seconds.
        let crashes = vec![
            now - Duration::from_secs(5),
            now - Duration::from_secs(10),
            now - Duration::from_secs(20),
            now - Duration::from_secs(30),
        ];

        assert!(!policy.should_restart(&crashes, now));
    }

    #[test]
    fn ignores_crashes_outside_window() {
        let policy = CrashPolicy::new(2, Duration::from_secs(30));
        let now = Instant::now();

        // One very old crash (outside window), two recent ones.
        let crashes = vec![
            now - Duration::from_secs(300),
            now - Duration::from_secs(5),
            now - Duration::from_secs(10),
        ];

        // Only the two recent crashes should count → still within budget.
        assert!(policy.should_restart(&crashes, now));
    }
}
