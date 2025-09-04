//! Prometheus metrics for the bus (kept separate for clarity and testability).

#![forbid(unsafe_code)]

use std::sync::OnceLock;
use prometheus::{register_int_counter, IntCounter};

/// Global Prometheus counter for dropped events due to lag.
///
/// We do this via OnceLock so we safely initialize the metric the first time the
/// bus is constructed, and it sticks around for the process lifetime.
pub(crate) fn overflow_counter() -> &'static IntCounter {
    static C: OnceLock<IntCounter> = OnceLock::new();
    C.get_or_init(|| {
        register_int_counter!(
            "bus_overflow_dropped_total",
            "Total KernelEvent messages dropped due to subscriber lag/overflow"
        )
        .expect("register bus_overflow_dropped_total")
    })
}
