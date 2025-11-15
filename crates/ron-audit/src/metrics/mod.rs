// crates/ron-audit/src/metrics/mod.rs
//! RO:WHAT  — Zero-IO metrics hook for ron-audit (library-only).
//! RO:WHY   — Let hosts publish `audit_*` metrics (Prometheus, etc.) without
//!            pulling heavy deps into the core crate.
//! RO:INTERACTS — Host services (svc-edge, svc-gateway, micronode, etc.) can
//!                install a recorder; ron-audit stays ignorant of the backend.
//!
//! Design notes:
//! - Default is a NO-OP recorder: no allocations, no locks on the hot path.
//! - This crate never depends on prometheus/tokio/axum; that’s host territory.
//! - API is intentionally tiny and stable: counter + histogram + gauge.
//! - Install is best-effort: first caller wins; later calls are ignored.
//!
//! Example (host crate):
//! ```ignore
//! use ron_audit::metrics::{install_recorder, MetricsRecorder};
//!
//! struct PromRecorder { /* wraps prometheus registry */ }
//! impl MetricsRecorder for PromRecorder {
//!     fn counter_add(&self, name: &'static str, by: u64) { /* ... */ }
//!     fn hist_ns(&self, name: &'static str, value: u64) { /* ... */ }
//!     fn gauge_set(&self, name: &'static str, value: i64) { /* ... */ }
//! }
//!
//! static PROM: PromRecorder = PromRecorder { /* ... */ };
//!
//! pub fn init_metrics() {
//!     ron_audit::metrics::install_recorder(&PROM);
//! }
//! ```
//!
//! Inside ron-audit we can call the helpers in hot paths (e.g. verify, append)
//! without knowing how they are implemented by the host.

use std::sync::OnceLock;

/// Minimal interface a host-side metrics backend must implement.
///
/// All methods must be cheap and non-blocking — they are called on the audit
/// hot path (verify/append). Any heavy lifting (aggregation, encoding, I/O)
/// should be done in the host, off the hot path.
pub trait MetricsRecorder: Send + Sync + 'static {
    /// Monotonic counter add.
    ///
    /// Example names (host suggestion):
    /// - "ron_audit_emit_total"
    /// - "ron_audit_verify_ok_total"
    /// - "ron_audit_verify_fail_total"
    fn counter_add(&self, name: &'static str, by: u64);

    /// Histogram observation in nanoseconds.
    ///
    /// Hosts may choose to export in seconds/milliseconds; the unit here is
    /// just a convention for the value we pass.
    fn hist_ns(&self, name: &'static str, value: u64);

    /// Gauge set for instantaneous values.
    ///
    /// Example names:
    /// - "ron_audit_heads_tracked"
    /// - "ron_audit_wal_queue_depth"
    fn gauge_set(&self, name: &'static str, value: i64);
}

/// NO-OP recorder used when no host has installed a real backend.
///
/// This is the default; it ensures we never panic or allocate on the hot path
/// even if nothing is wired up yet.
struct NoopRecorder;

impl MetricsRecorder for NoopRecorder {
    #[inline]
    fn counter_add(&self, _name: &'static str, _by: u64) {
        // no-op
    }

    #[inline]
    fn hist_ns(&self, _name: &'static str, _value: u64) {
        // no-op
    }

    #[inline]
    fn gauge_set(&self, _name: &'static str, _value: i64) {
        // no-op
    }
}

static NOOP_RECORDER: NoopRecorder = NoopRecorder;

/// Global recorder pointer; first install wins.
///
/// We store a `&'static dyn MetricsRecorder` so hosts can keep their own
/// statics and avoid extra allocations here.
static RECORDER: OnceLock<&'static dyn MetricsRecorder> = OnceLock::new();

#[inline]
fn recorder() -> &'static dyn MetricsRecorder {
    // If a host has installed a recorder, use it; otherwise fall back to NOOP.
    RECORDER.get().copied().unwrap_or(&NOOP_RECORDER)
}

/// Install a global metrics recorder.
///
/// This should be called once by a host crate at startup. If called multiple
/// times, only the first recorder is kept; subsequent calls are ignored.
///
/// This behavior is intentional: it avoids surprising mid-flight swaps.
pub fn install_recorder(rec: &'static dyn MetricsRecorder) {
    let _ = RECORDER.set(rec);
}

/// Increment a counter by `by`.
///
/// Thin wrapper around `MetricsRecorder::counter_add`.
#[inline]
pub fn counter_add(name: &'static str, by: u64) {
    recorder().counter_add(name, by);
}

/// Observe a latency value (nanoseconds) in a histogram.
///
/// Thin wrapper around `MetricsRecorder::hist_ns`.
#[inline]
pub fn hist_ns(name: &'static str, value: u64) {
    recorder().hist_ns(name, value);
}

/// Set a gauge to `value`.
///
/// Thin wrapper around `MetricsRecorder::gauge_set`.
#[inline]
pub fn gauge_set(name: &'static str, value: i64) {
    recorder().gauge_set(name, value);
}
