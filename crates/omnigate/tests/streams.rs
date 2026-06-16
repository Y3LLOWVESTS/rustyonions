//! RO:WHAT — Route-level tests for stream-lite sessions and receipt-gated latest segment access.
//! RO:WHY — Paid stream proof needs backend segment return after content_view wallet receipt, without fake local unlock.
//! RO:INTERACTS — omnigate stream routes, mock svc-wallet receipt route.
//! RO:INVARIANTS — missing receipts reject; matching wallet transfer receipt unlocks latest bounded segment.
//! RO:TEST — cargo test -p omnigate --test streams.

use std::net::SocketAddr;

use axum::{extract::Path, http::StatusCode, routing::get, Json, Router};
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const STREAM_ID: &str = "stream_test";
const ASSET_URL: &str =
    "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.stream";
const RECEIPT_HASH: &str = "b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

#[tokio::test]
async fn latest_segment_requires_stream_bound_wallet_receipt() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let wallet = start_wallet().await;
    std::env::set_var("OMNIGATE_WALLET_BASE_URL", format!("http://{wallet}"));
    std::env::set_var("OMNIGATE_WALLET_BEARER", "dev");

    let app = Router::new().nest("/v1/streams", omnigate::routes::v1::streams::router());
    let omnigate = spawn_router(app).await;
    let base = format!("http://{omnigate}");
    let client = reqwest::Client::new();

    let start = client
        .post(format!("{base}/v1/streams/{STREAM_ID}/start"))
        .header("x-ron-wallet-account", "acct_creator")
        .json(&json!({
            "asset_crab_url": ASSET_URL,
            "asset_cid": format!("b3:{HASH}"),
            "title": "Test stream",
            "creator_account": "acct_creator"
        }))
        .send()
        .await
        .expect("start response");

    assert_eq!(start.status(), StatusCode::OK);

    let put = client
        .post(format!("{base}/v1/streams/{STREAM_ID}/segments"))
        .header("x-ron-wallet-account", "acct_creator")
        .json(&json!({
            "asset_crab_url": ASSET_URL,
            "media_type": "text/plain",
            "text": "hello paid stream",
            "source": "test"
        }))
        .send()
        .await
        .expect("segment put response");

    assert_eq!(put.status(), StatusCode::OK);

    let denied = client
        .post(format!("{base}/v1/streams/{STREAM_ID}/segments/latest"))
        .json(&json!({
            "asset_crab_url": ASSET_URL,
            "payer_account": "acct_viewer",
            "recipient_account": "acct_creator"
        }))
        .send()
        .await
        .expect("denied latest response");

    assert_eq!(denied.status(), StatusCode::PAYMENT_REQUIRED);

    let latest = client
        .post(format!("{base}/v1/streams/{STREAM_ID}/segments/latest"))
        .json(&json!({
            "asset_crab_url": ASSET_URL,
            "payer_account": "acct_viewer",
            "recipient_account": "acct_creator",
            "txid": "tx_stream_test",
            "receipt_hash": RECEIPT_HASH,
            "amount_minor": "5"
        }))
        .send()
        .await
        .expect("latest response");

    assert_eq!(latest.status(), StatusCode::OK);
    let body: Value = latest.json().await.expect("latest JSON");
    assert_eq!(body["ok"], true);
    assert_eq!(body["segment"]["text"], "hello paid stream");
    assert_eq!(body["access"]["status"], "receipt_verified");

    clear_env();
}

async fn start_wallet() -> SocketAddr {
    async fn receipt(Path(txid): Path<String>) -> (StatusCode, Json<Value>) {
        if txid != "tx_stream_test" {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "code": "not_found",
                    "message": "receipt not found"
                })),
            );
        }

        (
            StatusCode::OK,
            Json(json!({
                "txid": "tx_stream_test",
                "op": "transfer",
                "from": "acct_viewer",
                "to": "acct_creator",
                "asset": "roc",
                "amount_minor": "5",
                "nonce": 1,
                "idem": expected_idem(),
                "ledger_root": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "receipt_hash": RECEIPT_HASH
            })),
        )
    }

    spawn_router(Router::new().route("/v1/tx/:txid", get(receipt))).await
}

async fn spawn_router(router: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test router");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("test server should run");
    });

    addr
}

fn expected_idem() -> String {
    format!("cl-view-pay:{}:{}", &HASH[..16], fnv1a_hex("acct_viewer"))
}

fn fnv1a_hex(value: &str) -> String {
    let mut hash = 0x811c9dc5u32;

    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }

    format!("{hash:08x}")
}

fn clear_env() {
    for key in ["OMNIGATE_WALLET_BASE_URL", "OMNIGATE_WALLET_BEARER"] {
        std::env::remove_var(key);
    }
}
