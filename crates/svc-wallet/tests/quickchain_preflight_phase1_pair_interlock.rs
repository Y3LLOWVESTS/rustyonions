//! RO:WHAT — QC-1A pair-level interlock tests for svc-wallet ↔ ron-accounting.
//! RO:WHY — svc-wallet may emit derivative accounting observations, but accounting must never authorize wallet commits.
//! RO:INTERACTS — WalletState HTTP routes, accounting::client, Cargo.toml, route source.
//! RO:INVARIANTS — wallet receipt first; accounting observation after receipt; no accounting/reward/root/finality authority in HTTP receipts.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents accounting/rewarder surfaces from becoming spend, receipt, balance, settlement, root, or payout authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_phase1_pair_interlock.

#![cfg(feature = "quickchain-preflight")]

use std::{fs, path::PathBuf};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use svc_wallet::routes::{self, WalletState};
use tower::ServiceExt;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    let path = crate_dir().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn app() -> Router {
    routes::router(WalletState::dev().expect("dev wallet state should build"))
}

fn json_post_request(path: &str, idempotency_key: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::AUTHORIZATION, "Bearer dev")
        .header(header::CONTENT_TYPE, "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(
            serde_json::to_vec(&body).expect("JSON request body should encode"),
        ))
        .expect("POST request should build")
}

async fn post_json(
    router: Router,
    path: &str,
    idempotency_key: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = router
        .oneshot(json_post_request(path, idempotency_key, body))
        .await
        .expect("router request should complete");

    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("response body should read");
    let value = serde_json::from_slice(&bytes).expect("response body should be JSON");

    (status, value)
}

fn assert_field_absent(value: &Value, field: &str) {
    let object = value.as_object().expect("receipt JSON should be an object");
    assert!(
        !object.contains_key(field),
        "live wallet receipt must not expose pair-level authority field: {field}"
    );
}

#[tokio::test]
async fn live_wallet_receipt_does_not_expose_accounting_reward_or_root_authority() {
    let router = app();

    let (status, receipt) = post_json(
        router,
        "/v1/issue",
        "idem_qc1a_pair_interlock_issue",
        json!({
            "to": "acct_qc1a_pair_interlock",
            "asset": "roc",
            "amount_minor": "123",
            "memo": null
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(receipt["op"], "issue");
    assert_eq!(receipt["settlement_status"], "accepted");

    for forbidden in [
        "accounting_status",
        "accounting_receipt",
        "accounting_root",
        "accounting_export_status",
        "reward_snapshot",
        "reward_root",
        "rewarder_decision",
        "payout_plan",
        "payout_intent",
        "payout_receipt",
        "wallet_execution_proof",
        "ledger_mutation_proof",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "validator_signature",
        "anchor_status",
        "external_settlement_status",
    ] {
        assert_field_absent(&receipt, forbidden);
    }
}

#[test]
fn accounting_observer_client_is_unit_noop_and_cannot_gate_wallet_commits() {
    let text = read("src/accounting/client.rs");
    let code = strip_line_comments(&text);

    assert!(
        text.contains("No-op accounting client")
            && text.contains("derivative counters only")
            && text.contains("never replaces ron-ledger truth"),
        "accounting observer seam must preserve derivative-only documentation"
    );
    assert!(
        code.contains("pub fn record(&self, _event: AccountingEvent) {}"),
        "accounting observer record must remain a unit-returning no-op in this boundary slice"
    );

    for forbidden in [
        "-> Result",
        "WalletResult",
        "HttpError",
        "reqwest",
        "ron_accounting::",
        "svc_rewarder::",
        "receipt_hash",
        "balance_minor",
        "available_balance",
        "spendable_balance",
        "operation_id",
        "account_sequence",
        "state_root",
        "receipt_root",
        "checkpoint_root",
        "settlement_status",
        "finality",
    ] {
        assert!(
            !code.contains(forbidden),
            "svc-wallet accounting observer seam must not contain authority or failure-gating fragment: {forbidden}"
        );
    }
}

#[test]
fn wallet_routes_observe_accounting_only_after_backend_receipt_memory() {
    for route in ["issue.rs", "transfer.rs", "burn.rs", "escrow.rs"] {
        let relative = format!("src/routes/v1/{route}");
        let code = strip_line_comments(&read(&relative));

        let record_pos = code
            .find("state.accounting.record(")
            .unwrap_or_else(|| panic!("{route} must emit a derivative accounting observation"));
        let remember_pos = code
            .find("state.remember_receipt(receipt.clone());")
            .unwrap_or_else(|| panic!("{route} must remember the backend-derived receipt"));

        assert!(
            remember_pos < record_pos,
            "{route} must remember backend-derived wallet receipt before emitting accounting observation"
        );

        let before_record = &code[..record_pos];
        assert!(
            !before_record.contains("accounting."),
            "{route} must not consult accounting before the wallet/ledger receipt exists"
        );
    }
}

#[test]
fn wallet_manifest_keeps_accounting_rewarder_and_external_settlement_out_of_authority_path() {
    let manifest = read("Cargo.toml");

    for forbidden in [
        "ron-accounting",
        "ron_accounting",
        "svc-rewarder",
        "svc_rewarder",
        "quickchain-consensus",
        "quickchain-validator",
        "solana",
        "solana-sdk",
        "solana-client",
        "ethers",
        "alloy",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "svc-wallet must not link pair/external settlement authority through Cargo dependency: {forbidden}"
        );
    }
}
