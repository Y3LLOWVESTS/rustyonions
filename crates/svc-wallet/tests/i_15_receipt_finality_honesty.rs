//! RO:WHAT — Receipt settlement-status honesty tests for svc-wallet.
//! RO:WHY — Wallet receipts may unlock paid content at accepted status, but
//! svc-wallet must not invent epoch inclusion, finality, anchoring, or chain
//! settlement claims.
//! RO:INTERACTS — LocalLedgerClient, Receipt, ReceiptSettlementStatus.
//! RO:INVARIANTS — wallet receipts expose accepted-only status; no fake
//! finalized/anchored/epoch-included labels.
//! RO:METRICS — none.
//! RO:CONFIG — uses default amnesia-safe wallet config.
//! RO:SECURITY — prevents fake finality from entering client-visible receipt DTOs.
//! RO:TEST — cargo test -p svc-wallet --test i_15_receipt_finality_honesty.

mod harness;

use serde_json::Value;
use svc_wallet::dto::responses::{Receipt, ReceiptSettlementStatus};

#[test]
fn wallet_mutation_receipts_are_accepted_only() {
    let cfg = harness::cfg();
    let client = harness::client();

    let issue = client
        .issue(
            &cfg,
            &harness::issue_req("acct_finality_alice", 100),
            "idem_finality_issue",
        )
        .expect("issue should succeed");
    assert_eq!(issue.settlement_status, ReceiptSettlementStatus::Accepted);

    let transfer = client
        .transfer(
            &cfg,
            &harness::transfer_req("acct_finality_alice", "acct_finality_bob", 40, 1),
            "idem_finality_transfer",
        )
        .expect("transfer should succeed");
    assert_eq!(
        transfer.settlement_status,
        ReceiptSettlementStatus::Accepted
    );

    let burn = client
        .burn(
            &cfg,
            &harness::burn_req("acct_finality_alice", 10, 2),
            "idem_finality_burn",
        )
        .expect("burn should succeed");
    assert_eq!(burn.settlement_status, ReceiptSettlementStatus::Accepted);
}

#[test]
fn receipt_status_serializes_as_accepted_and_not_future_finality() {
    let receipt = harness::dummy_receipt("tx_status_contract", "idem_status_contract");
    let encoded = serde_json::to_string(&receipt).expect("receipt should serialize");

    assert!(encoded.contains(r#""settlement_status":"accepted""#));
    assert!(!encoded.contains("epoch_included"));
    assert!(!encoded.contains("finalized"));
    assert!(!encoded.contains("anchored"));
}

#[test]
fn future_finality_labels_are_not_accepted_by_wallet_receipt_dto() {
    let receipt = harness::dummy_receipt("tx_status_reject", "idem_status_reject");
    let mut value = serde_json::to_value(&receipt).expect("receipt should convert to JSON");

    let Value::Object(ref mut object) = value else {
        panic!("receipt JSON should be an object");
    };
    object.insert(
        "settlement_status".to_string(),
        Value::String("finalized".to_string()),
    );

    let parsed = serde_json::from_value::<Receipt>(value);
    assert!(
        parsed.is_err(),
        "svc-wallet must not deserialize fake finalized receipt status"
    );
}
