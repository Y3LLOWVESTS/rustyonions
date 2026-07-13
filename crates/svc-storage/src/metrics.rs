//! RO:WHAT — Local Prometheus metrics for svc-storage.
//! RO:WHY — Observability contract for paid writes, accounting export, and moderation refusals.
//! RO:INTERACTS — /metrics, paid_object, accounting exporter, object-read moderation gates.
//! RO:INVARIANTS — no account IDs, CIDs, paths, IPs, receipt hashes, or private labels.
//! RO:METRICS — storage_paid_write_total, storage_accounting_export_total, storage_moderation_refusals_total.
//! RO:CONFIG — enabled by the `metrics` feature, default-on for svc-storage.
//! RO:SECURITY — labels are closed, low-cardinality machine statuses only.
//! RO:TEST — paid_write_policy, paid_write_accounting_export, moderation_observability.

use once_cell::sync::Lazy;
use prometheus::{register_int_counter, register_int_counter_vec, IntCounter, IntCounterVec};
use ron_policy::ModerationReasonCode;

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

static ACCOUNTING_EXPORT_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "storage_accounting_export_total",
        "Total paid-storage usage-event accounting exports by status.",
        &["status"]
    )
    .expect("storage_accounting_export_total registration should succeed")
});

static ACCOUNTING_EXPORT_EVENTS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "storage_accounting_export_events_total",
        "Total usage events successfully exported by svc-storage."
    )
    .expect("storage_accounting_export_events_total registration should succeed")
});

static MODERATION_REFUSALS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "storage_moderation_refusals_total",
        "Total object-read refusals by bounded route and moderation reason.",
        &["route", "reason"]
    )
    .expect("storage_moderation_refusals_total registration should succeed")
});

const MODERATION_ROUTES: [&str; 3] = ["oap_obj_get", "legacy_get", "legacy_head"];

const MODERATION_REFUSAL_REASONS: [ModerationReasonCode; 4] = [
    ModerationReasonCode::GlobalDeny,
    ModerationReasonCode::OwnerTombstone,
    ModerationReasonCode::LocalBlock,
    ModerationReasonCode::Quarantined,
];

/// Record one paid-write admission result.
pub fn observe_paid_write(status: &'static str, bytes_stored: u64) {
    PAID_WRITE_TOTAL.with_label_values(&[status]).inc();

    if bytes_stored > 0 {
        PAID_WRITE_BYTES_TOTAL.inc_by(bytes_stored);
    }
}

/// Record one accounting export result.
pub fn observe_accounting_export(status: &'static str, event_count: u64) {
    ACCOUNTING_EXPORT_TOTAL.with_label_values(&[status]).inc();

    if status == "exported" && event_count > 0 {
        ACCOUNTING_EXPORT_EVENTS_TOTAL.inc_by(event_count);
    }
}

/// Record one moderation refusal using bounded route and core-owned reason labels.
pub(crate) fn observe_moderation_refusal(route: &'static str, reason: ModerationReasonCode) {
    if let Some(reason) = reason.refusal_label() {
        MODERATION_REFUSALS_TOTAL
            .with_label_values(&[route, reason])
            .inc();
    }
}

/// Ensure moderation series exist before traffic arrives.
pub(crate) fn register_moderation_metrics() {
    for route in MODERATION_ROUTES {
        for reason in MODERATION_REFUSAL_REASONS {
            if let Some(reason) = reason.refusal_label() {
                MODERATION_REFUSALS_TOTAL.with_label_values(&[route, reason]);
            }
        }
    }
}

/// Ensure paid-write and accounting-export metric families exist before traffic arrives.
///
/// The `/metrics` route calls this so fresh dev runs and dashboards show the
/// expected low-cardinality series even before the first paid write/export.
pub fn register_paid_write_metrics() {
    PAID_WRITE_TOTAL.with_label_values(&["accepted"]);
    PAID_WRITE_TOTAL.with_label_values(&["payment_required"]);
    PAID_WRITE_TOTAL.with_label_values(&["bad_accounting_context"]);
    PAID_WRITE_TOTAL.with_label_values(&["storage_error"]);
    PAID_WRITE_TOTAL.with_label_values(&["disabled"]);
    PAID_WRITE_TOTAL.with_label_values(&["config_error"]);
    PAID_WRITE_TOTAL.with_label_values(&["settlement_error"]);

    ACCOUNTING_EXPORT_TOTAL.with_label_values(&["skipped"]);
    ACCOUNTING_EXPORT_TOTAL.with_label_values(&["exported"]);
    ACCOUNTING_EXPORT_TOTAL.with_label_values(&["failed"]);

    Lazy::force(&PAID_WRITE_BYTES_TOTAL);
    Lazy::force(&ACCOUNTING_EXPORT_EVENTS_TOTAL);
}
