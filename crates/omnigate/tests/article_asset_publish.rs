//! article_asset_publish.rs — integration tests for `/v1/assets/article/*`.
//!
//! RO:WHAT — Spin up dummy svc-storage/index and real omnigate routes; assert article prepare/publish behavior.
//! RO:WHY — NEXT_LEVEL stages article after post/comment so long-form text earns the same b3+manifest+index green gate.
//! RO:INTERACTS — omnigate::routes::v1::text_assets, svc-storage `/paid/o`, `/o`, svc-index pointer route.
//! RO:INVARIANTS — no wallet calls from omnigate; storage enforces paid write; index owns manifest pointer; site relation required.
//! RO:CONFIG — OMNIGATE_STORAGE_BASE_URL, OMNIGATE_INDEX_BASE_URL.
//! RO:TEST — cargo test -p omnigate --test article_asset_publish.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    body::Bytes,
    extract::Path,
    http::{HeaderMap, StatusCode, Uri},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

const ARTICLE_ASSET_CID: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ARTICLE_MANIFEST_CID: &str =
    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HERO_IMAGE_URL: &str =
    "crab://cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc.image";
const SOURCE_POST_URL: &str =
    "crab://dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd.post";

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
    x_ron_passport: Option<String>,
    x_ron_wallet_account: Option<String>,
    idempotency_key: Option<String>,
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
    x_ron_paid_op: Option<String>,
    x_ron_paid_asset: Option<String>,
    x_ron_paid_estimate_minor: Option<String>,
    x_ron_wallet_txid: Option<String>,
    x_ron_wallet_receipt_hash: Option<String>,
    x_ron_wallet_from: Option<String>,
    x_ron_wallet_to: Option<String>,
    x_ron_asset_kind: Option<String>,
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

        (
            StatusCode::OK,
            Json(serde_json::json!(EstimateEcho {
                schema: "svc-storage.paid-storage-estimate.v1",
                route: "/paid/o",
                action: "paid_storage_put",
                asset: "roc",
                bytes,
                amount_minor: "34",
                minimum_hold_minor: "34",
                pricing_mode: "roc-economics",
                authorization: grab(&headers, "authorization"),
                x_ron_passport: grab(&headers, "x-ron-passport"),
                x_ron_wallet_account: grab(&headers, "x-ron-wallet-account"),
                idempotency_key: grab(&headers, "idempotency-key"),
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

        let content: Value =
            serde_json::from_slice(&body).expect("omnigate should send article content JSON");

        assert_eq!(content["schema"], "ron.article-content.v1");
        assert_eq!(content["asset_kind"], "article");
        assert_eq!(content["title"], "First backend article");
        assert_eq!(content["subtitle"], "Long-form text enters the b3 graph");
        assert_eq!(content["summary"], "A short summary for article cards.");
        assert_eq!(
            content["body"],
            "This article should become a b3-backed long-form text asset."
        );
        assert_eq!(content["metadata"]["article_kind"], "essay");
        assert_eq!(content["metadata"]["language"], "en");
        assert_eq!(content["metadata"]["tags"][0], "article");
        assert_eq!(content["relations"]["site"], "crab://the-dusty-onion");
        assert_eq!(content["relations"]["hero_image"], HERO_IMAGE_URL);
        assert_eq!(content["relations"]["source"], SOURCE_POST_URL);
        assert_eq!(content["site_connection"]["relation"], "article_on_site");

        (
            StatusCode::OK,
            Json(serde_json::json!(UploadEcho {
                cid: ARTICLE_ASSET_CID,
                paid: true,
                payer: "acct_creator_alice",
                escrow: "escrow_paid_write",
                wallet_txid: "tx_paid_article_1",
                wallet_receipt_hash: "receipt_hash_paid_article_1",
                estimate_minor: "34",
                body_len: body.len(),
                content_type: grab(&headers, "content-type"),
                authorization: grab(&headers, "authorization"),
                x_ron_paid_op: grab(&headers, "x-ron-paid-op"),
                x_ron_paid_asset: grab(&headers, "x-ron-paid-asset"),
                x_ron_paid_estimate_minor: grab(&headers, "x-ron-paid-estimate-minor"),
                x_ron_wallet_txid: grab(&headers, "x-ron-wallet-txid"),
                x_ron_wallet_receipt_hash: grab(&headers, "x-ron-wallet-receipt-hash"),
                x_ron_wallet_from: grab(&headers, "x-ron-wallet-from"),
                x_ron_wallet_to: grab(&headers, "x-ron-wallet-to"),
                x_ron_asset_kind: grab(&headers, "x-ron-asset-kind"),
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
            serde_json::from_slice(&body).expect("omnigate should send article manifest JSON");

        assert_eq!(manifest["version"], 1);
        assert_eq!(manifest["asset_cid"], ARTICLE_ASSET_CID);
        assert_eq!(manifest["asset_kind"], "article");
        assert_eq!(manifest["owner"]["passport_subject"], "passport:main:alice");
        assert_eq!(manifest["owner"]["wallet_account"], "acct_creator_alice");
        assert_eq!(manifest["metadata"]["title"], "First backend article");
        assert_eq!(
            manifest["metadata"]["summary"],
            "A short summary for article cards."
        );
        assert_eq!(manifest["metadata"]["hero_image_crab_url"], HERO_IMAGE_URL);
        assert_eq!(
            manifest["metadata"]["linked_source_crab_url"],
            SOURCE_POST_URL
        );
        assert_eq!(
            manifest["site_connection"]["crab_url"],
            "crab://the-dusty-onion"
        );
        assert_eq!(manifest["article_references"]["hero_image"], HERO_IMAGE_URL);
        assert_eq!(manifest["article_references"]["source"], SOURCE_POST_URL);
        assert_eq!(manifest["receipts"][0]["tx_id"], "tx_paid_article_1");
        assert_eq!(manifest["receipts"][0]["receipt_kind"], "paid_storage");
        assert_eq!(manifest["receipts"][0]["amount_minor_units"], 34);

        (
            StatusCode::OK,
            Json(serde_json::json!(PutObjectResponse {
                cid: ARTICLE_MANIFEST_CID
            })),
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
        Path(asset_cid): Path<String>,
        Json(body): Json<PutAssetManifestPointer>,
    ) -> (StatusCode, Json<Value>) {
        assert_eq!(
            asset_cid,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(body.asset_kind, "article");
        assert_eq!(body.manifest_cid, ARTICLE_MANIFEST_CID);
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
                "asset_cid": ARTICLE_ASSET_CID,
                "asset_kind": "article",
                "manifest_cid": ARTICLE_MANIFEST_CID,
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
async fn article_asset_prepare_returns_hold_template() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_assets_route(storage_addr, index_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/assets/article/prepare"))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_creator_alice")
        .header("idempotency-key", "idem-article-prepare-1")
        .header("connection", "close")
        .json(&article_request())
        .send()
        .await
        .expect("omnigate article prepare response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse article prepare JSON body");

    assert_eq!(body["schema"], "omnigate.article-asset-prepare.v1");
    assert_eq!(body["asset_kind"], "article");
    assert_eq!(body["action"], "paid_storage_put");
    assert_eq!(body["asset"], "roc");
    assert_eq!(body["content_type"], "application/json; charset=utf-8");
    assert_eq!(body["title"], "First backend article");
    assert_eq!(
        body["site_connection"]["crab_url"],
        "crab://the-dusty-onion"
    );
    assert!(body["parent_reference"].is_null());
    assert_eq!(body["owner_passport_subject"], "passport:main:alice");
    assert_eq!(body["wallet_hold"]["currency"], "ROC");
    assert_eq!(body["wallet_hold"]["amount_minor"], "34");
    assert_eq!(body["next"]["submit_publish"], "/v1/assets/article");
    assert_eq!(
        body["next"]["resolve_after_publish"],
        "/v1/crab/resolve?url=crab://<hash>.article"
    );

    clear_env();
}

#[tokio::test]
async fn article_asset_prepare_requires_title() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_assets_route(storage_addr, index_addr).await;

    let mut request = article_request();
    request["title"] = Value::Null;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/assets/article/prepare"))
        .json(&request)
        .send()
        .await
        .expect("omnigate article prepare validation response");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = resp.json().await.expect("parse validation JSON body");
    assert_eq!(body["code"], "invalid_article_prepare_request");
    assert_eq!(body["reason"], "missing_title");

    clear_env();
}

#[tokio::test]
async fn article_asset_prepare_rejects_non_image_hero() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_assets_route(storage_addr, index_addr).await;

    let mut request = article_request();
    request["hero_image_crab_url"] = Value::String(
        "crab://eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee.post".to_owned(),
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/assets/article/prepare"))
        .json(&request)
        .send()
        .await
        .expect("omnigate article prepare hero validation response");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = resp.json().await.expect("parse hero validation JSON body");
    assert_eq!(body["code"], "invalid_article_prepare_request");
    assert_eq!(body["reason"], "invalid_hero_image_kind");

    clear_env();
}

#[tokio::test]
async fn article_asset_publish_coordinates_paid_storage_manifest_and_index_pointer() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_assets_route(storage_addr, index_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/assets/article"))
        .header("authorization", "Bearer dev")
        .header("content-type", "application/json")
        .header("x-ron-paid-op", "hold")
        .header("x-ron-paid-asset", "roc")
        .header("x-ron-paid-estimate-minor", "34")
        .header("x-ron-wallet-txid", "tx_paid_article_1")
        .header("x-ron-wallet-receipt-hash", "receipt_hash_paid_article_1")
        .header("x-ron-wallet-from", "acct_creator_alice")
        .header("x-ron-wallet-to", "escrow_paid_write")
        .header("x-ron-asset-kind", "article")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_creator_alice")
        .header("idempotency-key", "idem-article-publish-1")
        .header("connection", "close")
        .json(&article_request())
        .send()
        .await
        .expect("omnigate article publish coordinator response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse article publish JSON body");

    assert_eq!(body["schema"], "omnigate.article-asset-publish.v1");
    assert_eq!(body["asset_kind"], "article");
    assert_eq!(body["asset_cid"], ARTICLE_ASSET_CID);
    assert_eq!(
        body["crab_url"],
        "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article"
    );

    assert_eq!(body["storage_upload"]["cid"], ARTICLE_ASSET_CID);
    assert_eq!(body["storage_upload"]["paid"], true);
    assert_eq!(
        body["storage_upload"]["content_type"],
        "application/json; charset=utf-8"
    );
    assert_eq!(body["storage_upload"]["x_ron_paid_op"], "hold");
    assert_eq!(body["storage_upload"]["x_ron_paid_asset"], "roc");
    assert_eq!(body["storage_upload"]["x_ron_paid_estimate_minor"], "34");
    assert_eq!(
        body["storage_upload"]["x_ron_wallet_txid"],
        "tx_paid_article_1"
    );
    assert_eq!(
        body["storage_upload"]["x_ron_wallet_receipt_hash"],
        "receipt_hash_paid_article_1"
    );
    assert_eq!(body["storage_upload"]["x_ron_asset_kind"], "article");
    assert!(body["storage_upload"]["connection"].is_null());

    assert_eq!(body["manifest"]["status"], "stored");
    assert_eq!(body["manifest"]["manifest_cid"], ARTICLE_MANIFEST_CID);
    assert_eq!(body["manifest"]["storage_path"], "/o");

    assert_eq!(body["index_pointer"]["status"], "stored");
    assert_eq!(
        body["index_pointer"]["route"],
        "/v1/index/assets/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/manifest"
    );
    assert_eq!(body["index_pointer"]["http_status"], 202);

    assert_eq!(body["owner"]["passport_subject"], "passport:main:alice");
    assert_eq!(body["owner"]["wallet_account"], "acct_creator_alice");
    assert_eq!(body["payout"]["default_action"], "content_view");
    assert_eq!(body["payout"]["recipient_account"], "acct_creator_alice");

    assert_eq!(
        body["site_connection"]["crab_url"],
        "crab://the-dusty-onion"
    );

    assert_eq!(body["links"]["raw"], format!("/o/{ARTICLE_ASSET_CID}"));
    assert_eq!(
        body["links"]["crab"],
        "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article"
    );
    assert_eq!(
        body["links"]["http_b3"],
        "/v1/b3/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article"
    );
    assert_eq!(
        body["links"]["resolve"],
        "/v1/crab/resolve?url=crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article"
    );
    assert_eq!(
        body["links"]["manifest_raw"],
        format!("/o/{ARTICLE_MANIFEST_CID}")
    );

    assert!(body["warnings"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty));

    clear_env();
}

fn article_request() -> Value {
    serde_json::json!({
        "title": "First backend article",
        "subtitle": "Long-form text enters the b3 graph",
        "summary": "A short summary for article cards.",
        "body": "This article should become a b3-backed long-form text asset.",
        "creator_display": "@alice",
        "language": "en",
        "article_kind": "essay",
        "visibility": "public_preview",
        "rights_mode": "creator_owned_original",
        "moderation_mode": "site_policy_or_creator_default",
        "site_context_crab_url": "crab://the-dusty-onion",
        "hero_image_crab_url": HERO_IMAGE_URL,
        "linked_source_crab_url": SOURCE_POST_URL,
        "tags": ["article", "backend"],
        "content_warning": "none",
        "payer_account": "acct_creator_alice",
        "owner_passport_subject": "passport:main:alice",
        "client_idempotency_key": "idem-article-prepare-1"
    })
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
