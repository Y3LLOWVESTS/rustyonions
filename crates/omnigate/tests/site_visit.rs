//! RO:WHAT — Integration tests for Omnigate paid site_visit quote/pay routes.
//! RO:WHY — NEXT_LEVEL creator-economy proof; Visitor B must pay Creator A through svc-wallet receipts.
//! RO:INTERACTS — omnigate::routes::v1::site_visit, dummy svc-index/storage/wallet upstreams.
//! RO:INVARIANTS — quote is read-only; pay uses wallet transfer only; nonce recovery does not mint or bypass wallet.
//! RO:METRICS — exercises route middleware when mounted by the real app; dummy upstreams expose no metrics.
//! RO:CONFIG — OMNIGATE_INDEX_BASE_URL, OMNIGATE_STORAGE_BASE_URL, OMNIGATE_WALLET_BASE_URL, OMNIGATE_SITE_VISIT_PRICE_MINOR.
//! RO:SECURITY — strict route DTOs; payout recipient must come from manifest; bearer token remains an upstream header.
//! RO:TEST — cargo test -p omnigate --test site_visit.

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

const SITE_NAME: &str = "ron7";
const ROOT_DOCUMENT_CID: &str =
    "b3:056cd088f6bb9ae45b11b1cd4786a2eb9b62f576e35f7132b33a74e4f07dac96";
const SITE_MANIFEST_CID: &str =
    "b3:918dad927484a43c2f716691794e24e7a608c951f38b9a6341a39f558284a4a9";
const LEDGER_ROOT: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const RECEIPT_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[derive(Debug, Clone)]
struct TestStack {
    omnigate_base_url: String,
    transfer_attempts: Arc<AtomicUsize>,
}

#[derive(Debug, Clone)]
struct WalletFixtureState {
    transfer_attempts: Arc<AtomicUsize>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    message: &'static str,
    code: &'static str,
}

#[tokio::test]
async fn quote_returns_10_roc_and_manifest_payout_recipient() {
    let _guard = ENV_LOCK.lock().await;
    let stack = start_test_stack().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "{}/v1/sites/{SITE_NAME}/visit/quote",
            stack.omnigate_base_url
        ))
        .header("authorization", "Bearer dev")
        .header("content-type", "application/json")
        .header("x-ron-passport", "passport:main:visitor-b")
        .header("x-ron-wallet-account", "acct_visitor_b")
        .header("idempotency-key", "site-visit-quote-test")
        .json(&json!({
            "site_name": SITE_NAME,
            "crab_url": "crab://ron7",
            "action": "site_visit",
            "quantity": 1,
            "payer_account": "acct_visitor_b",
            "visitor_wallet_account": "acct_visitor_b",
            "visitor_passport_subject": "passport:main:visitor-b",
            "recipient_account": "acct_dev",
            "max_amount_minor": "10",
            "client_idempotency_key": "site-visit-quote-test"
        }))
        .send()
        .await
        .expect("quote response");

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("quote JSON");
    assert_eq!(body["schema"], "omnigate.site-visit-quote.v1");
    assert_eq!(body["ok"], true);
    assert_eq!(body["site_name"], SITE_NAME);
    assert_eq!(body["crab_url"], "crab://ron7");
    assert_eq!(body["action"], "site_visit");
    assert_eq!(body["asset"], "roc");
    assert_eq!(body["amount_minor"], "10");
    assert_eq!(body["display_amount"], "10 ROC");
    assert_eq!(body["payer_account"], "acct_visitor_b");
    assert_eq!(body["visitor_wallet_account"], "acct_visitor_b");
    assert_eq!(body["visitor_passport_subject"], "passport:main:visitor-b");
    assert_eq!(body["recipient_account"], "acct_dev");
    assert_eq!(
        body["quote"]["policy"]["wallet_front_door"],
        "svc-wallet /v1/transfer"
    );
    assert_eq!(body["quote"]["site"]["manifest_cid"], SITE_MANIFEST_CID);
    assert_eq!(
        body["quote"]["site"]["root_document_cid"],
        ROOT_DOCUMENT_CID
    );
    assert_eq!(stack.transfer_attempts.load(Ordering::SeqCst), 0);

    clear_site_visit_env();
}

#[tokio::test]
async fn pay_recovers_wallet_nonce_and_returns_wallet_receipt() {
    let _guard = ENV_LOCK.lock().await;
    let stack = start_test_stack().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "{}/v1/sites/{SITE_NAME}/visit/pay",
            stack.omnigate_base_url
        ))
        .header("authorization", "Bearer dev")
        .header("content-type", "application/json")
        .header("x-ron-passport", "passport:main:visitor-b")
        .header("x-ron-wallet-account", "acct_visitor_b")
        .header("idempotency-key", "site-visit-pay-test")
        .json(&json!({
            "site_name": SITE_NAME,
            "crab_url": "crab://ron7",
            "action": "site_visit",
            "quantity": 1,
            "payer_account": "acct_visitor_b",
            "visitor_wallet_account": "acct_visitor_b",
            "visitor_passport_subject": "passport:main:visitor-b",
            "recipient_account": "acct_dev",
            "amount_minor": "10",
            "asset": "roc",
            "quote_id": "site-visit-test-quote",
            "quote_hash": "quotehash",
            "nonce": 1,
            "client_idempotency_key": "site-visit-pay-test"
        }))
        .send()
        .await
        .expect("pay response");

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("pay JSON");
    assert_eq!(body["schema"], "omnigate.site-visit-payment.v1");
    assert_eq!(body["ok"], true);
    assert_eq!(body["site_name"], SITE_NAME);
    assert_eq!(body["crab_url"], "crab://ron7");
    assert_eq!(body["action"], "site_visit");
    assert_eq!(body["asset"], "roc");
    assert_eq!(body["amount_minor"], "10");
    assert_eq!(body["payer_account"], "acct_visitor_b");
    assert_eq!(body["recipient_account"], "acct_dev");
    assert_eq!(body["nonce"], 8);
    assert_eq!(body["txid"], "tx_site_visit_test");
    assert_eq!(body["receipt_hash"], RECEIPT_HASH);

    let receipt = &body["wallet_receipt"];
    assert_eq!(receipt["op"], "transfer");
    assert_eq!(receipt["from"], "acct_visitor_b");
    assert_eq!(receipt["to"], "acct_dev");
    assert_eq!(receipt["asset"], "roc");
    assert_eq!(receipt["amount_minor"], "10");
    assert_eq!(receipt["nonce"], 8);
    assert_eq!(receipt["idem"], "site-visit-pay-test");
    assert_eq!(receipt["ledger_root"], LEDGER_ROOT);
    assert_eq!(receipt["receipt_hash"], RECEIPT_HASH);

    assert_eq!(body["receipt"]["kind"], "site_visit");
    assert_eq!(body["receipt"]["wallet_txid"], "tx_site_visit_test");
    assert_eq!(body["receipt"]["wallet_receipt_hash"], RECEIPT_HASH);
    assert_eq!(body["receipt"]["manifest_cid"], SITE_MANIFEST_CID);
    assert_eq!(body["receipt"]["root_document_cid"], ROOT_DOCUMENT_CID);

    assert_eq!(stack.transfer_attempts.load(Ordering::SeqCst), 2);

    clear_site_visit_env();
}

#[tokio::test]
async fn quote_rejects_recipient_mismatch() {
    let _guard = ENV_LOCK.lock().await;
    let stack = start_test_stack().await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "{}/v1/sites/{SITE_NAME}/visit/quote",
            stack.omnigate_base_url
        ))
        .header("authorization", "Bearer dev")
        .header("content-type", "application/json")
        .header("x-ron-passport", "passport:main:visitor-b")
        .header("x-ron-wallet-account", "acct_visitor_b")
        .json(&json!({
            "site_name": SITE_NAME,
            "payer_account": "acct_visitor_b",
            "recipient_account": "acct_wrong",
            "max_amount_minor": "10"
        }))
        .send()
        .await
        .expect("quote mismatch response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = response.json().await.expect("problem JSON");
    assert_eq!(body["code"], "site_visit_recipient_mismatch");
    assert_eq!(body["reason"], "recipient_mismatch");
    assert_eq!(stack.transfer_attempts.load(Ordering::SeqCst), 0);

    clear_site_visit_env();
}

async fn start_test_stack() -> TestStack {
    clear_site_visit_env();

    let index_addr = start_dummy_index().await;
    let storage_addr = start_dummy_storage().await;
    let wallet_state = WalletFixtureState {
        transfer_attempts: Arc::new(AtomicUsize::new(0)),
    };
    let transfer_attempts = wallet_state.transfer_attempts.clone();
    let wallet_addr = start_dummy_wallet(wallet_state).await;

    std::env::set_var("OMNIGATE_INDEX_BASE_URL", format!("http://{index_addr}"));
    std::env::set_var(
        "OMNIGATE_STORAGE_BASE_URL",
        format!("http://{storage_addr}"),
    );
    std::env::set_var("OMNIGATE_WALLET_BASE_URL", format!("http://{wallet_addr}"));
    std::env::set_var("OMNIGATE_WALLET_BEARER", "dev");
    std::env::set_var("OMNIGATE_SITE_VISIT_PRICE_MINOR", "10");

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());
    let omnigate_addr = spawn_router(router).await;
    let omnigate_base_url = format!("http://{omnigate_addr}");

    wait_for_ok(format!("{omnigate_base_url}/v1/ping")).await;

    TestStack {
        omnigate_base_url,
        transfer_attempts,
    }
}

async fn start_dummy_index() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn site_pointer(Path(name): Path<String>) -> (StatusCode, Json<Value>) {
        if name != SITE_NAME {
            return (
                StatusCode::NOT_FOUND,
                Json(json!(ErrorBody {
                    message: "site not found",
                    code: "not_found",
                })),
            );
        }

        (
            StatusCode::OK,
            Json(json!({
                "version": 1,
                "name": SITE_NAME,
                "manifest_cid": SITE_MANIFEST_CID,
                "owner_passport_subject": "passport:main:dev",
                "owner_wallet_account": "acct_dev",
                "updated_at_ms": 1_775_000_000_000_u64
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/index/sites/:name/manifest", get(site_pointer));

    let addr = spawn_router(router).await;
    wait_for_ok(format!("http://{addr}/healthz")).await;
    addr
}

async fn start_dummy_storage() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn object(Path(cid): Path<String>) -> (StatusCode, Json<Value>) {
        if cid != SITE_MANIFEST_CID {
            return (
                StatusCode::NOT_FOUND,
                Json(json!(ErrorBody {
                    message: "object not found",
                    code: "not_found",
                })),
            );
        }

        (
            StatusCode::OK,
            Json(json!({
                "version": 1,
                "site_name": SITE_NAME,
                "root_document_cid": ROOT_DOCUMENT_CID,
                "asset_map": {},
                "route_map": {},
                "owner": {
                    "passport_subject": "passport:main:dev",
                    "wallet_account": "acct_dev"
                },
                "payout": {
                    "default_action": "site_visit",
                    "recipient_account": "acct_dev",
                    "splits": []
                },
                "metadata": {
                    "title": "Reference Graph Smoke Site",
                    "description": "site_visit test fixture",
                    "tags": ["smoke", "site_visit"]
                },
                "provenance": {},
                "storage": {},
                "receipts": []
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/o/:cid", get(object));

    let addr = spawn_router(router).await;
    wait_for_ok(format!("http://{addr}/healthz")).await;
    addr
}

async fn start_dummy_wallet(state: WalletFixtureState) -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn transfer(
        State(state): State<WalletFixtureState>,
        headers: HeaderMap,
        body: Bytes,
    ) -> (StatusCode, Json<Value>) {
        let attempt = state.transfer_attempts.fetch_add(1, Ordering::SeqCst) + 1;
        let request: Value = serde_json::from_slice(&body).expect("wallet transfer JSON");

        assert_eq!(
            header_value(&headers, "authorization").as_deref(),
            Some("Bearer dev")
        );
        assert_eq!(
            header_value(&headers, "idempotency-key").as_deref(),
            Some("site-visit-pay-test")
        );
        assert_eq!(request["from"], "acct_visitor_b");
        assert_eq!(request["to"], "acct_dev");
        assert_eq!(request["asset"], "roc");
        assert_eq!(request["amount_minor"], "10");
        assert_eq!(request["idempotency_key"], "site-visit-pay-test");
        assert_eq!(request["memo"], "crablink site_visit crab://ron7");

        let nonce = request["nonce"].as_u64().expect("nonce u64");

        if attempt == 1 {
            assert_eq!(nonce, 1);
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "code": "NONCE_CONFLICT",
                    "message": "nonce conflict: expected 8",
                    "retryable": false
                })),
            );
        }

        assert_eq!(nonce, 8);

        (
            StatusCode::OK,
            Json(json!({
                "txid": "tx_site_visit_test",
                "op": "transfer",
                "from": "acct_visitor_b",
                "to": "acct_dev",
                "asset": "roc",
                "amount_minor": "10",
                "nonce": 8,
                "idem": "site-visit-pay-test",
                "ts": 1_775_000_000_001_u64,
                "ledger_seq_start": 11,
                "ledger_seq_end": 12,
                "ledger_root": LEDGER_ROOT,
                "receipt_hash": RECEIPT_HASH
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/transfer", post(transfer))
        .with_state(state);

    let addr = spawn_router(router).await;
    wait_for_ok(format!("http://{addr}/healthz")).await;
    addr
}

async fn spawn_router(router: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener");
    let addr = listener.local_addr().expect("test listener local addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve test router");
    });

    addr
}

async fn wait_for_ok(url: String) {
    let client = reqwest::Client::new();

    for _ in 0..80 {
        if let Ok(response) = client.get(&url).send().await {
            if response.status().is_success() {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    panic!("service did not become healthy at {url}");
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn clear_site_visit_env() {
    std::env::remove_var("OMNIGATE_INDEX_BASE_URL");
    std::env::remove_var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL");
    std::env::remove_var("OMNIGATE_STORAGE_BASE_URL");
    std::env::remove_var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL");
    std::env::remove_var("OMNIGATE_WALLET_BASE_URL");
    std::env::remove_var("OMNIGATE_WALLET_BEARER");
    std::env::remove_var("OMNIGATE_SITE_VISIT_PRICE_MINOR");
    std::env::remove_var("OMNIGATE_SITE_VISIT_NONCE");
}
