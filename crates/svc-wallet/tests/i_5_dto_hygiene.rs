//! RO:WHAT — DTO hygiene and arithmetic-boundary tests.
//! RO:WHY  — Pillar 12; Concerns: DX/SEC/ECON. Malformed JSON must fail before auth/policy/ledger work.
//! RO:INTERACTS — dto::requests and WalletConfig validation.
//! RO:INVARIANTS — deny_unknown_fields; no floats; amount > 0; memo/header-visible text is bounded.
//! RO:METRICS — future wallet_rejects_total{reason="BAD_REQUEST"}.
//! RO:CONFIG — default asset and amount limits.
//! RO:SECURITY — rejects hidden/ambient auth fields in JSON.
//! RO:TEST — cargo test -p svc-wallet --test i_5_dto_hygiene.

use svc_wallet::{
    config::WalletConfig,
    dto::requests::{AmountMinor, IssueRequest, TransferRequest},
};

#[test]
fn transfer_rejects_ambient_authorization_field() {
    let raw = r#"{
        "from":"acct_a",
        "to":"acct_b",
        "asset":"roc",
        "amount_minor":"1",
        "nonce":1,
        "authorization":"Bearer should-not-be-here"
    }"#;

    assert!(serde_json::from_str::<TransferRequest>(raw).is_err());
}

#[test]
fn amount_rejects_zero_and_float() {
    let zero = r#"{
        "from":"acct_a",
        "to":"acct_b",
        "asset":"roc",
        "amount_minor":"0",
        "nonce":1
    }"#;
    let float = r#"{
        "from":"acct_a",
        "to":"acct_b",
        "asset":"roc",
        "amount_minor":1.5,
        "nonce":1
    }"#;

    assert!(serde_json::from_str::<TransferRequest>(zero).is_err());
    assert!(serde_json::from_str::<TransferRequest>(float).is_err());
}

#[test]
fn issue_validation_rejects_wrong_asset() {
    let cfg = WalletConfig::default();
    let req = IssueRequest {
        to: "acct_a".to_string(),
        asset: "not_roc".to_string(),
        amount_minor: AmountMinor(1),
        idempotency_key: None,
        memo: None,
    };

    assert!(req.validate(&cfg).is_err());
}

#[test]
fn memo_with_control_character_is_rejected() {
    let cfg = WalletConfig::default();
    let req = IssueRequest {
        to: "acct_a".to_string(),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(1),
        idempotency_key: None,
        memo: Some("bad\nmemo".to_string()),
    };

    assert!(req.validate(&cfg).is_err());
}
