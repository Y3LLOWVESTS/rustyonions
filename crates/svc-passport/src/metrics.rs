//! RO:WHAT — Prometheus counters/histograms for passport ops.
//! RO:WHY  — Golden metrics & perf gates.
//! RO:INVARIANTS — Default registry; labels kept low-cardinality.

use once_cell::sync::Lazy;
use prometheus::{
    register_histogram, register_int_counter_vec, Histogram, HistogramOpts, IntCounterVec, Opts,
};

pub static OPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        Opts::new("passport_ops_total", "passport operations total"),
        &["op", "result", "alg"]
    )
    .unwrap()
});

pub static FAILURES_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        Opts::new("passport_failures_total", "passport failures by reason"),
        &["reason"]
    )
    .unwrap()
});

pub static OP_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        HistogramOpts::new("passport_op_latency_seconds", "op latency").buckets(vec![
            0.0002, 0.0005, 0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1
        ])
    )
    .unwrap()
});

pub static BATCH_LEN: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        HistogramOpts::new("passport_batch_len", "verify batch length")
            .buckets(vec![1.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0])
    )
    .unwrap()
});
