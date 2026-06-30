//! RO:WHAT — Internal ROC Beta Phase 2 paid-action idempotency tests for svc-wallet.
//! RO:WHY — Internal ROC Beta Phase 2; Concerns: ECON/RES/SEC/GOV. Paid access retries must replay accepted wallet receipts or safely conflict without mutating value twice.
//! RO:INTERACTS — IdempotencyStore, LocalLedgerClient, wallet receipt DTOs, request_fingerprint.
//! RO:INVARIANTS — svc-wallet remains mutation front-door; idempotency_key is retry identity only; same request replays same receipt; different body conflicts; cancelled prepare/quote does not mutate.
//! RO:METRICS — none; route metrics remain covered by HTTP tests.
//! RO:CONFIG — WalletConfig::default idempotency TTL, asset=roc, amnesia=true.
//! RO:SECURITY — no bearer tokens, private keys, bridge, staking, liquidity, exchange-facing, or external settlement behavior.
//! RO:TEST — cargo test -p svc-wallet --test internal_roc_beta_phase2_paid_action_idempotency.

mod harness;

use svc_wallet::{
    dto::responses::{Receipt, WalletOp},
    errors::WalletErrorCode,
    idem::store::IdempotencyStore,
    util::blake3_receipt::request_fingerprint,
};

const VIEWER: &str = "acct_phase2_idem_viewer";
const CREATOR: &str = "acct_phase2_idem_creator";
const ESCROW: &str = "escrow_phase2_idem_paid_post";

#[test]
fn accepted_paid_action_retry_replays_receipts_without_second_mutation() {
    let cfg = harness::cfg();
    let client = harness::client();
    let store = IdempotencyStore::new(cfg.idempotency_ttl());

    harness::issue_to(&client, &cfg, VIEWER, 100, "phase2_idem_seed_viewer");

    let hold_req = harness::transfer_req(VIEWER, ESCROW, 30, 1);
    let hold_key = "phase2_paid_post_hold";
    let hold_fingerprint =
        request_fingerprint(WalletOp::Hold, &hold_req).expect("hold fingerprint");

    assert!(store
        .lookup(hold_key, &hold_fingerprint, 1_000)
        .expect("first hold lookup should not fail")
        .is_none());

    let hold_receipt = client
        .hold(&cfg, &hold_req, hold_key)
        .expect("hold mutation should succeed through wallet ledger adapter");
    store.insert(
        hold_key.to_string(),
        hold_fingerprint.clone(),
        hold_receipt.clone(),
        1_000,
    );

    assert_eq!(harness::balance_of(&client, &cfg, VIEWER), 70);
    assert_eq!(harness::balance_of(&client, &cfg, ESCROW), 30);
    assert_eq!(harness::balance_of(&client, &cfg, CREATOR), 0);

    let replayed_hold = store
        .lookup(hold_key, &hold_fingerprint, 1_001)
        .expect("same hold lookup should not fail")
        .expect("same hold fingerprint should replay original receipt");

    assert_same_replayed_receipt(&replayed_hold, &hold_receipt, WalletOp::Hold);
    assert_eq!(
        harness::balance_of(&client, &cfg, VIEWER),
        70,
        "idempotent hold replay must not debit viewer twice"
    );
    assert_eq!(
        harness::balance_of(&client, &cfg, ESCROW),
        30,
        "idempotent hold replay must not credit escrow twice"
    );

    let capture_req = harness::transfer_req(ESCROW, CREATOR, 30, 1);
    let capture_key = "phase2_paid_post_capture";
    let capture_fingerprint =
        request_fingerprint(WalletOp::Capture, &capture_req).expect("capture fingerprint");

    let capture_receipt = client
        .capture(&cfg, &capture_req, capture_key)
        .expect("capture mutation should succeed through wallet ledger adapter");
    store.insert(
        capture_key.to_string(),
        capture_fingerprint.clone(),
        capture_receipt.clone(),
        1_002,
    );

    assert_eq!(harness::balance_of(&client, &cfg, VIEWER), 70);
    assert_eq!(harness::balance_of(&client, &cfg, ESCROW), 0);
    assert_eq!(harness::balance_of(&client, &cfg, CREATOR), 30);

    let replayed_capture = store
        .lookup(capture_key, &capture_fingerprint, 1_003)
        .expect("same capture lookup should not fail")
        .expect("same capture fingerprint should replay original receipt");

    assert_same_replayed_receipt(&replayed_capture, &capture_receipt, WalletOp::Capture);
    assert_eq!(
        harness::balance_of(&client, &cfg, ESCROW),
        0,
        "idempotent capture replay must not debit escrow twice"
    );
    assert_eq!(
        harness::balance_of(&client, &cfg, CREATOR),
        30,
        "idempotent capture replay must not credit creator twice"
    );
}

#[test]
fn same_idempotency_key_with_different_paid_action_body_conflicts() {
    let cfg = harness::cfg();
    let store = IdempotencyStore::new(cfg.idempotency_ttl());
    let stored_receipt = harness::dummy_receipt("tx_phase2_idem_conflict", "phase2_paid_post_hold");

    let first_req = harness::transfer_req(VIEWER, ESCROW, 30, 1);
    let second_req = harness::transfer_req(VIEWER, ESCROW, 31, 1);

    let first_fingerprint =
        request_fingerprint(WalletOp::Hold, &first_req).expect("first fingerprint");
    let second_fingerprint =
        request_fingerprint(WalletOp::Hold, &second_req).expect("second fingerprint");

    store.insert(
        "phase2_paid_post_hold".to_string(),
        first_fingerprint,
        stored_receipt,
        1_000,
    );

    let err = store
        .lookup("phase2_paid_post_hold", &second_fingerprint, 1_001)
        .expect_err("same idempotency key with different body must conflict");

    assert_eq!(err.code, WalletErrorCode::IdempotencyConflict);
}

#[test]
fn cancelled_paid_action_before_mutation_leaves_no_receipt_or_balance_change() {
    let cfg = harness::cfg();
    let client = harness::client();
    let store = IdempotencyStore::new(cfg.idempotency_ttl());

    harness::issue_to(&client, &cfg, VIEWER, 50, "phase2_cancel_seed_viewer");

    let hold_req = harness::transfer_req(VIEWER, ESCROW, 25, 1);
    let hold_fingerprint =
        request_fingerprint(WalletOp::Hold, &hold_req).expect("hold fingerprint");

    // This models prepare/quote followed by explicit user cancellation before
    // the wallet mutation is submitted. No receipt is inserted and no ledger
    // operation runs.
    assert!(store
        .lookup("phase2_cancelled_hold", &hold_fingerprint, 1_000)
        .expect("cancelled lookup should not fail")
        .is_none());

    assert_eq!(harness::balance_of(&client, &cfg, VIEWER), 50);
    assert_eq!(harness::balance_of(&client, &cfg, ESCROW), 0);
    assert_eq!(harness::balance_of(&client, &cfg, CREATOR), 0);
}

fn assert_same_replayed_receipt(replayed: &Receipt, original: &Receipt, op: WalletOp) {
    assert_eq!(replayed, original);
    assert_eq!(replayed.op, op);
    assert_eq!(replayed.settlement_status.as_str(), "accepted");
    assert!(replayed.txid.starts_with("tx_"));
    assert!(replayed.receipt_hash.starts_with("b3:"));

    let encoded = serde_json::to_string(replayed).expect("receipt should encode");
    for forbidden in [
        "epoch_included",
        "finalized",
        "anchored",
        "bridge_txid",
        "staking_position_id",
        "outside_settlement_id",
        "liquidity_pool_id",
        "cache_unlock_authority",
        "client_finality_claim",
        "silent_spend",
        "fake_receipt",
        "fake_balance",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "wallet replay receipt must not smuggle forbidden authority field `{forbidden}`"
        );
    }
}
