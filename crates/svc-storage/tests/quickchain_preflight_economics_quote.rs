//! RO:WHAT — QuickChain Phase-0 quote/economics preflight tests for svc-storage.
//! RO:WHY — Storage may quote paid-write cost, but quotes must not mutate wallet, ledger, accounting, storage, or chain state.
//! RO:INTERACTS — /paid/o/estimate route, policy::economics, checked-in configs/roc-economics.toml.
//! RO:INVARIANTS — quote-only; integer minor units only; no floats; no holds/captures/releases; no roots/finality.
//! RO:METRICS — none; price estimate endpoint is side-effect-free.
//! RO:CONFIG — RON_STORAGE_ROC_ECONOMICS_PATH and RON_STORAGE_ROC_ECONOMICS_ACTION.
//! RO:SECURITY — no wallet receipt, account balance, CID, ledger root, checkpoint, validator, or bridge authority in quote path.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_economics_quote.

use std::{fs, path::PathBuf, sync::Arc};

use axum::{
    body::{to_bytes, Body, Bytes},
    http::{HeaderMap, Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use svc_storage::{
    http::{extractors::AppState, server::build_router},
    storage::{MemoryStorage, Storage},
};
use tokio::sync::Mutex;
use tower::ServiceExt;

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    crate_dir()
        .join("../..")
        .canonicalize()
        .expect("workspace root should canonicalize")
}

fn checked_in_economics_path() -> PathBuf {
    repo_root().join("configs/roc-economics.toml")
}

fn read(relative: &str) -> String {
    fs::read_to_string(crate_dir().join(relative)).unwrap_or_else(|err| {
        panic!("failed to read {relative}: {err}");
    })
}

fn app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    let state = AppState { store };

    build_router().with_state(state)
}

fn request(method: Method, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .expect("request should build")
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, HeaderMap, Bytes) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");

    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");

    (status, headers, body)
}

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice::<Value>(bytes).expect("response body should be JSON")
}

fn clear_economics_env() {
    std::env::remove_var("RON_STORAGE_ROC_ECONOMICS_PATH");
    std::env::remove_var("RON_STORAGE_ROC_ECONOMICS_ACTION");
    std::env::remove_var("RON_STORAGE_PAID_WRITE_VERIFIER_MODE");
    std::env::remove_var("RON_STORAGE_PAID_SETTLEMENT_MODE");
    std::env::remove_var("RON_STORAGE_ACCOUNTING_EXPORT_MODE");
}

#[tokio::test]
async fn paid_estimate_is_quote_only_and_integer_minor_units() {
    let _guard = ENV_LOCK.lock().await;
    clear_economics_env();

    let app = app();

    let (status, _headers, body) = send(
        app.clone(),
        request(Method::GET, "/paid/o/estimate?bytes=42", Body::empty()),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let json = json_body(&body);
    assert_eq!(json["schema"], "svc-storage.paid-storage-estimate.v1");
    assert_eq!(json["route"], "/paid/o");
    assert_eq!(json["action"], "paid_storage_put");
    assert_eq!(json["asset"], "roc");
    assert_eq!(json["bytes"], 42);
    assert_eq!(json["amount_minor"], "42");
    assert_eq!(json["minimum_hold_minor"], "42");
    assert_eq!(json["pricing_mode"], "legacy");
    assert!(json["economics_policy_path"].is_null());

    for forbidden in [
        "cid",
        "paid",
        "payer",
        "escrow",
        "wallet_txid",
        "wallet_receipt_hash",
        "wallet_idem",
        "paid_context_idem",
        "settlement",
        "accounting_export",
        "usage_events",
        "balance_minor",
        "state_root",
        "receipt_root",
        "checkpoint",
        "validator",
        "bridge",
        "finality",
    ] {
        assert!(
            json.get(forbidden).is_none(),
            "/paid/o/estimate must remain quote-only and must not expose `{forbidden}`"
        );
    }

    let (bad_status, _headers, bad_body) = send(
        app,
        request(Method::GET, "/paid/o/estimate?bytes=1.5", Body::empty()),
    )
    .await;

    assert_eq!(bad_status, StatusCode::BAD_REQUEST);

    let bad_json = json_body(&bad_body);
    assert_eq!(bad_json["error"], "bad_request");
    assert!(
        bad_json["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("unsigned integer"),
        "float-like byte counts must reject because QuickChain money/quantity inputs are integer-only"
    );

    clear_economics_env();
}

#[tokio::test]
async fn checked_in_roc_economics_policy_quotes_without_wallet_or_ledger_mutation() {
    let _guard = ENV_LOCK.lock().await;
    clear_economics_env();

    let policy_path = checked_in_economics_path();
    assert!(
        policy_path.exists(),
        "root configs/roc-economics.toml must stay checked in for svc-storage economics tests"
    );

    std::env::set_var("RON_STORAGE_ROC_ECONOMICS_PATH", &policy_path);
    std::env::set_var("RON_STORAGE_ROC_ECONOMICS_ACTION", "paid_storage_put");

    let app = app();

    let (status, _headers, body) = send(
        app,
        request(Method::GET, "/paid/o/estimate?bytes=48", Body::empty()),
    )
    .await;

    assert_eq!(status, StatusCode::OK);

    let json = json_body(&body);
    assert_eq!(json["pricing_mode"], "roc-economics");
    assert_eq!(json["asset"], "roc");
    assert_eq!(json["bytes"], 48);
    assert_eq!(json["amount_minor"], "84");
    assert_eq!(json["minimum_hold_minor"], "84");

    let policy_path_from_response = json["economics_policy_path"]
        .as_str()
        .expect("explicit economics path should be reported for operator diagnostics");

    assert!(
        policy_path_from_response.ends_with("configs/roc-economics.toml"),
        "operator-visible economics path should point at the checked-in ROC policy"
    );

    clear_economics_env();
}

#[test]
fn quote_and_economics_source_do_not_smuggle_mutation_or_chain_authority() {
    let quote_route = read("src/http/routes/paid_estimate.rs").to_ascii_lowercase();
    let economics = read("src/policy/economics.rs").to_ascii_lowercase();
    let combined = format!("{quote_route}\n--- economics ---\n{economics}");

    for required in [
        "paid-storage-estimate",
        "side-effect-free",
        "integer minor units",
        "price_for",
        "parse::<u64>",
        "amount_minor.to_string()",
    ] {
        assert!(
            combined.contains(required),
            "quote/economics source should retain required quote-only marker `{required}`"
        );
    }

    for forbidden in [
        "walletreceipthttpclient",
        "walletsettlementhttpclient",
        "devheaderverifier",
        "app.store.put",
        "export_usage_events_from_env",
        "settle_after_success",
        "paidstoragesettlement",
        "svc_wallet",
        "ron_ledger",
        "ledger::",
        "capture(",
        "release(",
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "bridge_settled",
        "external settlement",
        "f64",
        "f32",
    ] {
        assert!(
            !combined.contains(forbidden),
            "quote/economics path must not smuggle mutation, chain authority, or float math via `{forbidden}`"
        );
    }
}
