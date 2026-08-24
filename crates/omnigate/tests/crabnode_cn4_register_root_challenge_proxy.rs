//! RO:WHAT — CN-4 acceptance tests for Omnigate's fixed RegisterRoot challenge proxy.
//! RO:WHY — Prove the reviewed Omnigate path reaches only the fixed svc-passport RegisterRoot endpoint.
//! RO:INTERACTS — Omnigate v1 router, Passport proxy, dummy loopback svc-passport.
//! RO:INVARIANTS — exact path/body forwarding; 16 KiB cap; nearby unreviewed challenge/prove routes remain absent; no Passport authority moves into Omnigate.
//! RO:CONFIG — process-local `OMNIGATE_PASSPORT_BASE_URL` override.
//! RO:SECURITY — safe context headers only; hop-by-hop headers are not forwarded; no secrets.
//! RO:TEST — `cargo test -p omnigate --test crabnode_cn4_register_root_challenge_proxy`.

use std::net::SocketAddr;

use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

fn header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

async fn serve(router: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("address");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve");
    });

    addr
}

async fn dummy_passport() -> SocketAddr {
    async fn handler(headers: HeaderMap, body: Bytes) -> (StatusCode, Json<Value>) {
        let body: Value = serde_json::from_slice(&body).expect("forwarded JSON");

        (
            StatusCode::OK,
            Json(json!({
                "path": "/v1/passport/register/challenge",
                "body": body,
                "authorization": header(&headers, "authorization"),
                "correlation": header(&headers, "x-correlation-id"),
                "request_id": header(&headers, "x-request-id"),
                "connection": header(&headers, "connection")
            })),
        )
    }

    serve(Router::new().route("/v1/passport/register/challenge", post(handler))).await
}

fn clear_env() {
    std::env::remove_var("OMNIGATE_PASSPORT_BASE_URL");
    std::env::remove_var("OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL");
}

#[tokio::test]
async fn fixed_register_root_route_proxies_exactly() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let passport = dummy_passport().await;

    std::env::set_var("OMNIGATE_PASSPORT_BASE_URL", format!("http://{passport}"));

    let omnigate = serve(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    let body = json!({
        "passport_id":
            "passport:v1:main:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "requested_scopes": [
            "identity.read",
            "profile.read"
        ],
        "operation_body_hash":
            "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
    });

    let response = reqwest::Client::new()
        .post(format!(
            "http://{omnigate}/v1/identity/passport/register/challenge"
        ))
        .header("authorization", "Bearer cn4")
        .header("x-correlation-id", "corr-cn4")
        .header("x-request-id", "req-cn4")
        .header("connection", "close")
        .json(&body)
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);

    let response: Value = response.json().await.expect("JSON response");

    assert_eq!(response["path"], "/v1/passport/register/challenge",);
    assert_eq!(response["body"], body);
    assert_eq!(response["authorization"], "Bearer cn4");
    assert_eq!(response["correlation"], "corr-cn4");
    assert_eq!(response["request_id"], "req-cn4");
    assert!(
        response["connection"].is_null(),
        "hop-by-hop Connection header must not reach svc-passport",
    );

    clear_env();
}

#[tokio::test]
async fn nearby_unreviewed_challenge_and_prove_remain_absent() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate = serve(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    for path in [
        "/v1/identity/passport/challenge/extra",
        "/v1/identity/passport/prove/extra",
    ] {
        let response = reqwest::Client::new()
            .post(format!("http://{omnigate}{path}"))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .expect("negative response");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} must remain unmounted",
        );
    }

    clear_env();
}

#[tokio::test]
async fn register_root_route_enforces_16_kib_cap() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let passport = dummy_passport().await;

    std::env::set_var("OMNIGATE_PASSPORT_BASE_URL", format!("http://{passport}"));

    let omnigate = serve(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    let response = reqwest::Client::new()
        .post(format!(
            "http://{omnigate}/v1/identity/passport/register/challenge"
        ))
        .header("content-type", "application/octet-stream")
        .body(vec![b'a'; 16_385])
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE,);

    clear_env();
}

#[tokio::test]
async fn unavailable_passport_fails_as_bad_gateway() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    std::env::set_var("OMNIGATE_PASSPORT_BASE_URL", "http://127.0.0.1:9");

    let omnigate = serve(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    let response = reqwest::Client::new()
        .post(format!(
            "http://{omnigate}/v1/identity/passport/register/challenge"
        ))
        .header("content-type", "application/json")
        .body("{}")
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY,);

    clear_env();
}
