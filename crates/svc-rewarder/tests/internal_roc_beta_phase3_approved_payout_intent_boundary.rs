//! RO:WHAT — Internal ROC Beta Phase 3 Round 2 approved-payout intent boundary tests for svc-rewarder.
//! RO:WHY — Proves rewarder emits deterministic capped payout intent candidates and wallet issue handoff DTOs only; execution remains svc-wallet-only.
//! RO:INTERACTS — compute_manifest, SettlementBatch, WalletIssueBatch, WalletIssueRequest, IntentStore.
//! RO:INVARIANTS — rewarder plans only; no wallet/ledger mutation, fake receipt, fake balance, fake finality, bridge, staking, liquidity, or external settlement.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config changes.
//! RO:SECURITY — rejects authority-smuggling fields and keeps idempotency as retry/dedupe, not authority.
//! RO:TEST — cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use serde_json::{json, Value};
use svc_rewarder::{
    core::{compute_manifest, AmountMinor, ComputeInput},
    inputs::{
        AccountContribution, AccountingSnapshot, ContentCid, RewardFundingSource, RewardPolicy,
    },
    outputs::{
        IntentResult, IntentStore, SettlementBatch, WalletIssueBatch, WalletIssueRequest,
        ROC_ASSET, WALLET_ISSUE_PATH,
    },
};

const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const IDEMPOTENCY_SALT: &str = "internal-roc-beta|phase3|round2|approved-payout";

const FORBIDDEN_AUTHORITY_KEYS: &[&str] = &[
    "receipt_id",
    "receipt_hash",
    "receipt_root",
    "receipt_proof",
    "accepted_receipt",
    "wallet_receipt",
    "ledger_receipt",
    "balance",
    "balance_minor",
    "available_balance",
    "wallet_balance",
    "ledger_balance",
    "balance_truth",
    "receipt_truth",
    "payout_execution_truth",
    "wallet_mutation",
    "ledger_mutation",
    "operation_id",
    "account_sequence",
    "finality",
    "finalized",
    "checkpoint_hash",
    "checkpoint_root",
    "bridge_txid",
    "solana_signature",
    "rox_settlement_id",
    "staking_position_id",
    "staking_yield_bps",
    "liquidity_pool_id",
    "exchange_order_id",
    "outside_settlement_claim",
    "paid_unlock_authority",
    "cache_unlock_authority",
    "silent_spend",
    "fake_balance",
    "fake_receipt",
];

fn cid() -> ContentCid {
    ContentCid::parse(format!("b3:{}", "a".repeat(64))).expect("test cid parses")
}

fn policy(max_payout_minor_units: u128) -> RewardPolicy {
    RewardPolicy {
        id: "policy:internal-roc-beta-phase3-round2".to_owned(),
        hash: POLICY_HASH.to_owned(),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
        max_payout_minor_units: AmountMinor(max_payout_minor_units),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_owned(),
    }
}

fn snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1_900_400_000_000,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_creator_b".to_owned(),
                bytes_stored: 200,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_creator_a".to_owned(),
                bytes_stored: 300,
                bytes_served: 0,
                uptime_seconds: 0,
            },
        ],
    }
}

fn input(max_payout_minor_units: u128) -> ComputeInput {
    ComputeInput {
        epoch_id: "internal-roc-beta-phase3-round2".to_owned(),
        inputs_cid: cid(),
        policy: policy(max_payout_minor_units),
        snapshot: snapshot(),
        dry_run: false,
        idempotency_salt: IDEMPOTENCY_SALT.to_owned(),
    }
}

fn approved_payout_batch(max_payout_minor_units: u128) -> (SettlementBatch, WalletIssueBatch) {
    let manifest = compute_manifest(input(max_payout_minor_units), IntentResult::Accepted)
        .expect("approved payout manifest should compute");
    let settlement = SettlementBatch::from_manifest(&manifest)
        .expect("approved payout settlement intent candidates should plan");
    let wallet_batch = settlement.to_wallet_issue_batch();
    (settlement, wallet_batch)
}

fn assert_no_authority_keys(value: &Value) {
    match value {
        Value::Object(object) => {
            for key in object.keys() {
                assert!(
                    !FORBIDDEN_AUTHORITY_KEYS.contains(&key.as_str()),
                    "rewarder approved payout artifact must not expose authority key `{key}`"
                );
            }

            for nested in object.values() {
                assert_no_authority_keys(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_authority_keys(nested);
            }
        }
        _ => {}
    }
}

#[test]
fn approved_payout_intent_candidates_are_wallet_issue_handoff_only() {
    let (settlement, wallet_batch) = approved_payout_batch(700);

    assert_eq!(settlement.epoch_id, "internal-roc-beta-phase3-round2");
    assert_eq!(settlement.funding_source, RewardFundingSource::ProtocolPool);
    assert_eq!(settlement.total_minor_units.get(), 700);
    assert_eq!(settlement.intents.len(), 2);

    assert_eq!(wallet_batch.run_key, settlement.run_key);
    assert_eq!(wallet_batch.epoch_id, settlement.epoch_id);
    assert_eq!(
        wallet_batch.manifest_commitment,
        settlement.manifest_commitment
    );
    assert_eq!(
        wallet_batch.funding_source,
        RewardFundingSource::ProtocolPool
    );
    assert_eq!(wallet_batch.wallet_path, WALLET_ISSUE_PATH);
    assert_eq!(wallet_batch.total_minor_units, "700");
    assert_eq!(wallet_batch.requests.len(), 2);

    let accounts = settlement
        .intents
        .iter()
        .map(|intent| intent.to.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        accounts,
        vec!["acct_creator_a", "acct_creator_b"],
        "approved payout intent candidates must remain deterministically sorted"
    );

    for intent in &settlement.intents {
        assert_eq!(intent.asset, ROC_ASSET);
        assert_eq!(intent.run_key, settlement.run_key);
        assert_eq!(intent.epoch_id, settlement.epoch_id);
        assert_eq!(intent.manifest_commitment, settlement.manifest_commitment);
        assert_eq!(intent.funding_source, RewardFundingSource::ProtocolPool);
        assert!(intent.idempotency_key.starts_with("b3:"));
        assert!(
            intent.idempotency_key.len() <= 64,
            "svc-wallet Idempotency-Key must remain bounded"
        );
        assert!(
            intent.memo.starts_with("svc-rewarder:"),
            "memo should identify rewarder planning source without claiming receipt truth"
        );

        let request = intent.to_wallet_issue_request();
        assert_eq!(request.to, intent.to);
        assert_eq!(request.asset, ROC_ASSET);
        assert_eq!(
            request.amount_minor,
            intent.amount_minor_units.get().to_string()
        );
        assert_eq!(
            request.idempotency_key.as_deref(),
            Some(intent.idempotency_key.as_str())
        );
        assert_eq!(request.memo.as_deref(), Some(intent.memo.as_str()));
    }

    assert_no_authority_keys(&serde_json::to_value(&settlement).expect("settlement serializes"));
    assert_no_authority_keys(
        &serde_json::to_value(&wallet_batch).expect("wallet batch serializes"),
    );
}

#[test]
fn category_pool_cap_and_conservation_survive_payout_intent_handoff() {
    let requested_cap = 333_u128;
    let (settlement, wallet_batch) = approved_payout_batch(requested_cap);

    assert!(
        settlement.total_minor_units.get() <= requested_cap,
        "policy max payout cap must bound approved payout intent candidates"
    );
    assert_eq!(
        wallet_batch.total_minor_units,
        settlement.total_minor_units.get().to_string(),
        "wallet batch total must report the actual floor-rounded payout total"
    );

    let request_total = wallet_batch
        .requests
        .iter()
        .try_fold(0_u128, |acc, request| {
            request
                .amount_minor
                .parse::<u128>()
                .ok()
                .and_then(|value| acc.checked_add(value))
        })
        .expect("wallet issue request amounts should parse and add");

    assert_eq!(
        request_total,
        settlement.total_minor_units.get(),
        "wallet issue request handoff total must equal capped settlement total"
    );

    assert_eq!(
        requested_cap - settlement.total_minor_units.get(),
        1,
        "floor rounding should leave the deterministic residual out of wallet handoff truth"
    );

    for request in &wallet_batch.requests {
        assert_eq!(request.asset, ROC_ASSET);
        assert!(
            request.amount_minor.parse::<u128>().expect("amount parses") > 0,
            "dust-filtered payout request must be nonzero"
        );
    }
}

#[test]
fn duplicate_payout_planning_markers_are_deterministic_but_not_execution_truth() {
    let (first_settlement, first_wallet_batch) = approved_payout_batch(700);
    let (second_settlement, second_wallet_batch) = approved_payout_batch(700);

    assert_eq!(
        first_settlement, second_settlement,
        "same sealed inputs must produce identical approved payout intent candidates"
    );
    assert_eq!(
        first_wallet_batch, second_wallet_batch,
        "same sealed inputs must produce identical svc-wallet handoff requests"
    );

    let store = IntentStore::default();

    assert_eq!(
        store.emit_batch_once(&first_settlement, false).as_str(),
        "accepted",
        "first rewarder handoff marker may be accepted"
    );
    assert_eq!(
        store.emit_batch_once(&second_settlement, false).as_str(),
        "dup",
        "duplicate rewarder handoff marker must be dup, not second payout authority"
    );
    assert_eq!(
        store.emit_batch_once(&second_settlement, true).as_str(),
        "dry_run",
        "dry-run must remain planning-only"
    );
}

#[test]
fn wallet_issue_requests_remain_string_money_and_do_not_carry_receipt_truth() {
    let (_, wallet_batch) = approved_payout_batch(700);

    for request in &wallet_batch.requests {
        let encoded = serde_json::to_string(request).expect("wallet request serializes");

        assert!(encoded.contains(r#""amount_minor":""#));
        assert!(!encoded.contains(r#""amount_minor":0"#));
        assert!(encoded.contains(r#""idempotency_key":"#));

        for forbidden in [
            "receipt",
            "balance",
            "finality",
            "checkpoint",
            "bridge",
            "staking",
            "liquidity",
            "exchange",
            "settlement_status",
        ] {
            assert!(
                !encoded.to_ascii_lowercase().contains(forbidden),
                "wallet request handoff must not carry `{forbidden}` authority: {encoded}"
            );
        }
    }
}

#[test]
fn payout_handoff_dtos_reject_authority_smuggling_fields() {
    let (settlement, wallet_batch) = approved_payout_batch(700);

    for forbidden in FORBIDDEN_AUTHORITY_KEYS {
        let mut poisoned_settlement =
            serde_json::to_value(&settlement).expect("settlement serializes");
        poisoned_settlement
            .as_object_mut()
            .expect("settlement JSON object")
            .insert((*forbidden).to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<SettlementBatch>(poisoned_settlement).is_err(),
            "SettlementBatch must reject authority-smuggling field `{forbidden}`"
        );

        let mut poisoned_request =
            serde_json::to_value(&wallet_batch.requests[0]).expect("request serializes");
        poisoned_request
            .as_object_mut()
            .expect("wallet request JSON object")
            .insert((*forbidden).to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<WalletIssueRequest>(poisoned_request).is_err(),
            "WalletIssueRequest must reject authority-smuggling field `{forbidden}`"
        );
    }
}
