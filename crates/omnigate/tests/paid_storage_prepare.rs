//! paid_storage_prepare.rs — integration tests for `/v1/paid/o/prepare`.
//!
//! RO:WHAT — Spin up dummy svc-storage and real omnigate route; assert paid prepare DTO behavior.
//! RO:WHY — WEB3_2 product UX needs stable prepare responses before wallet holds.
//! RO:INTERACTS — omnigate::routes::v1::paid, svc-storage `/paid/o/estimate`, reqwest client.
//! RO:INVARIANTS — prepare calls storage estimate only; no wallet call, no capture/release, no bytes stored.
//! RO:METRICS — route metrics are covered when mounted through full App::build.
//! RO:CONFIG — OMNIGATE_STORAGE_BASE_URL, OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL.
//! RO:SECURITY — verifies auth/passport/idempotency headers reach storage without hop-by-hop headers.
//! RO:TEST — cargo test -p omnigate --test paid_storage_prepare.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    http::{HeaderMap, StatusCode, Uri},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Serialize)]
struct EstimateEcho {
    schema: &'static str,
    route: &'static str,
    action: &'static str,
    asset: &'static str,
    bytes: u64,
    amount_minor: &'static str,
    minimum_hold_minor: &'static str,
    pricing_mode: &'static str,
    authorization: Option<String>,
    x_ron_token: Option<String>,
    x_ron_passport: Option<String>,
    x_ron_wallet_account: Option<String>,
    idempotency_key: Option<String>,
    x_correlation_id: Option<String>,
    x_request_id: Option<String>,
    host: Option<String>,
    connection: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    reason: &'static str,
}

async fn start_dummy_storage() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn estimate_handler(headers: HeaderMap, uri: Uri) -> (StatusCode, Json<Value>) {
        let query = parse_query(&uri);
        let Some(raw_bytes) = query.get("bytes") else {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!(ErrorBody {
                    error: "bad_request",
                    reason: "missing required query parameter: bytes",
                })),
            );
        };

        let Ok(bytes) = raw_bytes.parse::<u64>() else {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!(ErrorBody {
                    error: "bad_request",
                    reason: "bytes must be an unsigned integer",
                })),
            );
        };

        if bytes == 13 {
            return (
                StatusCode::PAYMENT_REQUIRED,
                Json(serde_json::json!(ErrorBody {
                    error: "payment_required",
                    reason: "storage estimate rejected test fixture",
                })),
            );
        }

        (
            StatusCode::OK,
            Json(serde_json::json!(EstimateEcho {
                schema: "svc-storage.paid-storage-estimate.v1",
                route: "/paid/o",
                action: "paid_storage_put",
                asset: "roc",
                bytes,
                amount_minor: "84",
                minimum_hold_minor: "100",
                pricing_mode: "roc-economics",
                authorization: grab(&headers, "authorization"),
                x_ron_token: grab(&headers, "x-ron-token"),
                x_ron_passport: grab(&headers, "x-ron-passport"),
                x_ron_wallet_account: grab(&headers, "x-ron-wallet-account"),
                idempotency_key: grab(&headers, "idempotency-key"),
                x_correlation_id: grab(&headers, "x-correlation-id"),
                x_request_id: grab(&headers, "x-request-id"),
                host: grab(&headers, "host"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/paid/o/estimate", get(estimate_handler));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy storage");
    let addr = listener.local_addr().expect("dummy storage local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy storage serve");
    });

    wait_for_health(format!("http://{addr}/healthz")).await;
    addr
}

async fn start_omnigate_paid_route(storage_addr: SocketAddr) -> SocketAddr {
    let storage_base = format!("http://{storage_addr}");

    std::env::set_var("OMNIGATE_STORAGE_BASE_URL", storage_base);

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate paid route");
    let addr = listener
        .local_addr()
        .expect("omnigate paid route local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate paid route serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    addr
}

#[tokio::test]
async fn paid_object_prepare_returns_hold_template_from_storage_estimate() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let omnigate_addr = start_omnigate_paid_route(storage_addr).await;

    let request = serde_json::json!({
        "bytes": 48,
        "payer_account": "acct_creator_alice",
        "owner_passport_subject": "passport:main:alice",
        "asset_kind": "image",
        "content_type": "image/png",
        "expected_asset_cid": "b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57",
        "client_idempotency_key": "idem-image-prepare-1"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/paid/o/prepare"))
        .header("authorization", "Bearer dev")
        .header("x-ron-token", "ron-token-123")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_creator_alice")
        .header("idempotency-key", "idem-image-prepare-1")
        .header("x-correlation-id", "corr-paid-prepare")
        .header("x-request-id", "req-paid-prepare")
        .header("connection", "close")
        .json(&request)
        .send()
        .await
        .expect("omnigate paid prepare response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse paid prepare JSON body");

    assert_eq!(body["schema"], "omnigate.paid-object-prepare.v1");
    assert_eq!(body["action"], "paid_storage_put");
    assert_eq!(body["asset"], "roc");
    assert_eq!(body["bytes"], 48);
    assert_eq!(body["asset_kind"], "image");
    assert_eq!(body["content_type"], "image/png");
    assert_eq!(body["owner_passport_subject"], "passport:main:alice");
    assert_eq!(
        body["expected_asset_cid"],
        "b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57"
    );

    assert_eq!(
        body["storage_estimate"]["schema"],
        "svc-storage.paid-storage-estimate.v1"
    );
    assert_eq!(body["storage_estimate"]["bytes"], 48);
    assert_eq!(body["storage_estimate"]["amount_minor"], "84");
    assert_eq!(body["storage_estimate"]["minimum_hold_minor"], "100");

    assert_eq!(body["storage_estimate"]["authorization"], "Bearer dev");
    assert_eq!(body["storage_estimate"]["x_ron_token"], "ron-token-123");
    assert_eq!(
        body["storage_estimate"]["x_ron_passport"],
        "passport:main:alice"
    );
    assert_eq!(
        body["storage_estimate"]["x_ron_wallet_account"],
        "acct_creator_alice"
    );
    assert_eq!(
        body["storage_estimate"]["idempotency_key"],
        "idem-image-prepare-1"
    );
    assert_eq!(
        body["storage_estimate"]["x_correlation_id"],
        "corr-paid-prepare"
    );
    assert_eq!(body["storage_estimate"]["x_request_id"], "req-paid-prepare");
    assert!(body["storage_estimate"]["connection"].is_null());

    assert_eq!(body["wallet_hold"]["required"], true);
    assert_eq!(body["wallet_hold"]["action"], "paid_storage_put");
    assert_eq!(body["wallet_hold"]["currency"], "ROC");
    assert_eq!(body["wallet_hold"]["amount_minor"], "84");
    assert_eq!(body["wallet_hold"]["minimum_hold_minor"], "100");
    assert_eq!(body["wallet_hold"]["payer_account"], "acct_creator_alice");
    assert_eq!(
        body["wallet_hold"]["idempotency_key_hint"],
        "idem-image-prepare-1"
    );
    assert_eq!(
        body["wallet_hold"]["capability"]["required_action"],
        "wallet.hold"
    );
    assert_eq!(
        body["wallet_hold"]["capability"]["resource"],
        "paid_storage_put"
    );
    assert_eq!(body["wallet_hold"]["capability"]["audience"], "svc-wallet");

    assert_eq!(body["submit"]["method"], "POST");
    assert_eq!(body["submit"]["gateway_path"], "/paid/o");
    assert_eq!(body["submit"]["omnigate_path"], "/v1/paid/o");
    assert_eq!(body["submit"]["storage_path"], "/paid/o");
    assert!(body["submit"]["required_headers"]
        .as_array()
        .is_some_and(|headers| headers
            .iter()
            .any(|header| header == "x-ron-wallet-hold-txid")));

    assert!(body["warnings"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty));

    clear_env();
}

#[tokio::test]
async fn paid_object_prepare_maps_storage_estimate_errors() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let omnigate_addr = start_omnigate_paid_route(storage_addr).await;

    let request = serde_json::json!({
        "bytes": 13,
        "payer_account": "acct_creator_alice"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/paid/o/prepare"))
        .json(&request)
        .send()
        .await
        .expect("omnigate paid prepare rejected response");

    assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);

    let body: Value = resp.json().await.expect("parse rejected prepare JSON body");
    assert_eq!(body["code"], "storage_estimate_rejected");
    assert_eq!(body["message"], "storage estimate rejected prepare request");
    assert_eq!(body["retryable"], false);
    assert_eq!(body["reason"], "storage_estimate_rejected");
    assert_eq!(body["storage_status"], 402);
    assert_eq!(body["storage_error"]["error"], "payment_required");
    assert_eq!(
        body["storage_error"]["reason"],
        "storage estimate rejected test fixture"
    );

    clear_env();
}

#[tokio::test]
async fn paid_object_prepare_rejects_bad_json_without_storage_call() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let omnigate_addr = start_omnigate_paid_route(storage_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/paid/o/prepare"))
        .header("content-type", "application/json")
        .body(r#"{"bytes":48,"unknown":true}"#)
        .send()
        .await
        .expect("omnigate paid prepare bad JSON response");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = resp.json().await.expect("parse bad JSON prepare body");
    assert_eq!(body["code"], "invalid_prepare_request");
    assert_eq!(body["message"], "prepare request must be strict JSON");
    assert_eq!(body["retryable"], false);
    assert_eq!(body["reason"], "bad_json");

    clear_env();
}

#[tokio::test]
async fn paid_object_prepare_upstream_connect_failure_yields_problem_502() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    std::env::set_var("OMNIGATE_STORAGE_BASE_URL", "http://127.0.0.1:1");

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate paid route");
    let addr = listener
        .local_addr()
        .expect("omnigate paid route local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate paid route serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let request = serde_json::json!({
        "bytes": 48,
        "payer_account": "acct_creator_alice"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/v1/paid/o/prepare"))
        .json(&request)
        .send()
        .await
        .expect("omnigate paid prepare response");

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    let body: Value = resp.json().await.expect("parse omnigate Problem body");
    assert_eq!(body["code"], "upstream_unavailable");
    assert_eq!(body["message"], "storage estimate upstream unavailable");
    assert_eq!(body["retryable"], true);
    assert_eq!(body["reason"], "storage_connect");

    clear_env();
}

async fn wait_for_health(url: String) {
    let client = reqwest::Client::new();

    for _ in 0..50 {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    panic!("service did not become healthy at {url}");
}

fn parse_query(uri: &Uri) -> HashMap<String, String> {
    let mut map = HashMap::new();

    if let Some(query) = uri.query() {
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }

            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let key = key.trim();

            if key.is_empty() {
                continue;
            }

            map.insert(key.to_owned(), value.trim().to_owned());
        }
    }

    map
}

fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn clear_env() {
    std::env::remove_var("OMNIGATE_STORAGE_BASE_URL");
    std::env::remove_var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL");
}
