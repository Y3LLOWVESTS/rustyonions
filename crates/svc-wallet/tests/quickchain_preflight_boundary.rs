//! RO:WHAT — QuickChain Phase-0 compatibility smoke for svc-wallet.
//! RO:WHY — svc-wallet is the ROC mutation front-door; it must compile against
//! ron-ledger's gated preflight surface without becoming chain authority.
//! RO:INTERACTS — svc_wallet::config and ron_ledger::quickchain behind the
//! quickchain-preflight feature.
//! RO:INVARIANTS — no roots, checkpoints, validators, settlement, anchors,
//! bridge logic, pruning, external-chain logic, fake balances, or fake receipts.
//! RO:METRICS — none.
//! RO:CONFIG — enabled only with --features quickchain-preflight.
//! RO:SECURITY — confirms QuickChain preflight is inert compatibility surface,
//! not live wallet runtime authority.
//! RO:TEST — cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_boundary.

#![cfg(feature = "quickchain-preflight")]

use ron_ledger::quickchain::{QuickChainAcceptedReplayBoundary, QuickChainAtomicState};
use svc_wallet::config::WalletConfig;

#[test]
fn wallet_quickchain_preflight_feature_keeps_default_wallet_posture() {
    let cfg = WalletConfig::default();

    assert_eq!(cfg.asset, "roc");
    assert!(cfg.amnesia);
    assert!(cfg.validate().is_ok());
}

#[test]
fn ron_ledger_quickchain_preflight_surface_is_inert_from_wallet_boundary() {
    let state = QuickChainAtomicState::new();

    assert_eq!(state.current_supply_minor(), 0);
    assert_eq!(state.operation_count(), 0);
    assert_eq!(state.next_ledger_sequence(), 1);
    assert_eq!(state.balance_minor("acct_alice"), 0);
    assert_eq!(state.held_minor("acct_alice"), 0);

    let boundary = state.accepted_replay_boundary();

    assert_eq!(boundary, QuickChainAcceptedReplayBoundary::empty());
    assert_eq!(boundary.operation_count(), 0);
    assert_eq!(boundary.next_ledger_sequence(), 1);
    assert_eq!(boundary.chain_id(), None);
}

#[test]
fn empty_accepted_replay_boundary_rebuilds_without_wallet_mutation() {
    let rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
        &[],
        QuickChainAcceptedReplayBoundary::empty(),
    )
    .expect("empty accepted replay should rebuild an empty preflight state");

    assert_eq!(rebuilt.current_supply_minor(), 0);
    assert_eq!(rebuilt.operation_count(), 0);
    assert_eq!(rebuilt.next_ledger_sequence(), 1);
    assert_eq!(
        rebuilt.accepted_replay_boundary(),
        QuickChainAcceptedReplayBoundary::empty()
    );
}

// --- svc-wallet receipt projection boundary tests ---

use svc_wallet::{
    dto::{
        requests::AmountMinor,
        responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    },
    quickchain::{
        project_wallet_receipt_for_quickchain_preflight, QuickChainWalletReceiptProjectionContext,
        QuickChainWalletReceiptStatus, SVC_WALLET_QUICKCHAIN_RECEIPT_PROJECTION_SCHEMA,
    },
    util::blake3_receipt::finalize_receipt,
};

fn dummy_wallet_receipt_for_projection() -> Receipt {
    finalize_receipt(Receipt {
        txid: "tx_projection_test".to_string(),
        op: WalletOp::Transfer,
        from: Some("acct_alice".to_string()),
        to: Some("acct_bob".to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(7),
        nonce: Some(1),
        idem: "idem_projection_test".to_string(),
        ts: 1_777_309_851_000,
        ledger_seq_start: Some(10),
        ledger_seq_end: Some(11),
        ledger_root: "00".repeat(32),
        settlement_status: ReceiptSettlementStatus::Accepted,
        receipt_hash: String::new(),
    })
    .expect("dummy wallet receipt should hash")
}

#[test]
fn wallet_receipt_projection_requires_explicit_operation_context() {
    let receipt = dummy_wallet_receipt_for_projection();
    let context = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:transfer:projection-test",
    )
    .expect("explicit preflight context should validate");

    let projection = project_wallet_receipt_for_quickchain_preflight(&receipt, &context)
        .expect("wallet receipt should project with explicit context");

    assert_eq!(
        projection.schema,
        SVC_WALLET_QUICKCHAIN_RECEIPT_PROJECTION_SCHEMA
    );
    assert_eq!(projection.chain_id, "roc-dev");
    assert_eq!(
        projection.operation_id,
        "op:wallet:transfer:projection-test"
    );
    assert_eq!(projection.txid, receipt.txid);
    assert_eq!(projection.op, WalletOp::Transfer);
    assert_eq!(projection.from.as_deref(), Some("acct_alice"));
    assert_eq!(projection.to.as_deref(), Some("acct_bob"));
    assert_eq!(projection.amount_minor.get(), 7);
    assert_eq!(projection.idempotency_key, "idem_projection_test");
    assert_eq!(projection.ledger_seq_start, 10);
    assert_eq!(projection.ledger_seq_end, 11);
    assert_eq!(projection.legacy_ledger_root, "00".repeat(32));
    assert_eq!(
        projection.settlement_status,
        QuickChainWalletReceiptStatus::Accepted
    );
    assert!(projection.receipt_hash.starts_with("b3:"));

    let encoded = serde_json::to_string(&projection).expect("projection should serialize");
    assert!(encoded.contains(r#""schema":"svc-wallet.quickchain-receipt-projection.v1""#));
    assert!(encoded.contains(r#""amount_minor":"7""#));
    assert!(encoded.contains(r#""settlement_status":"accepted""#));
    assert!(encoded.contains(r#""legacy_ledger_root":"#));
}

#[test]
fn wallet_receipt_projection_rejects_missing_operation_id() {
    let context = QuickChainWalletReceiptProjectionContext {
        chain_id: "roc-dev".to_string(),
        operation_id: String::new(),
        settlement_status: QuickChainWalletReceiptStatus::Accepted,
    };

    assert!(
        context.validate().is_err(),
        "operation_id must be explicit and nonempty"
    );
}

#[test]
fn wallet_receipt_projection_rejects_fake_receipt_hash() {
    let mut receipt = dummy_wallet_receipt_for_projection();
    receipt.receipt_hash = "not-a-b3-hash".to_string();

    let context = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:transfer:bad-hash",
    )
    .expect("explicit preflight context should validate");

    assert!(
        project_wallet_receipt_for_quickchain_preflight(&receipt, &context).is_err(),
        "projection must not accept fake receipt hashes"
    );
}

#[test]
fn wallet_receipt_projection_rejects_missing_ledger_sequence_pair() {
    let mut receipt = dummy_wallet_receipt_for_projection();
    receipt.ledger_seq_end = None;

    let context = QuickChainWalletReceiptProjectionContext::accepted(
        "roc-dev",
        "op:wallet:transfer:missing-seq",
    )
    .expect("explicit preflight context should validate");

    assert!(
        project_wallet_receipt_for_quickchain_preflight(&receipt, &context).is_err(),
        "projection must not silently invent missing ledger sequence data"
    );
}
