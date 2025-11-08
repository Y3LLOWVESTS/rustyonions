// RO:WHAT  Limits enforcement tests: max_msg_bytes and max_batch.
// RO:WHY   Beta acceptance requires tight negative-path behavior.

use axum::body::{to_bytes, Body};
use axum::{http, http::Request};
use tower::ServiceExt;

use serde_json::json;

use svc_passport::{health::Health, http::router::build_router};

#[path = "../src/test_support.rs"]
mod test_support;
use test_support::default_config;

#[tokio::test]
async fn issue_oversize_returns_413() {
    // Override max_msg_bytes small to force 413.
    let mut cfg = default_config();
    cfg.limits.max_msg_bytes = 64; // bytes

    let app = build_router(cfg, Health::default());

    // Create a JSON body >64 bytes
    let big = "x".repeat(80);
    let body = Body::from(serde_json::to_vec(&json!({ "data": big })).unwrap());

    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/issue")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), http::StatusCode::PAYLOAD_TOO_LARGE);

    let buf = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let txt = String::from_utf8_lossy(&buf);
    assert!(txt.contains("MsgTooLarge"));
}

#[tokio::test]
async fn verify_batch_over_limit_returns_413() {
    // limits.max_batch = 4
    let mut cfg = default_config();
    cfg.limits.max_batch = 4;

    let app = build_router(cfg, Health::default());

    // Build 5 envelopes to trip the limit.
    let mut items = vec![];
    for _ in 0..5 {
        items.push(json!({
            "alg": "Ed25519",
            "kid": "K",
            "msg_b64": "eA==",  // "x"
            "sig_b64": "eA==",  // dummy; will fail at verifier, but the limit should trigger first
            "aud": "svc-passport"
        }));
    }

    let body = Body::from(serde_json::to_vec(&json!({ "items": items })).unwrap());

    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/verify_batch")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), http::StatusCode::PAYLOAD_TOO_LARGE);
    let buf = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let txt = String::from_utf8_lossy(&buf);
    assert!(txt.contains("BatchTooLarge"));
}
