//! image_asset_prepare.rs — integration tests for `/v1/assets/image/*`.
//!
//! RO:WHAT — Spin up dummy svc-storage/index and real omnigate routes; assert image prepare/upload behavior.
//! RO:WHY — WEB3_2 crab://image demo needs prepare UX and upload coordination.
//! RO:INTERACTS — omnigate::routes::v1::assets, svc-storage `/paid/o`, `/o`, svc-index pointer route.
//! RO:INVARIANTS — no wallet calls from omnigate; storage enforces paid write; index owns manifest pointer.
//! RO:CONFIG — OMNIGATE_STORAGE_BASE_URL, OMNIGATE_INDEX_BASE_URL.
//! RO:TEST — cargo test -p omnigate --test image_asset_prepare.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode, Uri},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

const ASSET_CID: &str = "b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57";
const MANIFEST_CID: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

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
    connection: Option<String>,
}

#[derive(Debug, Serialize)]
struct UploadEcho {
    cid: &'static str,
    paid: bool,
    payer: &'static str,
    escrow: &'static str,
    wallet_txid: &'static str,
    wallet_receipt_hash: &'static str,
    estimate_minor: &'static str,
    body_len: usize,
    content_type: Option<String>,
    authorization: Option<String>,
    wallet_hold_txid: Option<String>,
    x_ron_passport: Option<String>,
    x_ron_wallet_account: Option<String>,
    idempotency_key: Option<String>,
    connection: Option<String>,
}

#[derive(Debug, Serialize)]
struct PutObjectResponse {
    cid: &'static str,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    reason: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PutAssetManifestPointer {
    asset_kind: String,
    manifest_cid: String,
    owner_passport_subject: Option<String>,
    owner_wallet_account: Option<String>,
    updated_at_ms: u64,
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
                    reason: "storage estimate rejected image fixture",
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
                connection: grab(&headers, "connection"),
            })),
        )
    }

    async fn paid_upload_handler(headers: HeaderMap, body: Bytes) -> (StatusCode, Json<Value>) {
        if body.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!(ErrorBody {
                    error: "bad_request",
                    reason: "empty body",
                })),
            );
        }

        (
            StatusCode::OK,
            Json(serde_json::json!(UploadEcho {
                cid: ASSET_CID,
                paid: true,
                payer: "acct_creator_alice",
                escrow: "escrow_paid_write",
                wallet_txid: "tx_paid_image_1",
                wallet_receipt_hash: "receipt_hash_paid_image_1",
                estimate_minor: "84",
                body_len: body.len(),
                content_type: grab(&headers, "content-type"),
                authorization: grab(&headers, "authorization"),
                wallet_hold_txid: grab(&headers, "x-ron-wallet-hold-txid"),
                x_ron_passport: grab(&headers, "x-ron-passport"),
                x_ron_wallet_account: grab(&headers, "x-ron-wallet-account"),
                idempotency_key: grab(&headers, "idempotency-key"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    async fn put_object_handler(headers: HeaderMap, body: Bytes) -> (StatusCode, Json<Value>) {
        let content_type = grab(&headers, "content-type");
        assert_eq!(content_type.as_deref(), Some("application/json"));

        let manifest: Value =
            serde_json::from_slice(&body).expect("omnigate should send manifest JSON");

        assert_eq!(manifest["version"], 1);
        assert_eq!(manifest["asset_cid"], ASSET_CID);
        assert_eq!(manifest["asset_kind"], "image");
        assert_eq!(manifest["owner"]["passport_subject"], "passport:main:alice");
        assert_eq!(manifest["owner"]["wallet_account"], "acct_creator_alice");
        assert_eq!(
            manifest["payout"]["recipient_account"],
            "acct_creator_alice"
        );
        assert_eq!(manifest["metadata"]["title"], "Demo Crab Image");
        assert_eq!(
            manifest["metadata"]["description"],
            "A WEB3_2 coordinated image upload fixture."
        );
        assert_eq!(manifest["metadata"]["content_type"], "image/png");
        assert_eq!(manifest["metadata"]["tags"][0], "demo");
        assert_eq!(manifest["metadata"]["tags"][1], "image");
        assert_eq!(manifest["receipts"][0]["tx_id"], "tx_paid_image_1");
        assert_eq!(manifest["receipts"][0]["receipt_kind"], "paid_storage");
        assert_eq!(manifest["receipts"][0]["amount_minor_units"], 84);

        (
            StatusCode::OK,
            Json(serde_json::json!(PutObjectResponse { cid: MANIFEST_CID })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/paid/o/estimate", get(estimate_handler))
        .route("/paid/o", post(paid_upload_handler))
        .route("/o", post(put_object_handler));

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

async fn start_dummy_index() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn put_asset_pointer(
        axum::extract::Path(asset_cid): axum::extract::Path<String>,
        Json(body): Json<PutAssetManifestPointer>,
    ) -> (StatusCode, Json<Value>) {
        assert_eq!(
            asset_cid,
            "730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57"
        );
        assert_eq!(body.asset_kind, "image");
        assert_eq!(body.manifest_cid, MANIFEST_CID);
        assert_eq!(
            body.owner_passport_subject.as_deref(),
            Some("passport:main:alice")
        );
        assert_eq!(
            body.owner_wallet_account.as_deref(),
            Some("acct_creator_alice")
        );
        assert!(body.updated_at_ms > 0);

        (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "version": 1,
                "asset_cid": ASSET_CID,
                "asset_kind": "image",
                "manifest_cid": MANIFEST_CID,
                "owner_passport_subject": "passport:main:alice",
                "owner_wallet_account": "acct_creator_alice",
                "updated_at_ms": body.updated_at_ms
            })),
        )
    }

    let router = Router::new().route("/healthz", get(healthz)).route(
        "/v1/index/assets/:asset_cid/manifest",
        put(put_asset_pointer),
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy index");
    let addr = listener.local_addr().expect("dummy index local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy index serve");
    });

    wait_for_health(format!("http://{addr}/healthz")).await;
    addr
}

async fn start_omnigate_assets_route(
    storage_addr: SocketAddr,
    index_addr: SocketAddr,
) -> SocketAddr {
    let storage_base = format!("http://{storage_addr}");
    let index_base = format!("http://{index_addr}");

    std::env::set_var("OMNIGATE_STORAGE_BASE_URL", storage_base);
    std::env::set_var("OMNIGATE_INDEX_BASE_URL", index_base);

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate assets route");
    let addr = listener
        .local_addr()
        .expect("omnigate assets route local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate assets route serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    addr
}

#[tokio::test]
async fn image_asset_prepare_returns_image_specific_hold_template() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_assets_route(storage_addr, index_addr).await;

    let request = serde_json::json!({
        "bytes": 48,
        "payer_account": "acct_creator_alice",
        "owner_passport_subject": "passport:main:alice",
        "content_type": "image/png",
        "expected_asset_cid": ASSET_CID,
        "title": "Demo Crab Image",
        "description": "A WEB3_2 image prepare fixture.",
        "tags": ["demo", "image"],
        "client_idempotency_key": "idem-image-prepare-1"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/assets/image/prepare"))
        .header("authorization", "Bearer dev")
        .header("x-ron-token", "ron-token-123")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_creator_alice")
        .header("idempotency-key", "idem-image-prepare-1")
        .header("x-correlation-id", "corr-image-prepare")
        .header("x-request-id", "req-image-prepare")
        .header("connection", "close")
        .json(&request)
        .send()
        .await
        .expect("omnigate image prepare response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse image prepare JSON body");

    assert_eq!(body["schema"], "omnigate.image-asset-prepare.v1");
    assert_eq!(body["asset_kind"], "image");
    assert_eq!(body["action"], "paid_storage_put");
    assert_eq!(body["asset"], "roc");
    assert_eq!(body["bytes"], 48);
    assert_eq!(body["content_type"], "image/png");
    assert_eq!(body["title"], "Demo Crab Image");
    assert_eq!(body["description"], "A WEB3_2 image prepare fixture.");
    assert_eq!(body["expected_asset_cid"], ASSET_CID);
    assert_eq!(body["owner_passport_subject"], "passport:main:alice");

    assert_eq!(body["paid_storage"]["estimate_path"], "/v1/paid/o/prepare");
    assert_eq!(body["paid_storage"]["submit_path"], "/v1/paid/o");
    assert_eq!(
        body["paid_storage"]["estimate"]["schema"],
        "svc-storage.paid-storage-estimate.v1"
    );
    assert_eq!(body["paid_storage"]["estimate"]["bytes"], 48);
    assert_eq!(body["paid_storage"]["estimate"]["amount_minor"], "84");
    assert_eq!(
        body["paid_storage"]["estimate"]["minimum_hold_minor"],
        "100"
    );

    assert_eq!(
        body["paid_storage"]["estimate"]["authorization"],
        "Bearer dev"
    );
    assert_eq!(
        body["paid_storage"]["estimate"]["x_ron_token"],
        "ron-token-123"
    );
    assert_eq!(
        body["paid_storage"]["estimate"]["x_ron_passport"],
        "passport:main:alice"
    );
    assert_eq!(
        body["paid_storage"]["estimate"]["x_ron_wallet_account"],
        "acct_creator_alice"
    );
    assert_eq!(
        body["paid_storage"]["estimate"]["idempotency_key"],
        "idem-image-prepare-1"
    );
    assert_eq!(
        body["paid_storage"]["estimate"]["x_correlation_id"],
        "corr-image-prepare"
    );
    assert_eq!(
        body["paid_storage"]["estimate"]["x_request_id"],
        "req-image-prepare"
    );
    assert!(body["paid_storage"]["estimate"]["connection"].is_null());

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
    assert_eq!(body["wallet_hold"]["capability"]["audience"], "svc-wallet");

    assert_eq!(body["manifest_preview"]["will_create_manifest"], true);
    assert_eq!(body["manifest_preview"]["will_index_asset_pointer"], true);
    assert_eq!(
        body["manifest_preview"]["owner_source"],
        "request.owner_passport_subject_or_upload_headers"
    );

    assert_eq!(body["next"]["create_hold"], "/v1/wallet/hold");
    assert_eq!(body["next"]["submit_upload"], "/v1/assets/image");
    assert_eq!(
        body["next"]["resolve_after_upload"],
        "/v1/crab/resolve?url=crab://<hash>.image"
    );

    assert!(body["warnings"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty));

    clear_env();
}

#[tokio::test]
async fn image_asset_prepare_rejects_non_image_content_type() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_assets_route(storage_addr, index_addr).await;

    let request = serde_json::json!({
        "bytes": 48,
        "content_type": "text/html"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/assets/image/prepare"))
        .json(&request)
        .send()
        .await
        .expect("omnigate image prepare invalid content-type response");

    assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let body: Value = resp
        .json()
        .await
        .expect("parse invalid content type JSON body");

    assert_eq!(body["code"], "invalid_image_content_type");
    assert_eq!(body["retryable"], false);
    assert_eq!(body["reason"], "invalid_content_type");

    clear_env();
}

#[tokio::test]
async fn image_asset_prepare_maps_storage_estimate_errors() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_assets_route(storage_addr, index_addr).await;

    let request = serde_json::json!({
        "bytes": 13,
        "content_type": "image/png"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/assets/image/prepare"))
        .json(&request)
        .send()
        .await
        .expect("omnigate image prepare rejected response");

    assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);

    let body: Value = resp
        .json()
        .await
        .expect("parse rejected image prepare JSON body");

    assert_eq!(body["code"], "storage_estimate_rejected");
    assert_eq!(
        body["message"],
        "storage estimate rejected image prepare request"
    );
    assert_eq!(body["retryable"], false);
    assert_eq!(body["reason"], "storage_estimate_rejected");
    assert_eq!(body["storage_status"], 402);
    assert_eq!(body["storage_error"]["error"], "payment_required");
    assert_eq!(
        body["storage_error"]["reason"],
        "storage estimate rejected image fixture"
    );

    clear_env();
}

#[tokio::test]
async fn image_asset_upload_coordinates_paid_storage_manifest_and_index_pointer() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_assets_route(storage_addr, index_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/assets/image"))
        .header("authorization", "Bearer dev")
        .header("content-type", "image/png")
        .header("x-ron-wallet-hold-txid", "hold_tx_123")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_creator_alice")
        .header("x-ron-asset-title", "Demo Crab Image")
        .header(
            "x-ron-asset-description",
            "A WEB3_2 coordinated image upload fixture.",
        )
        .header("x-ron-asset-tags", "demo,image")
        .header("idempotency-key", "idem-image-upload-1")
        .header("connection", "close")
        .body(Bytes::from_static(
            b"not really png but good enough for coordinator test",
        ))
        .send()
        .await
        .expect("omnigate image upload coordinator response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse image upload JSON body");

    assert_eq!(body["schema"], "omnigate.image-asset-upload.v1");
    assert_eq!(body["asset_kind"], "image");
    assert_eq!(body["asset_cid"], ASSET_CID);
    assert_eq!(
        body["crab_url"],
        "crab://730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57.image"
    );

    assert_eq!(body["storage_upload"]["cid"], ASSET_CID);
    assert_eq!(body["storage_upload"]["paid"], true);
    assert_eq!(body["storage_upload"]["content_type"], "image/png");
    assert_eq!(body["storage_upload"]["authorization"], "Bearer dev");
    assert_eq!(body["storage_upload"]["wallet_hold_txid"], "hold_tx_123");
    assert_eq!(
        body["storage_upload"]["x_ron_passport"],
        "passport:main:alice"
    );
    assert_eq!(
        body["storage_upload"]["x_ron_wallet_account"],
        "acct_creator_alice"
    );
    assert_eq!(
        body["storage_upload"]["idempotency_key"],
        "idem-image-upload-1"
    );
    assert!(body["storage_upload"]["connection"].is_null());
    assert!(
        body["storage_upload"]["body_len"]
            .as_u64()
            .unwrap_or_default()
            > 0
    );

    assert_eq!(body["manifest"]["status"], "stored");
    assert_eq!(body["manifest"]["manifest_cid"], MANIFEST_CID);
    assert_eq!(body["manifest"]["storage_path"], "/o");

    assert_eq!(body["index_pointer"]["status"], "stored");
    assert_eq!(
        body["index_pointer"]["route"],
        "/v1/index/assets/730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57/manifest"
    );
    assert_eq!(body["index_pointer"]["http_status"], 202);

    assert_eq!(body["owner"]["passport_subject"], "passport:main:alice");
    assert_eq!(body["owner"]["wallet_account"], "acct_creator_alice");
    assert_eq!(body["payout"]["default_action"], "content_view");
    assert_eq!(body["payout"]["recipient_account"], "acct_creator_alice");

    assert_eq!(body["links"]["raw"], format!("/o/{ASSET_CID}"));
    assert_eq!(
        body["links"]["crab"],
        "crab://730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57.image"
    );
    assert_eq!(
        body["links"]["http_b3"],
        "/v1/b3/730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57.image"
    );
    assert_eq!(
        body["links"]["resolve"],
        "/v1/crab/resolve?url=crab://730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57.image"
    );
    assert_eq!(body["links"]["manifest_raw"], format!("/o/{MANIFEST_CID}"));

    assert!(body["warnings"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty));

    clear_env();
}

#[tokio::test]
async fn image_asset_prepare_upstream_connect_failure_yields_problem_502() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    std::env::set_var("OMNIGATE_STORAGE_BASE_URL", "http://127.0.0.1:1");

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate assets route");
    let addr = listener
        .local_addr()
        .expect("omnigate assets route local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate assets route serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let request = serde_json::json!({
        "bytes": 48,
        "content_type": "image/png"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/v1/assets/image/prepare"))
        .json(&request)
        .send()
        .await
        .expect("omnigate image prepare response");

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    let body: Value = resp.json().await.expect("parse upstream problem body");
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
    std::env::remove_var("OMNIGATE_INDEX_BASE_URL");
    std::env::remove_var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL");
}
