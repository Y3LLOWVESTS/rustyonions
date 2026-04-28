//! RO:WHAT — Local Prometheus metrics for svc-storage.
//! RO:WHY — Observability contract; paid storage must surface accepted/rejected write admission.
//! RO:INTERACTS — /metrics route via prometheus::gather(), paid_object handler.
//! RO:INVARIANTS — no account IDs, CIDs, receipt hashes, or private labels in metrics.
//! RO:METRICS — storage_paid_write_total{status}, storage_paid_write_bytes_total.
//! RO:CONFIG — enabled by the `metrics` feature, default-on for svc-storage.
//! RO:SECURITY — labels are low-cardinality machine statuses only.
//! RO:TEST — paid_write_policy and web3_paid_storage_loop assert metric text exposes paid-write series.

use once_cell::sync::Lazy;
use prometheus::{register_int_counter, register_int_counter_vec, IntCounter, IntCounterVec};

static PAID_WRITE_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "storage_paid_write_total",
        "Total paid storage write admissions by status.",
        &["status"]
    )
    .expect("storage_paid_write_total registration should succeed")
});

static PAID_WRITE_BYTES_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "storage_paid_write_bytes_total",
        "Total bytes stored through accepted paid storage writes."
    )
    .expect("storage_paid_write_bytes_total registration should succeed")
});

/// Record one paid-write admission result.
pub fn observe_paid_write(status: &'static str, bytes_stored: u64) {
    PAID_WRITE_TOTAL.with_label_values(&[status]).inc();

    if bytes_stored > 0 {
        PAID_WRITE_BYTES_TOTAL.inc_by(bytes_stored);
    }
}

/// Ensure paid-write metric families exist even before traffic arrives.
///
/// This makes `/metrics` useful in fresh dev runs and dashboard checks.
pub fn register_paid_write_metrics() {
    PAID_WRITE_TOTAL.with_label_values(&["accepted"]);
    PAID_WRITE_TOTAL.with_label_values(&["payment_required"]);
    PAID_WRITE_TOTAL.with_label_values(&["bad_accounting_context"]);
    PAID_WRITE_TOTAL.with_label_values(&["storage_error"]);
    PAID_WRITE_TOTAL.with_label_values(&["disabled"]);
    PAID_WRITE_TOTAL.with_label_values(&["config_error"]);
    Lazy::force(&PAID_WRITE_BYTES_TOTAL);
}
