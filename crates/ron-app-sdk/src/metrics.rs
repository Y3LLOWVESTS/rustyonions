//! RO:WHAT — Minimal metrics facade for SDK operations.
//! RO:WHY  — Give hosts a single, stable trait they can implement using
//!           Prometheus, OpenTelemetry, or their own metrics stack.
//! RO:INTERACTS — Intended to be threaded through planes (storage/edge/
//!                mailbox/index) and caches; default impl is no-op.
//! RO:INVARIANTS —
//!   - No global state; host controls concrete implementation.
//!   - No dependency on any metrics crate (Prometheus/Otel lives outside).
//!   - Low-cardinality labels: endpoints should be path-like, not per-ID.
//! RO:METRICS — Shape only; concrete counters/histograms are defined by hosts.
//! RO:CONFIG — Typically driven by `TracingCfg.metrics` and host config.
//! RO:SECURITY — Callers should avoid using PII-heavy label values.
//! RO:TEST — Basic unit tests for the no-op implementation.

/// High-level metrics trait for SDK operations.
///
/// Host applications can implement this trait using whatever metrics
/// framework they prefer. The SDK only cares about the *shape* of the
/// metrics, not how they are exported.
pub trait SdkMetrics: Send + Sync + 'static {
    /// Observe latency of a single SDK call (in milliseconds).
    ///
    /// `endpoint` should be a low-cardinality identifier such as a path
    /// (`/storage/put`) or logical name (`storage_put`).
    fn observe_latency(&self, endpoint: &str, success: bool, latency_ms: u64);

    /// Increment the retry counter for a given endpoint.
    fn inc_retry(&self, endpoint: &str);

    /// Increment a failure counter tagged with a coarse reason.
    fn inc_failure(&self, endpoint: &str, reason: &'static str);

    /// Count cache hits for SDK-level caches (if used).
    fn inc_cache_hit(&self, endpoint: &str);

    /// Count cache misses for SDK-level caches (if used).
    fn inc_cache_miss(&self, endpoint: &str);
}

/// No-op metrics implementation (default).
///
/// This is useful for tests, examples, and hosts that do not care about
/// metrics. All methods are intentionally zero-cost.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopSdkMetrics;

impl SdkMetrics for NoopSdkMetrics {
    #[inline]
    fn observe_latency(&self, _endpoint: &str, _success: bool, _latency_ms: u64) {
        // no-op
    }

    #[inline]
    fn inc_retry(&self, _endpoint: &str) {
        // no-op
    }

    #[inline]
    fn inc_failure(&self, _endpoint: &str, _reason: &'static str) {
        // no-op
    }

    #[inline]
    fn inc_cache_hit(&self, _endpoint: &str) {
        // no-op
    }

    #[inline]
    fn inc_cache_miss(&self, _endpoint: &str) {
        // no-op
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_is_send_sync_static() {
        fn assert_bounds<T: SdkMetrics>() {}
        assert_bounds::<NoopSdkMetrics>();
    }
}
