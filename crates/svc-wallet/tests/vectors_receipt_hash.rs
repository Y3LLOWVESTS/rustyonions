//! RO:WHAT — Receipt-hash vector tests for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: ECON/GOV/DX. Receipts are the client/auditor proof surface.
//! RO:INTERACTS — dto::responses and util::blake3_receipt.
//! RO:INVARIANTS — receipt_hash excludes itself; hash is stable; txid/request fingerprints are deterministic.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — hashes are integrity identifiers, not signatures.
//! RO:TEST — cargo test -p svc-wallet --test vectors_receipt_hash.

mod harness;

use svc_wallet::{
    dto::responses::WalletOp,
    util::blake3_receipt::{receipt_hash, request_fingerprint, txid_for},
};

#[test]
fn receipt_hash_is_stable_for_same_vector() {
    let receipt = harness::dummy_receipt("tx_vector", "idem_vector");

    let h1 = receipt_hash(&receipt).expect("receipt hash should compute");
    let h2 = receipt_hash(&receipt).expect("receipt hash should compute again");

    assert_eq!(h1, h2);
    assert_eq!(h1, receipt.receipt_hash);
    assert!(h1.starts_with("b3:"));
    assert_eq!(h1.len(), 67);
}

#[test]
fn receipt_hash_changes_when_amount_changes() {
    let receipt_1 = harness::dummy_receipt("tx_vector_amount_1", "idem_vector_amount");
    let mut receipt_2 = receipt_1.clone();
    receipt_2.amount_minor = svc_wallet::dto::requests::AmountMinor(2);
    receipt_2.receipt_hash.clear();

    let h1 = receipt_hash(&receipt_1).expect("receipt hash should compute");
    let h2 = receipt_hash(&receipt_2).expect("receipt hash should compute");

    assert_ne!(h1, h2);
}

#[test]
fn txid_and_request_fingerprint_are_deterministic() {
    let req = harness::transfer_req("acct_a", "acct_b", 40, 1);

    let txid_1 =
        txid_for(WalletOp::Transfer, "idem_vector_txid", &req).expect("txid should compute");
    let txid_2 =
        txid_for(WalletOp::Transfer, "idem_vector_txid", &req).expect("txid should compute again");

    let fp_1 = request_fingerprint(WalletOp::Transfer, &req).expect("fingerprint should compute");
    let fp_2 =
        request_fingerprint(WalletOp::Transfer, &req).expect("fingerprint should compute again");

    assert_eq!(txid_1, txid_2);
    assert_eq!(fp_1, fp_2);
    assert!(txid_1.starts_with("tx_"));
    assert!(fp_1.starts_with("b3:"));
}
