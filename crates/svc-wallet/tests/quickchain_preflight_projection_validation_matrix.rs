//! RO:WHAT — Validation matrix for svc-wallet's inert QuickChain preflight projection DTO.
//! RO:WHY — QuickChain Phase 0 needs strict projection boundaries before any
//! future root/proof work can safely consume wallet receipt data.
//! RO:INTERACTS — svc_wallet::quickchain, Receipt DTO, AmountMinor,
//! finalize_receipt, serde deny_unknown_fields.
//! RO:INVARIANTS — no roots/checkpoints/validators/anchors/settlement authority;
//! malformed chain_id/operation_id/receipt/projection fields reject; wallet
//! projection remains accepted-only and wallet-side.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only with quickchain-preflight.
//! RO:SECURITY — prevents malformed or future-authority-shaped projection data
//! from becoming spend, unlock, settlement, root, bridge, or chain authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_projection_validation_matrix.

#![cfg(feature = "quickchain-preflight")]

use serde_json::{json, Value};
use svc_wallet::{
    dto::{
        requests::AmountMinor,
        responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    },
    quickchain::{
        project_wallet_receipt_for_quickchain_preflight, QuickChainWalletReceiptProjection,
        QuickChainWalletReceiptProjectionContext, QuickChainWalletReceiptStatus,
        MAX_PREFLIGHT_CHAIN_ID_BYTES, MAX_PREFLIGHT_OPERATION_ID_BYTES,
        SVC_WALLET_QUICKCHAIN_RECEIPT_PROJECTION_SCHEMA,
    },
    util::blake3_receipt::finalize_receipt,
};

fn valid_wallet_receipt() -> Receipt {
    finalize_receipt(Receipt {
        txid: "tx_qc_projection_validation".to_string(),
        op: WalletOp::Transfer,
        from: Some("acct_qc_projection_alice".to_string()),
        to: Some("acct_qc_projection_bob".to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(17),
        nonce: Some(1),
        idem: "idem_qc_projection_validation".to_string(),
        ts: 1_777_309_851_000,
        ledger_seq_start: Some(20),
        ledger_seq_end: Some(21),
        ledger_root: "11".repeat(32),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: String::new(),
    })
    .expect("valid wallet receipt should finalize")
}

fn valid_context() -> QuickChainWalletReceiptProjectionContext {
    QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:transfer:projection-validation",
    )
    .expect("valid explicit projection context should build")
}

fn valid_projection_value() -> Value {
    let receipt = valid_wallet_receipt();
    let context = valid_context();
    let projection = project_wallet_receipt_for_quickchain_preflight(&receipt, &context)
        .expect("valid receipt should project");

    serde_json::to_value(projection).expect("projection should serialize")
}

fn assert_projection_rejects(receipt: Receipt, reason: &str) {
    let context = valid_context();

    assert!(
        project_wallet_receipt_for_quickchain_preflight(&receipt, &context).is_err(),
        "{reason}"
    );
}

#[test]
fn projection_context_rejects_malformed_chain_and_operation_identity() {
    assert!(
        QuickChainWalletReceiptProjectionContext::accepted("", "op:wallet:valid").is_err(),
        "chain_id must be explicit and nonempty"
    );

    assert!(
        QuickChainWalletReceiptProjectionContext::accepted(
            "a".repeat(MAX_PREFLIGHT_CHAIN_ID_BYTES + 1),
            "op:wallet:valid",
        )
        .is_err(),
        "chain_id must be bounded"
    );

    assert!(
        QuickChainWalletReceiptProjectionContext::accepted("roc dev", "op:wallet:valid").is_err(),
        "chain_id must reject spaces"
    );

    assert!(
        QuickChainWalletReceiptProjectionContext::accepted("roc/dev", "op:wallet:valid").is_err(),
        "chain_id must reject unsupported slash authority syntax"
    );

    assert!(
        QuickChainWalletReceiptProjectionContext::accepted("roc-dev", "").is_err(),
        "operation_id must be explicit and nonempty"
    );

    assert!(
        QuickChainWalletReceiptProjectionContext::accepted(
            "roc-dev",
            "a".repeat(MAX_PREFLIGHT_OPERATION_ID_BYTES + 1),
        )
        .is_err(),
        "operation_id must be bounded"
    );

    assert!(
        QuickChainWalletReceiptProjectionContext::accepted("roc-dev", "op wallet bad").is_err(),
        "operation_id must reject spaces"
    );

    assert!(
        QuickChainWalletReceiptProjectionContext::accepted("roc-dev", "op#bad").is_err(),
        "operation_id must reject unsupported authority punctuation"
    );

    let unknown_field = serde_json::from_value::<QuickChainWalletReceiptProjectionContext>(json!({
        "chain_id": "roc-dev",
        "operation_id": "op:wallet:valid",
        "settlement_status": "accepted",
        "state_root": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }));

    assert!(
        unknown_field.is_err(),
        "projection context DTO must reject unknown future root fields"
    );

    let future_finality =
        serde_json::from_value::<QuickChainWalletReceiptProjectionContext>(json!({
            "chain_id": "roc-dev",
            "operation_id": "op:wallet:valid",
            "settlement_status": "finalized"
        }));

    assert!(
        future_finality.is_err(),
        "svc-wallet projection context must not accept future finality labels"
    );
}

#[test]
fn projection_rejects_malformed_wallet_receipt_fields() {
    let mut wrong_asset = valid_wallet_receipt();
    wrong_asset.asset = "usd".to_string();
    assert_projection_rejects(
        wrong_asset,
        "projection must currently reject non-ROC asset receipts",
    );

    let mut bad_txid = valid_wallet_receipt();
    bad_txid.txid = "tx bad spaces".to_string();
    assert_projection_rejects(bad_txid, "projection must reject malformed wallet txids");

    let mut bad_from_account = valid_wallet_receipt();
    bad_from_account.from = Some("bad account".to_string());
    assert_projection_rejects(
        bad_from_account,
        "projection must reject malformed debit account ids",
    );

    let mut bad_to_account = valid_wallet_receipt();
    bad_to_account.to = Some("bad account".to_string());
    assert_projection_rejects(
        bad_to_account,
        "projection must reject malformed credit account ids",
    );

    let mut bad_idempotency_key = valid_wallet_receipt();
    bad_idempotency_key.idem = "bad idem with spaces".to_string();
    assert_projection_rejects(
        bad_idempotency_key,
        "projection must reject malformed idempotency keys",
    );

    let mut zero_timestamp = valid_wallet_receipt();
    zero_timestamp.ts = 0;
    assert_projection_rejects(
        zero_timestamp,
        "projection must reject zero produced_at_ms values",
    );

    let mut zero_sequence = valid_wallet_receipt();
    zero_sequence.ledger_seq_start = Some(0);
    zero_sequence.ledger_seq_end = Some(1);
    assert_projection_rejects(
        zero_sequence,
        "projection must reject zero ledger sequence starts",
    );

    let mut reversed_sequence = valid_wallet_receipt();
    reversed_sequence.ledger_seq_start = Some(22);
    reversed_sequence.ledger_seq_end = Some(21);
    assert_projection_rejects(
        reversed_sequence,
        "projection must reject reversed ledger sequence ranges",
    );

    let mut uppercase_legacy_root = valid_wallet_receipt();
    uppercase_legacy_root.ledger_root = "AA".repeat(32);
    assert_projection_rejects(
        uppercase_legacy_root,
        "projection must reject uppercase legacy ledger roots",
    );

    let mut short_legacy_root = valid_wallet_receipt();
    short_legacy_root.ledger_root = "1".repeat(63);
    assert_projection_rejects(
        short_legacy_root,
        "projection must reject incorrectly sized legacy ledger roots",
    );

    let mut uppercase_receipt_hash = valid_wallet_receipt();
    uppercase_receipt_hash.receipt_hash =
        "b3:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
    assert_projection_rejects(
        uppercase_receipt_hash,
        "projection must reject uppercase b3 receipt hashes",
    );
}

#[test]
fn projection_dto_rejects_unknown_future_authority_fields_on_deserialize() {
    let clean = valid_projection_value();

    for (field, value) in [
        (
            "state_root",
            json!("b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ),
        (
            "receipt_root",
            json!("b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        ),
        (
            "checkpoint",
            json!("b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"),
        ),
        ("anchor", json!("external-anchor-must-not-authorize")),
        ("validator", json!("validator_attacker")),
        ("finality", json!("anchored")),
        ("finalized", json!(true)),
        (
            "settlement_root",
            json!("b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
        ),
    ] {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("projection JSON should be an object")
            .insert(field.to_string(), value);

        assert!(
            serde_json::from_value::<QuickChainWalletReceiptProjection>(poisoned).is_err(),
            "projection DTO must reject unknown future authority field {field}"
        );
    }
}

#[test]
fn projection_dto_schema_and_status_remain_wallet_side_and_accepted_only() {
    let clean = valid_projection_value();

    let projection = serde_json::from_value::<QuickChainWalletReceiptProjection>(clean.clone())
        .expect("clean projection should deserialize");

    assert_eq!(
        projection.schema,
        SVC_WALLET_QUICKCHAIN_RECEIPT_PROJECTION_SCHEMA
    );
    assert_eq!(
        projection.settlement_status,
        QuickChainWalletReceiptStatus::Accepted
    );
    projection
        .validate()
        .expect("clean projection should validate");

    let mut canonical_chain_schema = clean.clone();
    canonical_chain_schema
        .as_object_mut()
        .expect("projection JSON should be an object")
        .insert("schema".to_string(), json!("quickchain.receipt.v1"));

    let dto = serde_json::from_value::<QuickChainWalletReceiptProjection>(canonical_chain_schema)
        .expect("schema is a string field, so serde should accept before semantic validation");

    assert!(
        dto.validate().is_err(),
        "svc-wallet projection must not masquerade as canonical QuickChain receipt DTO"
    );

    let mut future_status = clean;
    future_status
        .as_object_mut()
        .expect("projection JSON should be an object")
        .insert("settlement_status".to_string(), json!("finalized"));

    assert!(
        serde_json::from_value::<QuickChainWalletReceiptProjection>(future_status).is_err(),
        "svc-wallet projection must not accept future finality labels"
    );
}
