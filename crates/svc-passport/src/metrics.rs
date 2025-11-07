// crates/svc-passport/src/metrics.rs
//! RO:WHAT — Prometheus counters/histograms for passport ops + /metrics exporter.
//! RO:WHY  — Golden metrics & perf gates; scrapeable by Prometheus.

use axum::{http::StatusCode, response::IntoResponse};
use once_cell::sync::Lazy;
use prometheus::{
    gather, register_histogram, register_int_counter_vec, Encoder, Histogram, HistogramOpts,
    IntCounterVec, Opts, TextEncoder,
};

pub static OPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("passport_ops_total", "passport operations total")
        .const_label("service", "svc-passport");
    register_int_counter_vec!(opts, &["op", "result", "alg"]).expect("register passport_ops_total")
});

pub static FAILURES_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("passport_failures_total", "passport failures by reason")
        .const_label("service", "svc-passport");
    register_int_counter_vec!(opts, &["reason"]).expect("register passport_failures_total")
});

pub static OP_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    let opts = HistogramOpts {
        common_opts: Opts::new("passport_op_latency_seconds", "op latency")
            .const_label("service", "svc-passport"),
        buckets: vec![0.0002, 0.0005, 0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1],
    };
    register_histogram!(opts).expect("register passport_op_latency_seconds")
});

pub static BATCH_LEN: Lazy<Histogram> = Lazy::new(|| {
    let opts = HistogramOpts {
        common_opts: Opts::new("passport_batch_len", "verify batch length")
            .const_label("service", "svc-passport"),
        buckets: vec![1.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0],
    };
    register_histogram!(opts).expect("register passport_batch_len")
});

pub async fn export() -> impl IntoResponse {
    let metric_families = gather();
    let mut buf = Vec::with_capacity(16 * 1024);
    let encoder = TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("encode metrics: {e}"),
        )
            .into_response();
    }
    (
        StatusCode::OK,
        (
            [(axum::http::header::CONTENT_TYPE, encoder.format_type())],
            buf,
        ),
    )
        .into_response()
}
