// crates/omnigate/src/metrics/gates.rs
//! RED/Readiness gate metrics (gauges + counters)
//!
//! Also hosts gate-level guard counters used by policy/body/decompress/quotas middleware
//! so everything exports from the *default* Prometheus registry. The API exposes
//! this registry via /ops/metrics.

use once_cell::sync::Lazy;
use prometheus::{
    register_gauge, register_int_counter_vec, register_int_gauge, Gauge, IntCounterVec, IntGauge,
};

// ==============================
// Readiness
// ==============================

pub static READY_INFLIGHT_CURRENT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "ready_inflight_current",
        "Current in-flight requests as tracked by readiness policy"
    )
    .expect("register ready_inflight_current")
});

pub static READY_ERROR_RATE_PCT: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "ready_error_rate_pct",
        "Rolling 429/503 error rate percentage over the readiness window"
    )
    .expect("register ready_error_rate_pct")
});

pub static READY_QUEUE_SATURATED: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!("ready_queue_saturated", "Queue saturation flag (0/1)")
        .expect("register ready_queue_saturated")
});

pub static READY_TRIPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "ready_trips_total",
        "Count of readiness trips to degraded by reason",
        &["reason"] // inflight | err_rate | queue
    )
    .expect("register ready_trips_total")
});

pub static READY_STATE_CHANGES_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "ready_state_changes_total",
        "Readiness state changes",
        &["to"] // ready | degraded
    )
    .expect("register ready_state_changes_total")
});

// ==============================
// Gate guards (counters)
// ==============================

/// Policy middleware short-circuits (deny/legal/error).
/// Label `code`: "403" | "451" | "503".
pub static POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "policy_middleware_shortcircuits_total",
        "Policy middleware short-circuited the request",
        &["code"]
    )
    .expect("register policy_middleware_shortcircuits_total")
});

/// Body admission rejections. reason: "oversize" | "missing_length".
pub static BODY_REJECT_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "body_reject_total",
        "Body preflight or body-limit rejected the request",
        &["reason"]
    )
    .expect("register body_reject_total")
});

/// Decompression guard rejections. reason: "stacked" | "unknown" | "over_budget".
pub static DECOMPRESS_REJECT_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "decompress_reject_total",
        "Decompression guard rejected the request",
        &["reason"]
    )
    .expect("register decompress_reject_total")
});

/// Quota rejections (global or per-IP).
/// Labels:
///   - scope: "global" | "ip"
///   - reason: "qps" (MVP; future may distinguish "burst" etc.)
pub static QUOTA_REJECT_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quota_reject_total",
        "Quota guard rejected the request",
        &["scope", "reason"]
    )
    .expect("register quota_reject_total")
});

/// Force-initialize so the series exist before the first scrape.
pub fn init_gate_metrics() {
    let _ = &*POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL;
    let _ = &*BODY_REJECT_TOTAL;
    let _ = &*DECOMPRESS_REJECT_TOTAL;
    let _ = &*QUOTA_REJECT_TOTAL;

    let _ = &*READY_INFLIGHT_CURRENT;
    let _ = &*READY_ERROR_RATE_PCT;
    let _ = &*READY_QUEUE_SATURATED;
    let _ = &*READY_TRIPS_TOTAL;
    let _ = &*READY_STATE_CHANGES_TOTAL;
}
