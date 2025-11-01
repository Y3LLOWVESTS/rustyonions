//! RO:WHAT   Prometheus registry & handles for Omnigate.
//! RO:WHY    Stable counters/histograms backing the metrics contract test.
//! RO:INTERACTS middleware::{quotas,fair_queue,body_caps,decompress_guard,policy}, admin/handlers.
pub mod gates;
pub mod registry;

use once_cell::sync::Lazy;
use prometheus::{register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterVec};

pub static HTTP_REQS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "http_requests_total",
        "Requests by route/method/status",
        &["route", "method", "status"]
    )
    .expect("register http_requests_total")
});

pub static REQUEST_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "request_latency_seconds",
        "Request latency by route/method",
        &["route", "method"]
    )
    .expect("register request_latency_seconds")
});

pub static ADMISSION_QUOTA_EXHAUSTED_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "admission_quota_exhausted_total",
        "Quota rejections by scope",
        &["scope"] // global|ip|token
    )
    .expect("register admission_quota_exhausted_total")
});

pub static FAIR_Q_EVENTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "admission_fair_queue_events_total",
        "Fair queue events by type",
        &["event"] // enqueued|dropped
    )
    .expect("register admission_fair_queue_events_total")
});

pub static BODY_REJECT_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "body_reject_total",
        "Body rejections by reason",
        &["reason"] // oversize|missing_len
    )
    .expect("register body_reject_total")
});

pub static DECOMPRESS_REJECT_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "decompress_reject_total",
        "Decompression guard rejections",
        &["reason"] // unknown|stacked
    )
    .expect("register decompress_reject_total")
});

pub static POLICY_SHORTCIRCUITS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "policy_middleware_shortcircuits_total",
        "Requests denied by policy middleware",
        &["status"] // 403|451|503
    )
    .expect("register policy_middleware_shortcircuits_total")
});
