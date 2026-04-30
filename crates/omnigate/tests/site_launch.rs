//! site_launch.rs — integration tests for `/v1/sites/*`.
//!
//! RO:WHAT — Spin up dummy svc-storage/index and real omnigate routes; assert site prepare/create/resolve behavior.
//! RO:WHY — WEB3_2 Batch 8/9 needs crab://site backend foundation before browser-extension UX.
//! RO:INTERACTS — omnigate::routes::v1::sites, svc-storage `/paid/o/estimate`, `/o`, svc-index site pointer.
//! RO:INVARIANTS — no wallet calls from omnigate; index stores pointer only; storage stores manifest bytes.
//! RO:CONFIG — OMNIGATE_STORAGE_BASE_URL, OMNIGATE_INDEX_BASE_URL.
//! RO:TEST — cargo test -p omnigate --test site_launch.

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

const ROOT_DOCUMENT_CID: &str =
    "b3:1111111111111111111111111111111111111111111111111111111111111111";
const SITE_MANIFEST_CID: &str =
    "b3:2222222222222222222222222222222222222222222222222222222222222222";

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
    x_correlation_id: Option<String>,
    x_request_id: Option<String>,
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
struct PutSiteManifestPointer {
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
                    reason: "storage estimate rejected site fixture",
                })),
            );
        }

        (
            StatusCode::OK,
            Json(serde_json::json!(EstimateEcho {
                schema: "svc-storage.paid-storage-estimate.v1",
                route: "/paid/o",
                action: "paid_site_launch",
                asset: "roc",
                bytes,
                amount_minor: "144",
                minimum_hold_minor: "200",
                pricing_mode: "roc-economics",
                authorization: grab(&headers, "authorization"),
                x_ron_passport: grab(&headers, "x-ron-passport"),
                x_ron_wallet_account: grab(&headers, "x-ron-wallet-account"),
                idempotency_key: grab(&headers, "idempotency-key"),
                x_correlation_id: grab(&headers, "x-correlation-id"),
                x_request_id: grab(&headers, "x-request-id"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    async fn put_object_handler(headers: HeaderMap, body: Bytes) -> (StatusCode, Json<Value>) {
        let content_type = grab(&headers, "content-type");
        assert_eq!(content_type.as_deref(), Some("application/json"));

        let manifest: Value =
            serde_json::from_slice(&body).expect("omnigate should send site manifest JSON");

        assert_eq!(manifest["version"], 1);
        assert_eq!(manifest["site_name"], "sealobsta.com");
        assert_eq!(manifest["root_document_cid"], ROOT_DOCUMENT_CID);
        assert_eq!(manifest["owner"]["passport_subject"], "passport:main:alice");
        assert_eq!(manifest["owner"]["wallet_account"], "acct_site_owner");
        assert_eq!(manifest["payout"]["recipient_account"], "acct_site_owner");
        assert_eq!(manifest["metadata"]["title"], "SeaLobsta Demo");
        assert_eq!(
            manifest["metadata"]["description"],
            "A WEB3_2 static site launch fixture."
        );
        assert_eq!(manifest["route_map"]["/"], ROOT_DOCUMENT_CID);
        assert_eq!(manifest["asset_map"]["index.html"], ROOT_DOCUMENT_CID);
        assert_eq!(manifest["receipts"][0]["tx_id"], "hold_site_launch_1");
        assert_eq!(manifest["receipts"][0]["receipt_kind"], "paid_site_launch");

        (
            StatusCode::OK,
            Json(serde_json::json!(PutObjectResponse {
                cid: SITE_MANIFEST_CID
            })),
        )
    }

    async fn get_object_handler(
        axum::extract::Path(cid): axum::extract::Path<String>,
    ) -> (StatusCode, Json<Value>) {
        assert_eq!(cid, SITE_MANIFEST_CID);

        (StatusCode::OK, Json(site_manifest_json()))
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/paid/o/estimate", get(estimate_handler))
        .route("/o", post(put_object_handler))
        .route("/o/:cid", get(get_object_handler));

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

    async fn put_site_pointer(
        axum::extract::Path(site_name): axum::extract::Path<String>,
        Json(body): Json<PutSiteManifestPointer>,
    ) -> (StatusCode, Json<Value>) {
        assert_eq!(site_name, "sealobsta.com");
        assert_eq!(body.manifest_cid, SITE_MANIFEST_CID);
        assert_eq!(
            body.owner_passport_subject.as_deref(),
            Some("passport:main:alice")
        );
        assert_eq!(
            body.owner_wallet_account.as_deref(),
            Some("acct_site_owner")
        );
        assert!(body.updated_at_ms > 0);

        (
            StatusCode::ACCEPTED,
            Json(site_pointer_json(body.updated_at_ms)),
        )
    }

    async fn get_site_pointer(
        axum::extract::Path(site_name): axum::extract::Path<String>,
    ) -> (StatusCode, Json<Value>) {
        assert_eq!(site_name, "sealobsta.com");

        (
            StatusCode::OK,
            Json(site_pointer_json(1_776_000_000_000u64)),
        )
    }

    let router = Router::new().route("/healthz", get(healthz)).route(
        "/v1/index/sites/:name/manifest",
        put(put_site_pointer).get(get_site_pointer),
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

async fn start_omnigate_sites_route(
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
        .expect("bind omnigate sites route");
    let addr = listener
        .local_addr()
        .expect("omnigate sites route local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate sites route serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    addr
}

#[tokio::test]
async fn site_prepare_returns_wallet_hold_template() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_sites_route(storage_addr, index_addr).await;

    let request = serde_json::json!({
        "site_name": "SeaLobsta.COM",
        "files": [
            { "path": "index.html", "bytes": 100 },
            { "path": "assets/app.css", "bytes": 50 }
        ],
        "payer_account": "acct_site_owner",
        "owner_passport_subject": "passport:main:alice",
        "owner_wallet_account": "acct_site_owner",
        "title": "SeaLobsta Demo",
        "description": "A WEB3_2 static site prepare fixture.",
        "client_idempotency_key": "idem-site-prepare-1"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/sites/prepare"))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_site_owner")
        .header("idempotency-key", "idem-site-prepare-1")
        .header("x-correlation-id", "corr-site-prepare")
        .header("x-request-id", "req-site-prepare")
        .header("connection", "close")
        .json(&request)
        .send()
        .await
        .expect("omnigate site prepare response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse site prepare JSON body");

    assert_eq!(body["schema"], "omnigate.site-prepare.v1");
    assert_eq!(body["site_name"], "sealobsta.com");
    assert_eq!(body["action"], "paid_site_launch");
    assert_eq!(body["asset"], "roc");
    assert_eq!(body["total_bytes"], 150);
    assert_eq!(body["owner_passport_subject"], "passport:main:alice");
    assert_eq!(body["owner_wallet_account"], "acct_site_owner");
    assert_eq!(body["title"], "SeaLobsta Demo");
    assert_eq!(body["file_count"], 2);

    assert_eq!(
        body["paid_storage"]["estimate"]["schema"],
        "svc-storage.paid-storage-estimate.v1"
    );
    assert_eq!(body["paid_storage"]["estimate"]["bytes"], 150);
    assert_eq!(body["paid_storage"]["estimate"]["amount_minor"], "144");
    assert_eq!(
        body["paid_storage"]["estimate"]["minimum_hold_minor"],
        "200"
    );
    assert_eq!(
        body["paid_storage"]["estimate"]["authorization"],
        "Bearer dev"
    );
    assert_eq!(
        body["paid_storage"]["estimate"]["x_ron_passport"],
        "passport:main:alice"
    );
    assert_eq!(
        body["paid_storage"]["estimate"]["x_ron_wallet_account"],
        "acct_site_owner"
    );
    assert_eq!(
        body["paid_storage"]["estimate"]["idempotency_key"],
        "idem-site-prepare-1"
    );
    assert!(body["paid_storage"]["estimate"]["connection"].is_null());

    assert_eq!(body["wallet_hold"]["required"], true);
    assert_eq!(body["wallet_hold"]["action"], "paid_site_launch");
    assert_eq!(body["wallet_hold"]["currency"], "ROC");
    assert_eq!(body["wallet_hold"]["amount_minor"], "144");
    assert_eq!(body["wallet_hold"]["minimum_hold_minor"], "200");
    assert_eq!(body["wallet_hold"]["payer_account"], "acct_site_owner");
    assert_eq!(
        body["wallet_hold"]["idempotency_key_hint"],
        "idem-site-prepare-1"
    );
    assert_eq!(
        body["wallet_hold"]["capability"]["required_action"],
        "wallet.hold"
    );
    assert_eq!(
        body["wallet_hold"]["capability"]["resource"],
        "paid_site_launch"
    );

    assert_eq!(
        body["site_manifest_preview"]["will_create_site_manifest"],
        true
    );
    assert_eq!(
        body["site_manifest_preview"]["will_index_site_pointer"],
        true
    );
    assert_eq!(
        body["site_manifest_preview"]["name_pointer_route"],
        "/v1/index/sites/sealobsta.com/manifest"
    );

    assert_eq!(body["next"]["create_hold"], "/v1/wallet/hold");
    assert_eq!(body["next"]["submit_site"], "/v1/sites");
    assert_eq!(
        body["next"]["resolve_after_launch"],
        "/v1/sites/sealobsta.com"
    );

    assert!(body["warnings"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty));

    clear_env();
}

#[tokio::test]
async fn site_prepare_rejects_bad_site_name() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_sites_route(storage_addr, index_addr).await;

    let request = serde_json::json!({
        "site_name": "../bad",
        "total_bytes": 42
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/sites/prepare"))
        .json(&request)
        .send()
        .await
        .expect("omnigate bad site prepare response");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = resp.json().await.expect("parse bad site prepare body");
    assert_eq!(body["code"], "invalid_site_name");
    assert_eq!(body["retryable"], false);
    assert_eq!(body["reason"], "double_dot");

    clear_env();
}

#[tokio::test]
async fn site_prepare_maps_storage_estimate_errors() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_sites_route(storage_addr, index_addr).await;

    let request = serde_json::json!({
        "site_name": "sealobsta.com",
        "total_bytes": 13
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/sites/prepare"))
        .json(&request)
        .send()
        .await
        .expect("omnigate rejected site prepare response");

    assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);

    let body: Value = resp
        .json()
        .await
        .expect("parse rejected site prepare JSON body");

    assert_eq!(body["code"], "storage_estimate_rejected");
    assert_eq!(
        body["message"],
        "storage estimate rejected site prepare request"
    );
    assert_eq!(body["retryable"], false);
    assert_eq!(body["reason"], "storage_estimate_rejected");
    assert_eq!(body["storage_status"], 402);
    assert_eq!(body["storage_error"]["error"], "payment_required");
    assert_eq!(
        body["storage_error"]["reason"],
        "storage estimate rejected site fixture"
    );

    clear_env();
}

#[tokio::test]
async fn site_create_stores_manifest_and_index_pointer() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_sites_route(storage_addr, index_addr).await;

    let request = serde_json::json!({
        "site_name": "SeaLobsta.COM",
        "root_document_cid": ROOT_DOCUMENT_CID,
        "owner_passport_subject": "passport:main:alice",
        "owner_wallet_account": "acct_site_owner",
        "title": "SeaLobsta Demo",
        "description": "A WEB3_2 static site launch fixture.",
        "route_map": {
            "/": ROOT_DOCUMENT_CID
        },
        "asset_map": {
            "index.html": ROOT_DOCUMENT_CID
        }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{omnigate_addr}/v1/sites"))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_site_owner")
        .header("x-ron-wallet-hold-txid", "hold_site_launch_1")
        .header("idempotency-key", "idem-site-create-1")
        .header("connection", "close")
        .json(&request)
        .send()
        .await
        .expect("omnigate site create response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse site create JSON body");

    assert_eq!(body["schema"], "omnigate.site-create.v1");
    assert_eq!(body["site_name"], "sealobsta.com");
    assert_eq!(body["root_document_cid"], ROOT_DOCUMENT_CID);

    assert_eq!(body["manifest"]["status"], "stored");
    assert_eq!(body["manifest"]["manifest_cid"], SITE_MANIFEST_CID);
    assert_eq!(body["manifest"]["storage_path"], "/o");

    assert_eq!(body["index_pointer"]["status"], "stored");
    assert_eq!(
        body["index_pointer"]["route"],
        "/v1/index/sites/sealobsta.com/manifest"
    );
    assert_eq!(body["index_pointer"]["http_status"], 202);

    assert_eq!(body["owner"]["passport_subject"], "passport:main:alice");
    assert_eq!(body["owner"]["wallet_account"], "acct_site_owner");
    assert_eq!(body["payout"]["default_action"], "site_visit");
    assert_eq!(body["payout"]["recipient_account"], "acct_site_owner");

    assert_eq!(body["links"]["crab"], "crab://sealobsta.com");
    assert_eq!(body["links"]["resolve"], "/v1/sites/sealobsta.com");
    assert_eq!(
        body["links"]["manifest_raw"],
        format!("/o/{SITE_MANIFEST_CID}")
    );

    assert!(body["warnings"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty));

    clear_env();
}

#[tokio::test]
async fn site_resolve_hydrates_manifest_from_index_and_storage() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_sites_route(storage_addr, index_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{omnigate_addr}/v1/sites/SeaLobsta.COM"))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_site_owner")
        .header("connection", "close")
        .send()
        .await
        .expect("omnigate site resolve response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse site page JSON body");

    assert_eq!(body["schema"], "omnigate.site-page.v1");
    assert_eq!(body["site_name"], "sealobsta.com");
    assert_eq!(body["root_document_cid"], ROOT_DOCUMENT_CID);

    assert_eq!(body["manifest"]["status"], "present");
    assert_eq!(body["manifest"]["hydration_status"], "hydrated");
    assert_eq!(body["manifest"]["manifest_cid"], SITE_MANIFEST_CID);
    assert_eq!(body["manifest"]["updated_at_ms"], 1_776_000_000_000u64);
    assert_eq!(
        body["manifest"]["manifest_raw"],
        format!("/o/{SITE_MANIFEST_CID}")
    );

    assert_eq!(body["owner"]["passport_subject"], "passport:main:alice");
    assert_eq!(body["owner"]["wallet_account"], "acct_site_owner");
    assert_eq!(body["payout"]["default_action"], "site_visit");
    assert_eq!(body["payout"]["recipient_account"], "acct_site_owner");

    assert_eq!(body["metadata"]["title"], "SeaLobsta Demo");
    assert_eq!(
        body["metadata"]["description"],
        "A WEB3_2 static site launch fixture."
    );
    assert_eq!(body["metadata"]["tags"][0], "site");

    assert_eq!(body["route_map"]["/"], ROOT_DOCUMENT_CID);
    assert_eq!(body["asset_map"]["index.html"], ROOT_DOCUMENT_CID);
    assert_eq!(body["receipts"][0]["tx_id"], "hold_site_launch_1");
    assert_eq!(body["receipts"][0]["receipt_kind"], "paid_site_launch");

    assert_eq!(body["links"]["crab"], "crab://sealobsta.com");
    assert_eq!(body["links"]["resolve"], "/v1/sites/sealobsta.com");
    assert_eq!(
        body["links"]["manifest_raw"],
        format!("/o/{SITE_MANIFEST_CID}")
    );

    assert!(body["warnings"]
        .as_array()
        .is_some_and(std::vec::Vec::is_empty));

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

fn site_pointer_json(updated_at_ms: u64) -> Value {
    serde_json::json!({
        "version": 1,
        "name": "sealobsta.com",
        "manifest_cid": SITE_MANIFEST_CID,
        "owner_passport_subject": "passport:main:alice",
        "owner_wallet_account": "acct_site_owner",
        "updated_at_ms": updated_at_ms
    })
}

fn site_manifest_json() -> Value {
    serde_json::json!({
        "version": 1,
        "site_name": "sealobsta.com",
        "root_document_cid": ROOT_DOCUMENT_CID,
        "asset_map": {
            "index.html": ROOT_DOCUMENT_CID
        },
        "route_map": {
            "/": ROOT_DOCUMENT_CID
        },
        "owner": {
            "passport_subject": "passport:main:alice",
            "wallet_account": "acct_site_owner"
        },
        "payout": {
            "default_action": "site_visit",
            "recipient_account": "acct_site_owner",
            "splits": [
                {
                    "role": "creator",
                    "account": "acct_site_owner",
                    "bps": 10_000
                }
            ]
        },
        "metadata": {
            "title": "SeaLobsta Demo",
            "description": "A WEB3_2 static site launch fixture.",
            "tags": ["site"]
        },
        "receipts": [
            {
                "tx_id": "hold_site_launch_1",
                "receipt_kind": "paid_site_launch",
                "account": "acct_site_owner",
                "created_at_ms": 1_776_000_000_001u64
            }
        ]
    })
}

fn clear_env() {
    std::env::remove_var("OMNIGATE_STORAGE_BASE_URL");
    std::env::remove_var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL");
    std::env::remove_var("OMNIGATE_INDEX_BASE_URL");
    std::env::remove_var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL");
}
