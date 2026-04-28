//! RO:WHAT — API contract tests for svc-wallet DTO and receipt wire shapes.
//! RO:WHY  — Pillar 12; Concerns: DX/SEC/ECON. Locks the public JSON contract before adding SDK clients.
//! RO:INTERACTS — dto::requests, dto::responses, serde_json.
//! RO:INVARIANTS — request DTOs deny unknown fields; amounts are string-encoded u128; receipts carry b3 hashes.
//! RO:METRICS — none.
//! RO:CONFIG — uses default wallet asset "roc".
//! RO:SECURITY — no Authorization/token fields are accepted in request DTOs.
//! RO:TEST — cargo test -p svc-wallet --test api_contract.

mod harness;

use svc_wallet::{
    config::WalletConfig,
    dto::{
        requests::{AmountMinor, BalanceQuery, IssueRequest, TransferRequest},
        responses::WalletOp,
    },
};

#[test]
fn transfer_request_rejects_unknown_fields() {
    let raw = r#"{
        "from":"acct_a",
        "to":"acct_b",
        "asset":"roc",
        "amount_minor":"40",
        "nonce":1,
        "authorization":"Bearer nope"
    }"#;

    let parsed = serde_json::from_str::<TransferRequest>(raw);
    assert!(parsed.is_err(), "unknown fields must be rejected");
}

#[test]
fn issue_request_amount_is_string_encoded() {
    let req = IssueRequest {
        to: "acct_a".to_string(),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(100),
        idempotency_key: None,
        memo: None,
    };

    let encoded = serde_json::to_string(&req).expect("issue request should serialize");
    assert!(encoded.contains(r#""amount_minor":"100""#));
    assert!(!encoded.contains(r#""amount_minor":100"#));
}

#[test]
fn balance_query_validates_account_and_asset() {
    let cfg = WalletConfig::default();
    let query = BalanceQuery {
        account: "acct_a".to_string(),
        asset: "roc".to_string(),
    };

    query
        .validate(&cfg)
        .expect("valid balance query should pass");

    let wrong_asset = BalanceQuery {
        account: "acct_a".to_string(),
        asset: "other".to_string(),
    };
    assert!(wrong_asset.validate(&cfg).is_err());
}

#[test]
fn receipt_wire_shape_uses_expected_operation_label() {
    let receipt = harness::dummy_receipt("tx_contract", "idem_contract");
    let encoded = serde_json::to_string(&receipt).expect("receipt should serialize");

    assert!(encoded.contains(r#""op":"transfer""#));
    assert!(encoded.contains(r#""receipt_hash":"b3:"#));
    assert_eq!(receipt.op, WalletOp::Transfer);
}
