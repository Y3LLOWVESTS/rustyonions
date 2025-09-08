// crates/ron-kernel/src/bus/metrics.rs
#![forbid(unsafe_code)]
// Allow expect() only during startup-time metric construction (never in hot paths).
#![allow(clippy::expect_used)]

use std::sync::OnceLock;

use prometheus::{register, IntCounter};
use prometheus::register_int_counter;

/// Aggregated (unlabeled) bus metrics.
/// Currently not constructed by callers; kept for future ergonomic use.
#[allow(dead_code)]
#[derive(Clone)]
pub struct BusMetrics {
    /// Total KernelEvent messages dropped due to subscriber lag/overflow
    pub overflow_dropped_total: IntCounter,
}

impl BusMetrics {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let overflow_dropped_total = reg_counter(
            "bus_overflow_dropped_total",
            "Total KernelEvent messages dropped due to subscriber lag/overflow",
        );

        Self { overflow_dropped_total }
    }
}

/// Global overflow counter for the bus (unlabeled).
///
/// Call sites expect: `overflow_counter().inc_by(n);`
pub fn overflow_counter() -> &'static IntCounter {
    static OVERFLOW: OnceLock<IntCounter> = OnceLock::new();

    OVERFLOW.get_or_init(|| {
        let c = IntCounter::new(
            "ron_bus_overflow_total",
            "Number of bus messages dropped due to lagged receivers",
        )
        .expect("IntCounter::new(ron_bus_overflow_total)");
        let _ = register(Box::new(c.clone())); // ignore AlreadyRegistered
        c
    })
}

#[allow(dead_code)]
fn reg_counter(name: &'static str, help: &'static str) -> IntCounter {
    // Avoid panicking on registration: create an unregistered fallback on error.
    register_int_counter!(name, help).unwrap_or_else(|e| {
        eprintln!("prometheus: failed to register counter {name}: {e}");
        IntCounter::new(format!("{name}_fallback"), help.to_string())
            .expect("fallback IntCounter")
    })
}
