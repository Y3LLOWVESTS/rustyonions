// RO:WHAT  Audience and algorithm validation tests.
// RO:WHY   Enforce require_aud=true and Ed25519-only for Beta.

use axum::body::{to_bytes, Body};
use axum::{http, http::Request};
use tower::ServiceExt;

use serde_json::json;

use svc_passport::{health::Health, http::router::build_router};

#[path = "../src/test_support.rs"]
mod test_support;
use test_support::default_config;

#[tokio::test]
async fn verify_rejects_bad_alg() {
    let mut cfg = default_config();
    cfg.security.require_aud = false;

    let app = build_router(cfg, Health::default());

    let body = Body::from(
        serde_json::to_vec(&json!({
            "alg": "RSA256",
            "kid": "K",
            "msg_b64": "eA==",
            "sig_b64": "eA=="
        }))
        .unwrap(),
    );

    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/verify")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);

    let buf = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let txt = String::from_utf8_lossy(&buf);
    assert!(txt.contains("BadAlg"));
}

#[tokio::test]
async fn verify_missing_aud_when_required_400() {
    let mut cfg = default_config();
    cfg.security.require_aud = true;

    let app = build_router(cfg, Health::default());

    let body = Body::from(
        serde_json::to_vec(&json!({
            "alg": "Ed25519",
            "kid": "K",
            "msg_b64": "eA==",
            "sig_b64": "eA=="
            // no aud
        }))
        .unwrap(),
    );

    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/verify")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);

    let buf = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let txt = String::from_utf8_lossy(&buf);
    assert!(txt.contains("BadAudience"));
}
