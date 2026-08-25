//! RO:WHAT — CN-4 acceptance for Omnigate's fixed IssueCapability challenge/proof proxy pair.
//! RO:WHY — Device-bound capability issuance must traverse the reviewed 9090 façade to svc-passport without moving capability or Passport authority into Omnigate.
//! RO:INTERACTS — Omnigate v1 router, shared Passport proxy, fixed-route policy admission, and dummy loopback svc-passport.
//! RO:INVARIANTS — exact fixed downstream paths; opaque body/header forwarding; 16 KiB cap; capability refresh/revoke remain absent; no capability verification/storage or username/value mutation.
//! RO:METRICS — none.
//! RO:CONFIG — process-local `OMNIGATE_PASSPORT_BASE_URL` override.
//! RO:SECURITY — no DeviceKey, RecoveryRoot, PIN, capability derivation, request-proof verification, username authority, wallet, or ledger authority.
//! RO:TEST — `cargo test -p omnigate --test crabnode_cn4_capability_proxy`.

use std::net::SocketAddr;

use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode, Uri},
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
    async fn handler(uri: Uri, headers: HeaderMap, body: Bytes) -> (StatusCode, Json<Value>) {
        let body: Value = serde_json::from_slice(&body).expect("forwarded JSON");

        (
            StatusCode::OK,
            Json(json!({
                "path": uri.path(),
                "body": body,
                "authorization":
                    header(&headers, "authorization"),
                "correlation":
                    header(&headers, "x-correlation-id"),
                "request_id":
                    header(&headers, "x-request-id"),
                "connection":
                    header(&headers, "connection"),
            })),
        )
    }

    serve(
        Router::new()
            .route("/v1/passport/capability/challenge", post(handler))
            .route("/v1/passport/capability/prove", post(handler)),
    )
    .await
}

fn clear_env() {
    std::env::remove_var("OMNIGATE_PASSPORT_BASE_URL");

    std::env::remove_var("OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL");
}

#[tokio::test]
async fn fixed_capability_issue_pair_proxies_exactly() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let passport = dummy_passport().await;

    std::env::set_var("OMNIGATE_PASSPORT_BASE_URL", format!("http://{passport}"));

    let omnigate = serve(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    for (public_path, downstream_path, marker) in [
        (
            "/v1/identity/passport/capability/challenge",
            "/v1/passport/capability/challenge",
            "capability-challenge",
        ),
        (
            "/v1/identity/passport/capability/prove",
            "/v1/passport/capability/prove",
            "capability-prove",
        ),
    ] {
        let payload = json!({
            "opaque_test_marker": marker,
        });

        let response = reqwest::Client::new()
            .post(format!("http://{omnigate}{public_path}"))
            .header("authorization", "Bearer cn4-capability-test")
            .header("content-type", "application/json")
            .header("x-correlation-id", format!("corr-{marker}"))
            .header("x-request-id", format!("req-{marker}"))
            .header("connection", "close")
            .json(&payload)
            .send()
            .await
            .expect("capability proxy response");

        assert_eq!(response.status(), StatusCode::OK,);

        let body: Value = response.json().await.expect("proxy JSON");

        assert_eq!(body["path"], downstream_path,);

        assert_eq!(body["body"], payload,);

        assert_eq!(body["authorization"], "Bearer cn4-capability-test",);

        assert_eq!(body["correlation"], format!("corr-{marker}"),);

        assert_eq!(body["request_id"], format!("req-{marker}"),);

        assert!(
            body["connection"].is_null(),
            "hop-by-hop Connection header must not be forwarded",
        );
    }

    clear_env();
}

#[tokio::test]
async fn fixed_capability_issue_pair_enforces_16_kib_cap() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let passport = dummy_passport().await;

    std::env::set_var("OMNIGATE_PASSPORT_BASE_URL", format!("http://{passport}"));

    let omnigate = serve(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    for path in [
        "/v1/identity/passport/capability/challenge",
        "/v1/identity/passport/capability/prove",
    ] {
        let response = reqwest::Client::new()
            .post(format!("http://{omnigate}{path}"))
            .header("content-type", "application/octet-stream")
            .body(vec![b'a'; 16_385])
            .send()
            .await
            .expect("oversized response");

        assert_eq!(
            response.status(),
            StatusCode::PAYLOAD_TOO_LARGE,
            "{path} must enforce the 16 KiB body cap",
        );
    }

    clear_env();
}

#[tokio::test]
async fn unreviewed_capability_lifecycle_routes_remain_absent() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate = serve(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    for path in [
        "/v1/identity/passport/capability/refresh",
        "/v1/identity/passport/capability/revoke",
        "/v1/identity/passport/capability/prove/extra",
    ] {
        let response = reqwest::Client::new()
            .post(format!("http://{omnigate}{path}"))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .expect("closed route response");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} must remain unmounted",
        );
    }

    clear_env();
}
