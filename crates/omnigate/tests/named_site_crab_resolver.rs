//! RO:WHAT — Integration test for resolving named `crab://<site>` URLs through the crab resolver.
//! RO:WHY — WEB3_2/NEXT_LEVEL site proof needs `/crab/resolve?url=crab://name` to hydrate the same page as `/v1/sites/:name`.
//! RO:INTERACTS — omnigate::routes::v1::crab, omnigate::routes::v1::sites, dummy svc-index/storage fixtures.
//! RO:INVARIANTS — names are human pointers; b3 hashes stay canonical; resolver remains read-only; storage/index own durable truth.
//! RO:CONFIG — OMNIGATE_STORAGE_BASE_URL, OMNIGATE_INDEX_BASE_URL.
//! RO:SECURITY — no wallet/ledger mutation; no backend HTML execution; strict site/name fixture.
//! RO:TEST — cargo test -p omnigate --test named_site_crab_resolver.

use std::{net::SocketAddr, time::Duration};

use axum::{http::StatusCode, routing::get, Json, Router};
use serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

const ROOT_DOCUMENT_CID: &str =
    "b3:1111111111111111111111111111111111111111111111111111111111111111";
const SITE_MANIFEST_CID: &str =
    "b3:2222222222222222222222222222222222222222222222222222222222222222";

#[tokio::test]
async fn crab_resolve_named_site_hydrates_site_page() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_route(storage_addr, index_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{omnigate_addr}/v1/crab/resolve"))
        .query(&[("url", "crab://SeaLobsta.COM")])
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:alice")
        .header("x-ron-wallet-account", "acct_site_owner")
        .send()
        .await
        .expect("omnigate named crab site resolve response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse named crab site JSON body");

    assert_eq!(body["schema"], "omnigate.site-page.v1");
    assert_eq!(body["site_name"], "sealobsta.com");
    assert_eq!(body["root_document_cid"], ROOT_DOCUMENT_CID);
    assert_eq!(body["manifest"]["status"], "present");
    assert_eq!(body["manifest"]["hydration_status"], "hydrated");
    assert_eq!(body["manifest"]["manifest_cid"], SITE_MANIFEST_CID);
    assert_eq!(body["links"]["crab"], "crab://sealobsta.com");
    assert_eq!(body["links"]["resolve"], "/v1/sites/sealobsta.com");
    assert_eq!(body["owner"]["passport_subject"], "passport:main:alice");
    assert_eq!(body["owner"]["wallet_account"], "acct_site_owner");
    assert_eq!(body["payout"]["default_action"], "site_visit");
    assert_eq!(body["payout"]["recipient_account"], "acct_site_owner");
    assert_eq!(body["metadata"]["title"], "SeaLobsta Demo");
    assert_eq!(body["route_map"]["/"], ROOT_DOCUMENT_CID);
    assert_eq!(body["asset_map"]["index.html"], ROOT_DOCUMENT_CID);
    assert_eq!(body["receipts"][0]["tx_id"], "hold_site_launch_1");

    clear_env();
}

#[tokio::test]
async fn crab_resolve_bare_b3_hash_still_requires_asset_kind() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let storage_addr = start_dummy_storage().await;
    let index_addr = start_dummy_index().await;
    let omnigate_addr = start_omnigate_route(storage_addr, index_addr).await;

    let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{omnigate_addr}/v1/crab/resolve"))
        .query(&[("url", format!("crab://{hash}"))])
        .send()
        .await
        .expect("omnigate bare hash response");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = resp.json().await.expect("parse bad request body");
    assert_eq!(body["code"], "invalid_crab_url");
    assert_eq!(body["reason"], "missing_asset_kind");

    clear_env();
}

async fn start_dummy_storage() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn get_object_handler(
        axum::extract::Path(cid): axum::extract::Path<String>,
    ) -> (StatusCode, Json<Value>) {
        assert_eq!(cid, SITE_MANIFEST_CID);
        (StatusCode::OK, Json(site_manifest_json()))
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
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

    async fn get_site_pointer(
        axum::extract::Path(site_name): axum::extract::Path<String>,
    ) -> (StatusCode, Json<Value>) {
        assert_eq!(site_name, "sealobsta.com");
        (StatusCode::OK, Json(site_pointer_json()))
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/index/sites/:name/manifest", get(get_site_pointer));

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

async fn start_omnigate_route(storage_addr: SocketAddr, index_addr: SocketAddr) -> SocketAddr {
    std::env::set_var(
        "OMNIGATE_STORAGE_BASE_URL",
        format!("http://{storage_addr}"),
    );
    std::env::set_var("OMNIGATE_INDEX_BASE_URL", format!("http://{index_addr}"));

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate route");
    let addr = listener.local_addr().expect("omnigate route local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate route serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
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

fn site_pointer_json() -> Value {
    serde_json::json!({
        "version": 1,
        "name": "sealobsta.com",
        "manifest_cid": SITE_MANIFEST_CID,
        "owner_passport_subject": "passport:main:alice",
        "owner_wallet_account": "acct_site_owner",
        "updated_at_ms": 1_776_000_000_000u64
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
            "splits": []
        },
        "metadata": {
            "title": "SeaLobsta Demo",
            "description": "A WEB3_2 static site launch fixture.",
            "tags": ["site", "demo"]
        },
        "provenance": {
            "created_by": "omnigate.site-test"
        },
        "storage": {
            "root_document_cid": ROOT_DOCUMENT_CID
        },
        "receipts": [
            {
                "tx_id": "hold_site_launch_1",
                "receipt_kind": "paid_site_launch",
                "amount_minor_units": 200,
                "account": "acct_site_owner"
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
