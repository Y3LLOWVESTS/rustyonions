#![forbid(unsafe_code)]

use prometheus::{IntCounterVec, GaugeVec, register_int_counter_vec, register_gauge_vec};

pub fn restarts_metric() -> IntCounterVec {
    register_int_counter_vec!(
        "supervisor_restarts_total",
        "Total number of restarts performed by the supervisor",
        &["service"]
    ).expect("register supervisor_restarts_total")
}

pub fn backoff_metric() -> GaugeVec {
    register_gauge_vec!(
        "supervisor_backoff_seconds",
        "Current backoff delay before restarting a service",
        &["service"]
    ).expect("register supervisor_backoff_seconds")
}
