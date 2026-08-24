//! RO:WHAT — CN-4 acceptance tests for Omnigate's exact DeviceAuthorize proxy.
//! RO:WHY — Prove the reviewed route forwards only to svc-passport's fixed device-admission endpoint.
//! RO:INTERACTS — Omnigate v1 router, Passport proxy, and dummy loopback svc-passport.
//! RO:INVARIANTS — exact opaque forwarding; 16 KiB body cap; nearby unreviewed device/prove routes remain absent; no Passport authority moves into Omnigate.
//! RO:METRICS — none.
//! RO:CONFIG — process-local OMNIGATE_PASSPORT_BASE_URL override.
//! RO:SECURITY — safe context headers only; no root/device private key, possession proof, capability, wallet, or ledger authority.
//! RO:TEST — cargo test -p omnigate --test crabnode_cn4_device_authorize_proxy.

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
                "path":
                    "/v1/passport/device/authorize",
                "body":
                    body,
                "authorization":
                    header(
                        &headers,
                        "authorization",
                    ),
                "correlation":
                    header(
                        &headers,
                        "x-correlation-id",
                    ),
                "request_id":
                    header(
                        &headers,
                        "x-request-id",
                    ),
                "connection":
                    header(
                        &headers,
                        "connection",
                    ),
            })),
        )
    }

    serve(Router::new().route("/v1/passport/device/authorize", post(handler))).await
}

fn clear_env() {
    std::env::remove_var("OMNIGATE_PASSPORT_BASE_URL");

    std::env::remove_var("OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL");
}

#[tokio::test]
async fn fixed_device_authorize_route_proxies_exactly() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let passport = dummy_passport().await;

    std::env::set_var("OMNIGATE_PASSPORT_BASE_URL", format!("http://{passport}"));

    let omnigate = serve(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    let payload = json!({
        "authorization": {
            "opaque_test_marker":
                "cn4-device-authorize",
        },
    });

    let response = reqwest::Client::new()
        .post(format!(
            "http://{omnigate}/v1/identity/passport/device/authorize"
        ))
        .header("authorization", "Bearer cn4-device")
        .header("x-correlation-id", "corr-cn4-device")
        .header("x-request-id", "req-cn4-device")
        .header("connection", "close")
        .json(&payload)
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK,);

    let body: Value = response.json().await.expect("JSON response");

    assert_eq!(body["path"], "/v1/passport/device/authorize",);
    assert_eq!(body["body"], payload);
    assert_eq!(body["authorization"], "Bearer cn4-device",);
    assert_eq!(body["correlation"], "corr-cn4-device",);
    assert_eq!(body["request_id"], "req-cn4-device",);
    assert!(
        body["connection"].is_null(),
        "hop-by-hop Connection header must not reach svc-passport",
    );

    clear_env();
}

#[tokio::test]
async fn device_authorize_route_enforces_16_kib_cap() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let passport = dummy_passport().await;

    std::env::set_var("OMNIGATE_PASSPORT_BASE_URL", format!("http://{passport}"));

    let omnigate = serve(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    let response = reqwest::Client::new()
        .post(format!(
            "http://{omnigate}/v1/identity/passport/device/authorize"
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
async fn nearby_device_and_unreviewed_prove_routes_remain_absent() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let omnigate = serve(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    for path in [
        "/v1/identity/passport/device/register",
        "/v1/identity/passport/prove/extra",
    ] {
        let response = reqwest::Client::new()
            .post(format!("http://{omnigate}{path}"))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .expect("negative route response");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} must remain unmounted",
        );
    }

    clear_env();
}
