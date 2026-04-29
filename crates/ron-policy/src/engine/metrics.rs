//! RO:WHAT — Prometheus metrics for policy evaluations.
//!
//! RO:INVARIANTS — register once; use default registry.

use prometheus::{
    register_histogram, register_int_counter, register_int_counter_vec, Histogram, HistogramOpts,
    IntCounter, IntCounterVec, Opts,
};

/// Lazily create or return the policy request counter.
pub fn requests_total() -> &'static IntCounter {
    static CELL: std::sync::OnceLock<IntCounter> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        register_int_counter!(Opts::new("policy_requests_total", "Policy evaluations")).unwrap()
    })
}

/// Lazily create or return the policy rejected counter vector.
pub fn rejected_total() -> &'static IntCounterVec {
    static CELL: std::sync::OnceLock<IntCounterVec> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        register_int_counter_vec!(
            Opts::new("policy_rejected_total", "Total rejects by reason"),
            &["reason"]
        )
        .unwrap()
    })
}

/// Lazily create or return the policy evaluation latency histogram.
pub fn eval_latency_seconds() -> &'static Histogram {
    static CELL: std::sync::OnceLock<Histogram> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        register_histogram!(HistogramOpts::new(
            "policy_eval_latency_seconds",
            "Evaluation latency"
        ))
        .unwrap()
    })
}
