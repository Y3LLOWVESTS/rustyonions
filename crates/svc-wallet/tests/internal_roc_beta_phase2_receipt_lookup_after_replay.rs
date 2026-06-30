//! RO:WHAT — Internal ROC Beta Phase 2 receipt lookup stability tests for svc-wallet.
//! RO:WHY — Internal ROC Beta Phase 2; Concerns: ECON/RES/GOV. Backend-accepted receipts must be rehydratable as display/read evidence after replay without fabricating unknown receipts.
//! RO:INTERACTS — WalletState receipt book, LocalLedgerClient, wallet receipt DTOs.
//! RO:INVARIANTS — receipt lookup is read/display evidence only; accepted wallet receipts can be reloaded after replay; unknown receipts stay absent; no fake finality or cache-only entitlement.
//! RO:METRICS — none.
//! RO:CONFIG — WalletState::dev, WalletConfig::default.
//! RO:SECURITY — no secrets, private keys, bridge, staking, liquidity, exchange-facing, or external settlement behavior.
//! RO:TEST — cargo test -p svc-wallet --test internal_roc_beta_phase2_receipt_lookup_after_replay.

mod harness;

use svc_wallet::{
    dto::responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    routes::WalletState,
};

const VIEWER: &str = "acct_phase2_lookup_viewer";
const CREATOR: &str = "acct_phase2_lookup_creator";
const ESCROW: &str = "escrow_phase2_lookup_paid_article";

#[test]
fn backend_accepted_receipts_rehydrate_lookup_after_replay_without_fabrication() {
    let state = WalletState::dev().expect("dev wallet state should build");
    let cfg = state.config.as_ref();

    let issue = state
        .ledger
        .issue(cfg, &harness::issue_req(VIEWER, 100), "phase2_lookup_issue")
        .expect("seed issue should succeed");

    let hold = state
        .ledger
        .hold(
            cfg,
            &harness::transfer_req(VIEWER, ESCROW, 40, 1),
            "phase2_lookup_hold",
        )
        .expect("paid article hold should succeed");

    let capture = state
        .ledger
        .capture(
            cfg,
            &harness::transfer_req(ESCROW, CREATOR, 40, 1),
            "phase2_lookup_capture",
        )
        .expect("paid article capture should succeed");

    let accepted_history = vec![issue, hold, capture];

    for receipt in &accepted_history {
        state.remember_receipt(receipt.clone());
    }

    for receipt in &accepted_history {
        assert_receipt_lookup(&state, receipt);
    }

    assert!(
        state.receipt("tx_never_accepted").is_none(),
        "unknown receipt lookup must not fabricate receipt truth"
    );

    // Simulate replay/rehydration by creating a fresh wallet state and loading
    // only the accepted wallet/ledger receipts from history into the display
    // receipt book.
    let replayed_state = WalletState::dev().expect("replayed dev wallet state should build");

    for receipt in &accepted_history {
        assert!(
            replayed_state.receipt(&receipt.txid).is_none(),
            "fresh receipt book must not invent preloaded receipts"
        );
    }

    for receipt in &accepted_history {
        replayed_state.remember_receipt(receipt.clone());
    }

    for receipt in &accepted_history {
        assert_receipt_lookup(&replayed_state, receipt);
    }

    assert!(
        replayed_state.receipt("tx_never_accepted").is_none(),
        "replayed receipt book must not invent unknown receipt truth"
    );
}

fn assert_receipt_lookup(state: &WalletState, expected: &Receipt) {
    let found = state
        .receipt(&expected.txid)
        .expect("accepted receipt should be available after backend replay/rehydration");

    assert_eq!(&found, expected);
    assert_eq!(found.settlement_status, ReceiptSettlementStatus::Accepted);
    assert!(
        matches!(
            found.op,
            WalletOp::Issue | WalletOp::Hold | WalletOp::Capture
        ),
        "unexpected operation in Phase 2 receipt lookup proof: {:?}",
        found.op
    );
    assert!(found.receipt_hash.starts_with("b3:"));

    let encoded = serde_json::to_value(&found).expect("receipt should encode to JSON");
    assert_no_forbidden_authority(&encoded);
}

fn assert_no_forbidden_authority(value: &serde_json::Value) {
    let forbidden = [
        "epoch_included",
        "finalized",
        "anchored",
        "bridge_txid",
        "staking_position_id",
        "outside_settlement_id",
        "liquidity_pool_id",
        "cache_unlock_authority",
        "client_finality_claim",
        "paid_unlock_authority",
        "balance_truth",
        "receipt_truth",
        "silent_spend",
        "fake_receipt",
        "fake_balance",
    ];

    match value {
        serde_json::Value::Object(object) => {
            for key in object.keys() {
                assert!(
                    !forbidden.iter().any(|item| item == key),
                    "receipt lookup display evidence must not expose forbidden authority key `{key}`"
                );
            }

            for nested in object.values() {
                assert_no_forbidden_authority(nested);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                assert_no_forbidden_authority(nested);
            }
        }
        _ => {}
    }
}
