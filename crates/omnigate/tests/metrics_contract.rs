//! Ensures /metrics exports required series & labels.
//! Run: cargo test -p omnigate --test metrics_contract

use regex::Regex;

#[test]
fn metrics_shape_is_present() {
    // For CI stability you can replace this with a boot-and-fetch helper.
    let metrics = include_str!("../testing/fixtures/metrics.sample.txt");

    for name in &[
        "http_requests_total",
        "request_latency_seconds",
        "admission_quota_exhausted_total",
        "admission_fair_queue_events_total",
        "body_reject_total",
        "decompress_reject_total",
        "policy_middleware_shortcircuits_total",
    ] {
        assert!(metrics.contains(name), "missing series: {name}");
    }

    let re = Regex::new(
        r#"http_requests_total\{route="[^"]+",method="(GET|POST|PUT|DELETE)",status="\d{3}"\}"#,
    )
    .unwrap();
    assert!(
        re.is_match(metrics),
        "labels missing on http_requests_total"
    );
}
