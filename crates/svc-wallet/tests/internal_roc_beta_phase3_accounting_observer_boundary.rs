//! RO:WHAT — Internal ROC Beta Phase 3 svc-wallet accounting-observer boundary tests.
//! RO:WHY — Phase 3 Round 1 proves accounting/reward-plan material cannot become wallet mutation, receipt creation, balance truth, or payout execution.
//! RO:INTERACTS — LocalLedgerClient, NoopAccountingClient, wallet request DTOs, wallet receipts.
//! RO:INVARIANTS — svc-wallet remains the only mutation front-door; accounting remains observer-only; reward plans/snapshots cannot smuggle wallet operations.
//! RO:METRICS — none.
//! RO:CONFIG — WalletConfig::default, in-memory ron-ledger test adapter.
//! RO:SECURITY — no secrets; no bridge/staking/liquidity/external-settlement behavior.
//! RO:TEST — cargo test -p svc-wallet --test internal_roc_beta_phase3_accounting_observer_boundary.

mod harness;

use serde_json::{json, Value};
use svc_wallet::{
    accounting::client::{AccountingEvent, NoopAccountingClient},
    dto::{
        requests::{IssueRequest, TransferRequest},
        responses::{ReceiptSettlementStatus, WalletOp},
    },
};

const VIEWER: &str = "acct_phase3_wallet_viewer";
const CREATOR: &str = "acct_phase3_wallet_creator";

const FORBIDDEN_PHASE3_FIELDS: &[&str] = &[
    "reward_plan_id",
    "reward_plan_root",
    "accounting_snapshot_id",
    "accounting_snapshot_root",
    "source_event_class",
    "event_class",
    "proof_eligible",
    "ad_budgeted",
    "analytics_only",
    "metering",
    "payout_candidate_count",
    "payout_receipt_txid",
    "wallet_side_effect",
    "ledger_side_effect",
    "client_finality_claim",
    "bridge_txid",
    "solana_signature",
    "rox_settlement_id",
    "staking_position_id",
    "staking_yield_bps",
    "liquidity_pool_id",
    "exchange_order_id",
    "outside_settlement_claim",
];

fn assert_no_phase3_authority_keys(value: &Value) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert!(
                    !FORBIDDEN_PHASE3_FIELDS.contains(&key.as_str()),
                    "wallet receipt/request artifact must not expose phase3 authority key `{key}`"
                );
                assert_no_phase3_authority_keys(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_phase3_authority_keys(nested);
            }
        }
        _ => {}
    }
}

fn transfer_request_json() -> Value {
    json!({
        "from": VIEWER,
        "to": CREATOR,
        "asset": "roc",
        "amount_minor": "25",
        "nonce": 1
    })
}

fn issue_request_json() -> Value {
    json!({
        "to": VIEWER,
        "asset": "roc",
        "amount_minor": "100"
    })
}

#[test]
fn accounting_observer_events_do_not_mutate_wallet_or_ledger_balances() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(
        &client,
        &cfg,
        VIEWER,
        100,
        "phase3_seed_viewer_for_accounting_observer",
    );

    let before_viewer = harness::balance_of(&client, &cfg, VIEWER);
    let before_creator = harness::balance_of(&client, &cfg, CREATOR);

    let accounting = NoopAccountingClient;
    accounting.record(AccountingEvent {
        op: "economic_receipt",
        asset: "roc".to_owned(),
        amount_minor: 70,
    });

    accounting.record(AccountingEvent {
        op: "reward_plan_reference",
        asset: "roc".to_owned(),
        amount_minor: 999,
    });

    assert_eq!(
        harness::balance_of(&client, &cfg, VIEWER),
        before_viewer,
        "accounting observation must not debit payer"
    );
    assert_eq!(
        harness::balance_of(&client, &cfg, CREATOR),
        before_creator,
        "accounting observation must not credit recipient"
    );
}

#[test]
fn accepted_wallet_receipt_remains_wallet_ledger_derived_observation_source() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(
        &client,
        &cfg,
        VIEWER,
        100,
        "phase3_seed_viewer_for_real_wallet_receipt",
    );

    let receipt = client
        .transfer(
            &cfg,
            &harness::transfer_req(VIEWER, CREATOR, 25, 1),
            "phase3_wallet_ledger_receipt_transfer",
        )
        .expect("wallet transfer should commit through ron-ledger");

    assert_eq!(receipt.op, WalletOp::Transfer);
    assert_eq!(receipt.settlement_status, ReceiptSettlementStatus::Accepted);
    assert_eq!(receipt.amount_minor.get(), 25);
    assert_eq!(harness::balance_of(&client, &cfg, VIEWER), 75);
    assert_eq!(harness::balance_of(&client, &cfg, CREATOR), 25);

    let accounting = NoopAccountingClient;
    accounting.record(AccountingEvent {
        op: "economic_receipt_observed_after_wallet_ledger_truth",
        asset: receipt.asset.clone(),
        amount_minor: receipt.amount_minor.get(),
    });

    assert_eq!(
        harness::balance_of(&client, &cfg, VIEWER),
        75,
        "post-receipt accounting observation must not spend again"
    );
    assert_eq!(
        harness::balance_of(&client, &cfg, CREATOR),
        25,
        "post-receipt accounting observation must not credit again"
    );

    let receipt_json = serde_json::to_value(&receipt).expect("receipt should serialize");
    assert_no_phase3_authority_keys(&receipt_json);
}

#[test]
fn reward_plan_and_snapshot_fields_cannot_smuggle_wallet_mutation_requests() {
    for forbidden in FORBIDDEN_PHASE3_FIELDS {
        let mut transfer = transfer_request_json();
        transfer
            .as_object_mut()
            .expect("transfer request JSON should be object")
            .insert((*forbidden).to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<TransferRequest>(transfer).is_err(),
            "TransferRequest must reject phase3 smuggled field `{forbidden}`"
        );

        let mut issue = issue_request_json();
        issue
            .as_object_mut()
            .expect("issue request JSON should be object")
            .insert((*forbidden).to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<IssueRequest>(issue).is_err(),
            "IssueRequest must reject phase3 smuggled field `{forbidden}`"
        );
    }
}

#[test]
fn reward_plan_labels_alone_do_not_create_wallet_receipts_or_balances() {
    let cfg = harness::cfg();
    let client = harness::client();

    let before_viewer = harness::balance_of(&client, &cfg, VIEWER);
    let before_creator = harness::balance_of(&client, &cfg, CREATOR);

    let accounting = NoopAccountingClient;
    for op in [
        "reward_plan:phase3:test",
        "accounting_snapshot:phase3:test",
        "metering:raw-view",
        "proof_eligible:watch-time",
        "ad_budgeted:impression",
        "analytics_only:like",
    ] {
        accounting.record(AccountingEvent {
            op,
            asset: "roc".to_owned(),
            amount_minor: 1_000,
        });
    }

    assert_eq!(harness::balance_of(&client, &cfg, VIEWER), before_viewer);
    assert_eq!(harness::balance_of(&client, &cfg, CREATOR), before_creator);
}

#[test]
fn wallet_receipt_wire_shape_stays_backend_receipt_not_reward_plan() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(
        &client,
        &cfg,
        VIEWER,
        50,
        "phase3_seed_viewer_for_receipt_wire_shape",
    );

    let receipt = client
        .transfer(
            &cfg,
            &harness::transfer_req(VIEWER, CREATOR, 10, 1),
            "phase3_receipt_wire_shape_transfer",
        )
        .expect("wallet transfer should commit");

    let value = serde_json::to_value(&receipt).expect("receipt should serialize");
    assert_eq!(value["op"], json!("transfer"));
    assert_eq!(value["settlement_status"], json!("accepted"));
    assert!(
        value["receipt_hash"]
            .as_str()
            .expect("receipt_hash should be string")
            .starts_with("b3:"),
        "wallet receipt hash must remain backend-derived b3 receipt hash"
    );

    assert_no_phase3_authority_keys(&value);
}
