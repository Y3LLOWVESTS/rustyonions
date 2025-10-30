//! RO:WHAT — Prometheus metrics for policy evaluations.
//!
//! RO:INVARIANTS — register once; use default registry.

use prometheus::{
    register_histogram, register_int_counter, register_int_counter_vec, Histogram, HistogramOpts,
    IntCounter, IntCounterVec, Opts,
};

pub static REQUESTS_TOTAL: std::sync::LazyLock<IntCounter> = std::sync::LazyLock::new(|| {
    register_int_counter!(Opts::new("policy_requests_total", "Policy evaluations")).unwrap()
});

pub static REJECTED_TOTAL: std::sync::LazyLock<IntCounterVec> = std::sync::LazyLock::new(|| {
    register_int_counter_vec!(
        Opts::new("policy_rejected_total", "Total rejects by reason"),
        &["reason"]
    )
    .unwrap()
});

pub static EVAL_LATENCY_SECONDS: std::sync::LazyLock<Histogram> = std::sync::LazyLock::new(|| {
    register_histogram!(HistogramOpts::new(
        "policy_eval_latency_seconds",
        "Evaluation latency"
    ))
    .unwrap()
});
