//! RO:WHAT — Tiny, optional queue depth heuristic for hosts/tests.
//! RO:WHY  — Tokio broadcast does not expose depth/len; we can maintain a
//!           conservative estimate from observed lag and publishes to help
//!           hosts make decisions (e.g., increase capacity, cut over).
//! RO:INTERACTS — Used by host loops/tests; not required by Bus hot path.
//! RO:INVARIANTS — Lock-free from the API perspective; no `.await` here.
//! RO:SECURITY — No secrets/PII; pure counters.
//! RO:TEST — Covered indirectly in integration benches/tests as needed.

use core::sync::atomic::{AtomicU64, Ordering};

/// A conservative queue depth heuristic derived from observed lag/drop.
///
/// This is **purely optional** and **not** wired into the Bus hot path.
/// Hosts may instantiate and update it from their recv loop whenever they
/// observe `RecvError::Lagged(n)` or after batches of publishes.
#[derive(Debug, Default)]
pub struct DepthEstimator {
    /// Count of published messages we tracked (monotonic).
    pub_published: AtomicU64,
    /// Sum of observed lag events (messages skipped by a receiver).
    pub_lagged_sum: AtomicU64,
}

impl DepthEstimator {
    /// Create a new estimator.
    pub const fn new() -> Self {
        Self {
            pub_published: AtomicU64::new(0),
            pub_lagged_sum: AtomicU64::new(0),
        }
    }

    /// Record that `n` messages were published (best effort).
    #[inline]
    pub fn on_published(&self, n: u64) {
        if n != 0 {
            self.pub_published.fetch_add(n, Ordering::Relaxed);
        }
    }

    /// Record that a receiver observed `lagged` dropped messages.
    #[inline]
    pub fn on_lagged(&self, lagged: u64) {
        if lagged != 0 {
            self.pub_lagged_sum.fetch_add(lagged, Ordering::Relaxed);
        }
    }

    /// Snapshot a conservative estimate.
    ///
    /// Not a true queue length — broadcast fanout and differing subscriber
    /// speeds mean there is no single “depth” — but this gives hosts a stable
    /// scalar to alert on (e.g., > X lagged per second).
    #[inline]
    pub fn snapshot(&self) -> DepthSnapshot {
        DepthSnapshot {
            published: self.pub_published.load(Ordering::Relaxed),
            lagged_sum: self.pub_lagged_sum.load(Ordering::Relaxed),
        }
    }

    /// Reset counters (e.g., at the end of a reporting interval).
    #[inline]
    pub fn reset(&self) {
        self.pub_published.store(0, Ordering::Relaxed);
        self.pub_lagged_sum.store(0, Ordering::Relaxed);
    }
}

/// Point-in-time heuristic values.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DepthSnapshot {
    pub published: u64,
    pub lagged_sum: u64,
}

impl DepthSnapshot {
    /// Returns an “estimated pressure” scalar suitable for alerting.
    ///
    /// Right now this is a simple passthrough of `lagged_sum`. Hosts may
    /// choose to apply a moving average or rate conversion externally.
    #[inline]
    pub fn pressure(self) -> u64 {
        self.lagged_sum
    }
}
