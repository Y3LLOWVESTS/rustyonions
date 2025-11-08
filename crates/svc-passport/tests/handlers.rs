// crates/svc-passport/tests/handlers.rs
use axum::body::to_bytes;
use axum::{body::Body, http, http::Request};
use tower::ServiceExt;

use svc_passport::{health::Health, http::router::build_router};

#[path = "../src/test_support.rs"]
mod test_support;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::json;
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
    let app = build_router(default_config(), Health::default());

    // 1) issue
    let body = Body::from(serde_json::to_vec(&json!({"hello":"world"})).unwrap());
    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/issue")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success());

    // axum 0.7: to_bytes(body, limit)
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let env: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let kid = env["kid"].as_str().unwrap().to_string();
    let msg_b64 = env["msg_b64"].as_str().unwrap().to_string();
    let sig_b64 = env["sig_b64"].as_str().unwrap().to_string();

    // 2) verify single
    let verify_env = json!({
        "kid": kid,
        "msg_b64": msg_b64,
        "sig_b64": sig_b64,
        "alg": "Ed25519"
    });
    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/verify")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&verify_env).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success());
    let ok: bool =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(ok, "verify should succeed");

    // 3) verify batch (one good, one tampered)
    let mut sig_tampered = STANDARD.decode(sig_b64.as_str()).unwrap();
    if !sig_tampered.is_empty() {
        sig_tampered[0] ^= 0x01;
    }
    let sig_tampered_b64 = STANDARD.encode(sig_tampered);

    let batch_envs = json!([
        verify_env,
        {
          "kid": env["kid"].as_str().unwrap(),
          "msg_b64": env["msg_b64"].as_str().unwrap(),
          "sig_b64": sig_tampered_b64,
          "alg": "Ed25519"
        }
    ]);
    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/verify_batch")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&batch_envs).unwrap()))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success());
    let results: Vec<bool> =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(results, vec![true, false], "batch should be [true,false]");
}
