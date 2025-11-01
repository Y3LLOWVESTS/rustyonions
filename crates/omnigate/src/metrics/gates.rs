// crates/omnigate/src/metrics/gates.rs
//! RED/Readiness gate metrics (gauges + counters)

use once_cell::sync::Lazy;
use prometheus::{
    register_gauge, register_int_counter_vec, register_int_gauge, Gauge, IntCounterVec, IntGauge,
};

// Current inflight (requests in service) as seen by the gate.
pub static READY_INFLIGHT_CURRENT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "ready_inflight_current",
        "Current in-flight requests as tracked by readiness policy"
    )
    .expect("register ready_inflight_current")
});

// Rolling error rate (0.0–100.0); store as a plain gauge.
pub static READY_ERROR_RATE_PCT: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "ready_error_rate_pct",
        "Rolling 429/503 error rate percentage over the readiness window"
    )
    .expect("register ready_error_rate_pct")
});

// Queue saturation flag (0 or 1).
pub static READY_QUEUE_SATURATED: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!("ready_queue_saturated", "Queue saturation flag (0/1)")
        .expect("register ready_queue_saturated")
});

// When ready trips to degraded, count reason.
pub static READY_TRIPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "ready_trips_total",
        "Count of readiness trips to degraded by reason",
        &["reason"] // inflight | err_rate | queue
    )
    .expect("register ready_trips_total")
});

// Count state transitions (ready -> degraded, degraded -> ready).
pub static READY_STATE_CHANGES_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "ready_state_changes_total",
        "Readiness state changes",
        &["to"] // ready | degraded
    )
    .expect("register ready_state_changes_total")
});
