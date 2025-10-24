//! RO:WHAT — Host-owned metrics facade (no-op default).
//! RO:WHY  — ron-bus does not emit metrics itself; hosts increment counters
//!           in their recv loops. This file provides a tiny trait & a noop
//!           impl for hosts/tests that want a uniform interface.
//! RO:INTERACTS — Referenced by hosts; not used by Bus hot path.
//! RO:INVARIANTS — No global state; zero-alloc; zero-cost if not used.

/// Minimal metrics interface hosts may use around ron-bus operations.
///
/// Intentionally tiny; hosts are free to add richer metrics externally.
pub trait BusMetrics {
    /// Count messages published (best-effort).
    fn inc_published(&self, n: u64);

    /// Count messages received (best-effort).
    fn inc_received(&self, n: u64);

    /// Count messages dropped due to lag (best-effort).
    fn inc_lagged_drop(&self, n: u64);
}

/// A no-op metrics sink (default).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopMetrics;

impl BusMetrics for NoopMetrics {
    #[inline]
    fn inc_published(&self, _n: u64) {}

    #[inline]
    fn inc_received(&self, _n: u64) {}

    #[inline]
    fn inc_lagged_drop(&self, _n: u64) {}
}
