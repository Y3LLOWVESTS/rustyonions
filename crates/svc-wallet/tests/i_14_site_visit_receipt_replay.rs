//! RO:WHAT — Wallet-level regression tests for paid site_visit transfer receipts.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/GOV. Paid creator access must replay through wallet/ledger truth.
//! RO:INTERACTS — svc_wallet ledger adapter, ron-ledger memory storage, receipt DTOs.
//! RO:INVARIANTS — Visitor B decreases; Creator A increases; receipt binds txid/op/amount/nonce/root/hash.
//! RO:METRICS — none; HTTP route metrics are covered by svc-wallet black-box tests.
//! RO:CONFIG — uses WalletConfig::default with in-memory amnesia-safe ledger.
//! RO:SECURITY — no bearer tokens or private keys; tests only local economic invariants.
//! RO:TEST — cargo test -p svc-wallet --test i_14_site_visit_receipt_replay.

mod harness;

use svc_wallet::{dto::responses::WalletOp, errors::WalletErrorCode};

const VISITOR_ACCOUNT: &str = "acct_visitor_b";
const CREATOR_ACCOUNT: &str = "acct_dev";
const SITE_VISIT_AMOUNT_MINOR: u128 = 10;

#[test]
fn site_visit_transfer_changes_balances_and_returns_wallet_receipt_contract() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(
        &client,
        &cfg,
        VISITOR_ACCOUNT,
        1_000,
        "site_visit_replay_issue_visitor_b",
    );
    harness::issue_to(
        &client,
        &cfg,
        CREATOR_ACCOUNT,
        1_751,
        "site_visit_replay_issue_creator_a",
    );

    let visitor_before = harness::balance_of(&client, &cfg, VISITOR_ACCOUNT);
    let creator_before = harness::balance_of(&client, &cfg, CREATOR_ACCOUNT);

    let receipt = client
        .transfer(
            &cfg,
            &harness::transfer_req(VISITOR_ACCOUNT, CREATOR_ACCOUNT, SITE_VISIT_AMOUNT_MINOR, 1),
            "site_visit:crab://ron7:acct_visitor_b:1",
        )
        .expect("site_visit transfer should commit through wallet ledger adapter");

    let visitor_after = harness::balance_of(&client, &cfg, VISITOR_ACCOUNT);
    let creator_after = harness::balance_of(&client, &cfg, CREATOR_ACCOUNT);

    assert_eq!(visitor_after, visitor_before - SITE_VISIT_AMOUNT_MINOR);
    assert_eq!(creator_after, creator_before + SITE_VISIT_AMOUNT_MINOR);

    assert_eq!(receipt.op, WalletOp::Transfer);
    assert_eq!(receipt.from.as_deref(), Some(VISITOR_ACCOUNT));
    assert_eq!(receipt.to.as_deref(), Some(CREATOR_ACCOUNT));
    assert_eq!(receipt.asset, "roc");
    assert_eq!(receipt.amount_minor.get(), SITE_VISIT_AMOUNT_MINOR);
    assert_eq!(receipt.nonce, Some(1));
    assert_eq!(receipt.idem, "site_visit:crab://ron7:acct_visitor_b:1");

    assert!(
        receipt.txid.starts_with("tx_"),
        "wallet txid should be stable structured proof, got {}",
        receipt.txid
    );
    assert_b3_hash(&receipt.receipt_hash);
    assert_ledger_root(&receipt.ledger_root);

    let start = receipt
        .ledger_seq_start
        .expect("site_visit receipt should include ledger start sequence");
    let end = receipt
        .ledger_seq_end
        .expect("site_visit receipt should include ledger end sequence");

    assert!(
        start <= end,
        "ledger sequence bounds must be monotonic: start={start}, end={end}"
    );
}

#[test]
fn unfunded_site_visit_rejects_without_crediting_creator() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(
        &client,
        &cfg,
        CREATOR_ACCOUNT,
        1_751,
        "site_visit_replay_issue_creator_unfunded_case",
    );

    let visitor_before = harness::balance_of(&client, &cfg, VISITOR_ACCOUNT);
    let creator_before = harness::balance_of(&client, &cfg, CREATOR_ACCOUNT);

    let err = client
        .transfer(
            &cfg,
            &harness::transfer_req(VISITOR_ACCOUNT, CREATOR_ACCOUNT, SITE_VISIT_AMOUNT_MINOR, 1),
            "site_visit:crab://ron7:acct_visitor_b:unfunded",
        )
        .expect_err("unfunded site_visit must reject");

    assert_eq!(err.code, WalletErrorCode::InsufficientFunds);
    assert_eq!(
        harness::balance_of(&client, &cfg, VISITOR_ACCOUNT),
        visitor_before
    );
    assert_eq!(
        harness::balance_of(&client, &cfg, CREATOR_ACCOUNT),
        creator_before
    );
}

#[test]
fn site_visit_receipt_hash_is_canonical_b3_and_serializes_as_string_amount() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(
        &client,
        &cfg,
        VISITOR_ACCOUNT,
        100,
        "site_visit_receipt_shape_issue",
    );

    let receipt = client
        .transfer(
            &cfg,
            &harness::transfer_req(VISITOR_ACCOUNT, CREATOR_ACCOUNT, SITE_VISIT_AMOUNT_MINOR, 1),
            "site_visit:receipt-shape",
        )
        .expect("site_visit receipt shape transfer should commit");

    let encoded = serde_json::to_string(&receipt).expect("receipt should serialize");

    assert!(encoded.contains(r#""op":"transfer""#));
    assert!(encoded.contains(r#""amount_minor":"10""#));
    assert!(!encoded.contains(r#""amount_minor":10"#));
    assert!(encoded.contains(r#""receipt_hash":"b3:"#));
    assert_b3_hash(&receipt.receipt_hash);
}

fn assert_b3_hash(value: &str) {
    assert!(
        value.starts_with("b3:"),
        "expected b3:<64 lowercase hex>, got {value}"
    );

    let hex = &value[3..];
    assert_eq!(hex.len(), 64, "expected 64 hex chars, got {value}");
    assert!(
        hex.chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "expected lowercase hex receipt hash, got {value}"
    );
}

fn assert_ledger_root(value: &str) {
    assert_eq!(
        value.len(),
        64,
        "ledger_root should be 64 lowercase hex chars"
    );
    assert!(
        value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "ledger_root should be lowercase hex, got {value}"
    );
}
