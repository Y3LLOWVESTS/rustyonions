//! RO:WHAT — No-doublespend invariant tests for nonce and idempotency gates.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. A debit nonce must be accepted at most once.
//! RO:INTERACTS — seq::nonce, idem::store, ledger::client.
//! RO:INVARIANTS — same account+nonce cannot commit twice; same idempotency key cannot change request body.
//! RO:METRICS — future wallet_nonce_conflicts_total and wallet_idem_replays_total.
//! RO:CONFIG — default idempotency TTL.
//! RO:SECURITY — account identifiers only.
//! RO:TEST — cargo test -p svc-wallet --test i_1_doublespend.

mod harness;

use svc_wallet::{
    dto::responses::WalletOp, errors::WalletErrorCode, idem::store::IdempotencyStore,
    seq::nonce::NonceTable, util::blake3_receipt::request_fingerprint,
};

#[test]
fn nonce_table_blocks_second_commit_for_same_account_nonce() {
    let cfg = harness::cfg();
    let client = harness::client();
    let nonces = NonceTable::default();

    harness::issue_to(&client, &cfg, "acct_a", 100, "idem_issue_ds");

    let first_reservation = nonces
        .reserve_strict("acct_a", 1)
        .expect("first nonce should reserve");
    client
        .transfer(
            &cfg,
            &harness::transfer_req("acct_a", "acct_b", 40, 1),
            "idem_transfer_ds_1",
        )
        .expect("first transfer should commit");
    first_reservation.commit();

    let second = nonces.reserve_strict("acct_a", 1);
    assert!(second.is_err(), "same nonce must not reserve twice");

    assert_eq!(harness::balance_of(&client, &cfg, "acct_a"), 60);
    assert_eq!(harness::balance_of(&client, &cfg, "acct_b"), 40);
}

#[test]
fn idempotency_store_replays_same_fingerprint_and_rejects_different_body() {
    let cfg = harness::cfg();
    let store = IdempotencyStore::new(cfg.idempotency_ttl());
    let req_1 = harness::transfer_req("acct_a", "acct_b", 1, 1);
    let req_2 = harness::transfer_req("acct_a", "acct_b", 2, 1);

    let fp_1 = request_fingerprint(WalletOp::Transfer, &req_1)
        .expect("fingerprint should be deterministic");
    let fp_2 = request_fingerprint(WalletOp::Transfer, &req_2)
        .expect("fingerprint should be deterministic");

    let receipt = harness::dummy_receipt("tx_idem_ds", "idem_ds");
    store.insert("idem_ds".to_string(), fp_1.clone(), receipt.clone(), 1_000);

    let replay = store
        .lookup("idem_ds", &fp_1, 1_001)
        .expect("same fingerprint lookup should succeed")
        .expect("same fingerprint should replay");
    assert_eq!(replay, receipt);

    let conflict = store.lookup("idem_ds", &fp_2, 1_002).unwrap_err();
    assert_eq!(conflict.code, WalletErrorCode::IdempotencyConflict);
}
