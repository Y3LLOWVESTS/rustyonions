// crates/svc-passport/tests/handlers.rs
use axum::body::to_bytes;
use axum::{body::Body, http, http::Request};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use tower::ServiceExt;

use svc_passport::{health::Health, http::router::build_router};

#[path = "../src/test_support.rs"]
mod test_support;

use test_support::default_config;

#[tokio::test]
async fn healthz_ok() {
    let app = build_router(default_config(), Health::default());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(resp.status().is_success());
}

#[tokio::test]
async fn issue_then_verify_ok() {
    let cfg = default_config();
    let expected_aud = cfg.passport.issuer.clone();
    let app = build_router(cfg, Health::default());

    // 1) issue. Current svc-passport contract returns the envelope flat:
    // { "alg": "Ed25519", "kid": "...", "msg_b64": "...", "sig_b64": "..." }.
    let issue_body = json!({
        "sub": "passport:main:dev",
        "aud": [expected_aud.clone()],
        "scopes": ["profile:read", "profile:write"],
        "ctx": {
            "purpose": "handlers_roundtrip"
        },
        "ttl_s": 900,
        "nonce": "handlers-test-nonce-001"
    });

    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/issue")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&issue_body).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();

    assert!(
        status.is_success(),
        "issue failed with status {status}; body={}",
        String::from_utf8_lossy(&bytes)
    );

    let env: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(env["alg"], "Ed25519");
    let kid = env["kid"]
        .as_str()
        .expect("kid in issue response")
        .to_owned();
    let msg_b64 = env["msg_b64"]
        .as_str()
        .expect("msg_b64 in issue response")
        .to_owned();
    let sig_b64 = env["sig_b64"]
        .as_str()
        .expect("sig_b64 in issue response")
        .to_owned();

    // 2) verify single. Current contract accepts the flat envelope and requires
    // top-level `aud` when [security].require_aud=true.
    let verify_env = json!({
        "alg": "Ed25519",
        "kid": kid,
        "msg_b64": msg_b64,
        "sig_b64": sig_b64,
        "aud": expected_aud.clone()
    });

    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/verify")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&verify_env).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();

    assert!(
        status.is_success(),
        "verify failed with status {status}; body={}",
        String::from_utf8_lossy(&bytes)
    );

    let ok: bool = serde_json::from_slice(&bytes).unwrap();
    assert!(ok, "verify should succeed");

    // 3) verify batch: current contract accepts a bare array and returns Vec<bool>.
    let mut sig_tampered = STANDARD
        .decode(sig_b64.as_bytes())
        .expect("signature should decode as standard base64");

    if !sig_tampered.is_empty() {
        sig_tampered[0] ^= 0x01;
    }

    let sig_tampered_b64 = STANDARD.encode(sig_tampered);

    let batch_envs = json!([
        verify_env,
        {
            "alg": "Ed25519",
            "kid": env["kid"].as_str().unwrap(),
            "msg_b64": env["msg_b64"].as_str().unwrap(),
            "sig_b64": sig_tampered_b64,
            "aud": expected_aud
        }
    ]);

    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/verify_batch")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&batch_envs).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();

    assert!(
        status.is_success(),
        "verify_batch failed with status {status}; body={}",
        String::from_utf8_lossy(&bytes)
    );

    let results: Vec<bool> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(results, vec![true, false], "batch should be [true,false]");
}
