//! RO:WHAT — Internal ROC Beta Phase 3 Round 2 approved-payout execution boundary tests for svc-wallet.
//! RO:WHY — Proves approved payout intents execute only through svc-wallet's existing wallet/ledger issue path and return durable backend receipts without rewarder/accounting/policy/client authority.
//! RO:INTERACTS — LocalLedgerClient, IssueRequest, Receipt, NoopAccountingClient.
//! RO:INVARIANTS — svc-wallet remains the only mutation front-door; accounting observes after receipt truth only; no fake balance, fake receipt, fake finality, bridge, staking, liquidity, or external settlement.
//! RO:METRICS — none.
//! RO:CONFIG — WalletConfig::default, in-memory ron-ledger adapter.
//! RO:SECURITY — no secrets; no direct rewarder/accounting/policy ledger mutation.
//! RO:TEST — cargo test -p svc-wallet --test internal_roc_beta_phase3_approved_payout_execution_boundary.

mod harness;

use serde_json::{json, Value};
use svc_wallet::{
    accounting::client::{AccountingEvent, NoopAccountingClient},
    dto::{
        requests::{AmountMinor, IssueRequest},
        responses::{Receipt, ReceiptSettlementStatus, WalletOp},
    },
};

const CREATOR_A: &str = "acct_phase3_round2_creator_a";
const CREATOR_B: &str = "acct_phase3_round2_creator_b";

const FORBIDDEN_AUTHORITY_KEYS: &[&str] = &[
    "reward_plan_root",
    "accounting_snapshot_root",
    "policy_receipt",
    "rewarder_receipt",
    "accounting_receipt",
    "client_receipt",
    "balance_truth",
    "receipt_truth",
    "payout_execution_truth",
    "finality_truth",
    "client_finality_claim",
    "epoch_included",
    "finalized",
    "anchored",
    "bridge_txid",
    "solana_signature",
    "rox_settlement_id",
    "staking_position_id",
    "staking_yield_bps",
    "liquidity_pool_id",
    "exchange_order_id",
    "outside_settlement_claim",
    "cache_unlock_authority",
    "paid_unlock_authority",
    "silent_spend",
    "fake_balance",
    "fake_receipt",
];

fn approved_payout_request(account: &str, amount_minor: u128, plan_id: &str) -> IssueRequest {
    IssueRequest {
        to: account.to_owned(),
        asset: "roc".to_owned(),
        amount_minor: AmountMinor(amount_minor),
        idempotency_key: None,
        memo: Some(format!(
            "approved payout via svc-wallet only; plan_ref={plan_id}"
        )),
    }
}

fn assert_no_forbidden_authority_keys(value: &Value) {
    match value {
        Value::Object(object) => {
            for key in object.keys() {
                assert!(
                    !FORBIDDEN_AUTHORITY_KEYS.contains(&key.as_str()),
                    "wallet payout receipt must not expose forbidden authority key `{key}`"
                );
            }

            for nested in object.values() {
                assert_no_forbidden_authority_keys(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_forbidden_authority_keys(nested);
            }
        }
        _ => {}
    }
}

fn assert_accepted_payout_receipt(
    receipt: &Receipt,
    account: &str,
    amount_minor: u128,
    idem: &str,
) {
    assert_eq!(receipt.op, WalletOp::Issue);
    assert_eq!(receipt.from, None);
    assert_eq!(receipt.to.as_deref(), Some(account));
    assert_eq!(receipt.asset, "roc");
    assert_eq!(receipt.amount_minor.get(), amount_minor);
    assert_eq!(receipt.nonce, None);
    assert_eq!(receipt.idem, idem);
    assert_eq!(receipt.settlement_status, ReceiptSettlementStatus::Accepted);
    assert!(
        receipt.ledger_seq_start.is_some(),
        "approved payout receipt should carry backend ledger sequence context"
    );
    assert_eq!(
        receipt.ledger_seq_start, receipt.ledger_seq_end,
        "single approved payout issue should commit as a single ledger operation"
    );
    assert_eq!(
        receipt.ledger_root.len(),
        64,
        "ledger root should be 64 lowercase hex chars"
    );
    assert!(
        receipt
            .ledger_root
            .chars()
            .all(|ch| matches!(ch, '0'..='9' | 'a'..='f')),
        "ledger root should be lowercase hex"
    );
    assert!(
        receipt.receipt_hash.starts_with("b3:"),
        "wallet receipt hash must be b3-prefixed"
    );

    let json = serde_json::to_value(receipt).expect("receipt should serialize");
    assert_no_forbidden_authority_keys(&json);
}

#[test]
fn approved_payout_executes_only_as_wallet_issue_receipt() {
    let cfg = harness::cfg();
    let client = harness::client();

    let request = approved_payout_request(CREATOR_A, 77, "reward-plan-phase3-round2-a");
    let idem = "internal_roc_phase3_round2_payout_creator_a";

    let receipt = client
        .issue(&cfg, &request, idem)
        .expect("approved payout should execute through wallet issue path");

    assert_accepted_payout_receipt(&receipt, CREATOR_A, 77, idem);
    assert_eq!(harness::balance_of(&client, &cfg, CREATOR_A), 77);
}

#[test]
fn duplicate_approved_payout_idempotency_does_not_double_issue() {
    let cfg = harness::cfg();
    let client = harness::client();

    let request = approved_payout_request(CREATOR_A, 31, "reward-plan-phase3-round2-dupe");
    let idem = "internal_roc_phase3_round2_payout_duplicate_guard";

    let first = client
        .issue(&cfg, &request, idem)
        .expect("first approved payout should execute");
    let first_balance = harness::balance_of(&client, &cfg, CREATOR_A);

    let replay = client
        .issue(&cfg, &request, idem)
        .expect("same idempotency key should replay accepted ledger result");

    let replay_balance = harness::balance_of(&client, &cfg, CREATOR_A);

    assert_eq!(first_balance, 31);
    assert_eq!(
        replay_balance, first_balance,
        "idempotent approved payout retry must not double issue"
    );
    assert_eq!(first.txid, replay.txid);
    assert_eq!(first.ledger_seq_start, replay.ledger_seq_start);
    assert_eq!(first.ledger_seq_end, replay.ledger_seq_end);
    assert_eq!(first.ledger_root, replay.ledger_root);
    assert_accepted_payout_receipt(&replay, CREATOR_A, 31, idem);
}

#[test]
fn approved_payouts_to_distinct_accounts_remain_backend_balance_truth() {
    let cfg = harness::cfg();
    let client = harness::client();

    let payout_a = client
        .issue(
            &cfg,
            &approved_payout_request(CREATOR_A, 13, "reward-plan-phase3-round2-a"),
            "internal_roc_phase3_round2_payout_a",
        )
        .expect("approved payout A should execute");

    let payout_b = client
        .issue(
            &cfg,
            &approved_payout_request(CREATOR_B, 29, "reward-plan-phase3-round2-b"),
            "internal_roc_phase3_round2_payout_b",
        )
        .expect("approved payout B should execute");

    assert_accepted_payout_receipt(
        &payout_a,
        CREATOR_A,
        13,
        "internal_roc_phase3_round2_payout_a",
    );
    assert_accepted_payout_receipt(
        &payout_b,
        CREATOR_B,
        29,
        "internal_roc_phase3_round2_payout_b",
    );

    assert_eq!(harness::balance_of(&client, &cfg, CREATOR_A), 13);
    assert_eq!(harness::balance_of(&client, &cfg, CREATOR_B), 29);
    assert_ne!(
        payout_a.txid, payout_b.txid,
        "separate approved payout executions need separate backend receipt ids"
    );
}

#[test]
fn reward_plan_policy_and_accounting_fields_cannot_smuggle_payout_issue_request_authority() {
    for forbidden in FORBIDDEN_AUTHORITY_KEYS.iter().copied().chain([
        "reward_plan_id",
        "accounting_snapshot_id",
        "policy_decision_id",
        "payout_receipt_txid",
        "wallet_mutation",
        "ledger_mutation",
        "operation_id",
        "account_sequence",
    ]) {
        let mut poisoned = serde_json::to_value(approved_payout_request(
            CREATOR_A,
            11,
            "reward-plan-phase3-round2-poison",
        ))
        .expect("issue request should serialize");

        poisoned
            .as_object_mut()
            .expect("issue request JSON should be object")
            .insert(forbidden.to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<IssueRequest>(poisoned).is_err(),
            "IssueRequest must reject authority-smuggling field `{forbidden}`"
        );
    }
}

#[test]
fn accounting_observer_after_payout_receipt_cannot_mutate_wallet_balance() {
    let cfg = harness::cfg();
    let client = harness::client();
    let accounting = NoopAccountingClient;

    let receipt = client
        .issue(
            &cfg,
            &approved_payout_request(CREATOR_A, 43, "reward-plan-phase3-round2-observe"),
            "internal_roc_phase3_round2_payout_observe",
        )
        .expect("approved payout should execute before accounting observation");

    assert_accepted_payout_receipt(
        &receipt,
        CREATOR_A,
        43,
        "internal_roc_phase3_round2_payout_observe",
    );

    let before = harness::balance_of(&client, &cfg, CREATOR_A);

    accounting.record(AccountingEvent {
        op: "approved_payout",
        asset: receipt.asset.clone(),
        amount_minor: receipt.amount_minor.get(),
    });

    let after = harness::balance_of(&client, &cfg, CREATOR_A);

    assert_eq!(
        after, before,
        "accounting observation after approved payout receipt must not mutate wallet balance"
    );
}
