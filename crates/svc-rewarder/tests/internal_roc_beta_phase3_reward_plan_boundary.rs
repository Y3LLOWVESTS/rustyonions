//! RO:WHAT — Internal ROC Beta Phase 3 Round 1 reward-plan boundary tests for svc-rewarder.
//! RO:WHY — Proves rewarder consumes deterministic accounting snapshots and emits capped planning material only; no wallet/ledger mutation, fake receipt, fake balance, finality, bridge, staking, liquidity, or external settlement.
//! RO:INTERACTS — AccountingSnapshot, RewardPolicy, ComputeInput, RewardManifest, SettlementBatch, WalletIssueBatch.
//! RO:INVARIANTS — reward plans are not receipts; settlement batches are handoff previews only; svc-wallet remains mutation front-door; ron-ledger remains economic truth.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config changes.
//! RO:SECURITY — rejects unknown/authority-poison fields and raw-engagement payout authority.
//! RO:TEST — cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use serde_json::{json, Value};
use svc_rewarder::{
    core::{compute_manifest, AmountMinor, ComputeInput},
    http::dto::ComputeEpochRequest,
    inputs::{
        canonical_snapshot_cid, validate_reward_policy, AccountContribution, AccountingSnapshot,
        ContentCid, RewardFundingSource, RewardPolicy,
    },
    outputs::{IntentResult, SettlementBatch, ROC_ASSET, WALLET_ISSUE_PATH},
};

const POLICY_ID: &str = "policy:internal-roc-beta-phase3";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const IDEMPOTENCY_SALT: &str = "internal-roc-beta|phase3|reward-plan-boundary";

const FORBIDDEN_AUTHORITY_KEYS: &[&str] = &[
    "balance",
    "balance_minor",
    "available_balance",
    "wallet_balance",
    "ledger_balance",
    "wallet_receipt",
    "ledger_receipt",
    "receipt_id",
    "receipt_hash",
    "receipt_root",
    "receipt_txid",
    "payout_receipt_txid",
    "settlement_status",
    "finality",
    "finalized",
    "wallet_mutation",
    "ledger_mutation",
    "ledger_side_effect",
    "wallet_side_effect",
    "payout_execution",
    "payout_execution_truth",
    "operation_id",
    "account_sequence",
    "state_root",
    "checkpoint_root",
    "checkpoint_hash",
    "validator_signature",
    "bridge_txid",
    "solana_signature",
    "rox_settlement_id",
    "staking_position_id",
    "staking_yield_bps",
    "liquidity_pool_id",
    "exchange_order_id",
    "outside_settlement_claim",
];

fn beta_snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1_900_000_000_000,
        pool_minor_units: AmountMinor(10_000),
        contributions: vec![
            AccountContribution {
                account: "acct_post_creator".to_owned(),
                bytes_stored: 500,
                bytes_served: 200,
                uptime_seconds: 50,
            },
            AccountContribution {
                account: "acct_comment_creator".to_owned(),
                bytes_stored: 125,
                bytes_served: 60,
                uptime_seconds: 10,
            },
            AccountContribution {
                account: "acct_article_creator".to_owned(),
                bytes_stored: 900,
                bytes_served: 300,
                uptime_seconds: 100,
            },
            AccountContribution {
                account: "acct_content_view_creator".to_owned(),
                bytes_stored: 250,
                bytes_served: 80,
                uptime_seconds: 20,
            },
        ],
    }
}

fn capped_policy(max_minor_units: u128) -> RewardPolicy {
    RewardPolicy {
        id: POLICY_ID.to_owned(),
        hash: POLICY_HASH.to_owned(),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
        max_payout_minor_units: AmountMinor(max_minor_units),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_owned(),
    }
}

fn inputs_cid_for(snapshot: AccountingSnapshot) -> ContentCid {
    let cid = canonical_snapshot_cid(snapshot).expect("snapshot should canonicalize to CID");
    ContentCid::parse(cid).expect("canonical snapshot CID should parse")
}

fn beta_input(snapshot: AccountingSnapshot, max_minor_units: u128) -> ComputeInput {
    ComputeInput {
        epoch_id: "epoch-internal-roc-beta-phase3-round1".to_owned(),
        inputs_cid: inputs_cid_for(snapshot.clone()),
        policy: capped_policy(max_minor_units),
        snapshot,
        dry_run: true,
        idempotency_salt: IDEMPOTENCY_SALT.to_owned(),
    }
}

fn base_policy_json() -> Value {
    json!({
        "id": POLICY_ID,
        "hash": POLICY_HASH,
        "signed": true,
        "funding_source": "protocol_pool",
        "max_payout_minor_units": "1000",
        "min_payout_minor_units": "1",
        "weight_bps": 10000,
        "rounding": "floor"
    })
}

fn valid_compute_body() -> Value {
    let snapshot = beta_snapshot();
    let snapshot_json = serde_json::to_value(&snapshot).expect("snapshot serializes");
    let inputs_cid = canonical_snapshot_cid(snapshot).expect("snapshot CID");

    json!({
        "inputs_cid": inputs_cid,
        "policy_id": POLICY_ID,
        "policy_hash": POLICY_HASH,
        "dry_run": true,
        "snapshot": snapshot_json,
        "policy": base_policy_json()
    })
}

fn assert_b3_hash(value: &str) {
    let Some(hex) = value.strip_prefix("b3:") else {
        panic!("expected b3:<64 lowercase hex>, got {value}");
    };
    assert_eq!(hex.len(), 64, "b3 hash must be 64 hex chars");
    assert!(
        hex.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')),
        "b3 hash must be lowercase hex"
    );
}

fn assert_json_has_no_authority_keys(value: &Value) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert!(
                    !FORBIDDEN_AUTHORITY_KEYS.contains(&key.as_str()),
                    "rewarder planning artifact must not expose authority key `{key}`"
                );
                assert_json_has_no_authority_keys(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_json_has_no_authority_keys(nested);
            }
        }
        _ => {}
    }
}

#[test]
fn reward_plan_is_deterministic_for_same_snapshot_regardless_input_order() {
    let forward_snapshot = beta_snapshot();

    let mut reversed_snapshot = forward_snapshot.clone();
    reversed_snapshot.contributions.reverse();

    let forward = compute_manifest(beta_input(forward_snapshot, 10_000), IntentResult::DryRun)
        .expect("forward reward plan should compute");
    let reversed = compute_manifest(beta_input(reversed_snapshot, 10_000), IntentResult::DryRun)
        .expect("reversed reward plan should compute");

    assert_eq!(forward.run_key, reversed.run_key);
    assert_eq!(forward.commitment, reversed.commitment);
    assert_eq!(forward.inputs_cid, reversed.inputs_cid);
    assert_eq!(forward.totals, reversed.totals);
    assert_eq!(forward.payouts, reversed.payouts);

    assert_b3_hash(&forward.run_key);
    assert_b3_hash(&forward.commitment);
    assert_b3_hash(&forward.inputs_cid);

    assert!(
        !forward.ledger.emitted,
        "Phase 3 Round 1 planning must not emit wallet/ledger effects"
    );
    assert_eq!(
        forward.ledger.result, "dry_run",
        "Phase 3 Round 1 reward planning must stay dry_run"
    );

    let accounts = forward
        .payouts
        .iter()
        .map(|payout| payout.account.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        accounts,
        vec![
            "acct_article_creator",
            "acct_comment_creator",
            "acct_content_view_creator",
            "acct_post_creator",
        ],
        "reward plan payouts must be sorted deterministically by account"
    );

    assert_json_has_no_authority_keys(&serde_json::to_value(&forward).expect("manifest JSON"));
}

#[test]
fn reward_plan_enforces_pool_cap_and_conservation_without_execution() {
    let manifest = compute_manifest(beta_input(beta_snapshot(), 750), IntentResult::DryRun)
        .expect("capped reward plan should compute");

    assert_eq!(
        manifest.totals.pool_minor_units,
        AmountMinor(750),
        "policy max_payout_minor_units must cap the snapshot pool"
    );

    assert_eq!(
        manifest
            .totals
            .payout_minor_units
            .checked_add(manifest.totals.residual_minor_units)
            .expect("payout + residual should not overflow"),
        manifest.totals.pool_minor_units,
        "payout + residual must exactly equal capped pool"
    );

    assert!(
        manifest.totals.payout_minor_units <= manifest.totals.pool_minor_units,
        "planned payouts must not exceed capped pool"
    );
    assert!(
        !manifest.ledger.emitted,
        "pool-cap planning must not execute wallet or ledger effects"
    );

    validate_reward_policy(&capped_policy(750), POLICY_ID, POLICY_HASH)
        .expect("policy should validate as planning config only");
}

#[test]
fn wallet_issue_batch_is_handoff_preview_not_payout_receipt_or_balance_truth() {
    let manifest = compute_manifest(beta_input(beta_snapshot(), 1_000), IntentResult::DryRun)
        .expect("reward plan should compute");

    assert!(
        !manifest.ledger.emitted,
        "dry-run manifest must not claim wallet emission"
    );

    let batch = SettlementBatch::from_manifest(&manifest).expect("settlement batch preview");
    let wallet_batch = batch.to_wallet_issue_batch();

    assert_eq!(batch.run_key, manifest.run_key);
    assert_eq!(batch.epoch_id, manifest.epoch_id);
    assert_eq!(batch.manifest_commitment, manifest.commitment);
    assert_eq!(batch.total_minor_units, manifest.totals.payout_minor_units);
    assert_eq!(wallet_batch.wallet_path, WALLET_ISSUE_PATH);
    assert_eq!(
        wallet_batch.funding_source,
        RewardFundingSource::ProtocolPool
    );
    assert_eq!(wallet_batch.requests.len(), manifest.payouts.len());

    for request in &wallet_batch.requests {
        assert_eq!(request.asset, ROC_ASSET);
        assert!(
            request
                .amount_minor
                .parse::<u128>()
                .expect("wallet amount should be integer minor units")
                > 0
        );
        assert!(
            request
                .idempotency_key
                .as_deref()
                .expect("handoff preview should carry wallet idempotency key")
                .starts_with("b3:"),
            "wallet handoff idempotency keys must be deterministic b3 references"
        );
    }

    let json = serde_json::to_value(&wallet_batch).expect("wallet batch should serialize");
    assert_json_has_no_authority_keys(&json);

    for forbidden in [
        "receipt_hash",
        "balance_minor",
        "settlement_status",
        "finality",
        "operation_id",
        "ledger_mutation",
        "wallet_mutation",
    ] {
        let mut poisoned = json.clone();
        poisoned
            .as_object_mut()
            .expect("wallet batch JSON should be object")
            .insert(forbidden.to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<svc_rewarder::outputs::WalletIssueBatch>(poisoned).is_err(),
            "WalletIssueBatch must reject authority-smuggling field `{forbidden}`"
        );
    }
}

#[test]
fn reward_policy_and_compute_request_reject_authority_poison_fields() {
    for forbidden in FORBIDDEN_AUTHORITY_KEYS {
        let mut policy = base_policy_json();
        policy
            .as_object_mut()
            .expect("policy JSON should be object")
            .insert((*forbidden).to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<RewardPolicy>(policy).is_err(),
            "RewardPolicy must reject authority-smuggling field `{forbidden}`"
        );

        let mut compute = valid_compute_body();
        compute
            .as_object_mut()
            .expect("compute body JSON should be object")
            .insert((*forbidden).to_owned(), json!("client-supplied-authority"));

        assert!(
            serde_json::from_value::<ComputeEpochRequest>(compute).is_err(),
            "ComputeEpochRequest must reject authority-smuggling field `{forbidden}`"
        );
    }
}

#[test]
fn raw_engagement_fields_cannot_be_rewarder_payout_input_authority() {
    let snapshot = serde_json::to_value(beta_snapshot()).expect("snapshot should serialize");

    for forbidden_root in [
        "event_class",
        "source_event_class",
        "raw_views",
        "raw_likes",
        "raw_comments",
        "raw_watch_seconds",
        "analytics_only",
        "metering",
        "proof_eligible",
        "ad_budgeted",
        "reward_material",
        "raw_engagement_mints_roc",
    ] {
        let mut poisoned = snapshot.clone();
        poisoned
            .as_object_mut()
            .expect("snapshot JSON should be object")
            .insert(
                forbidden_root.to_owned(),
                json!("client-supplied-authority"),
            );

        assert!(
            serde_json::from_value::<AccountingSnapshot>(poisoned).is_err(),
            "AccountingSnapshot must reject root authority/raw-engagement field `{forbidden_root}`"
        );
    }

    for forbidden_contribution in [
        "event_class",
        "source_event_class",
        "raw_views",
        "raw_likes",
        "raw_comments",
        "raw_watch_seconds",
        "analytics_only",
        "proof_eligible",
        "ad_budgeted",
        "wallet_receipt",
        "ledger_receipt",
    ] {
        let mut poisoned = snapshot.clone();
        poisoned["contributions"]
            .as_array_mut()
            .expect("contributions should be array")[0]
            .as_object_mut()
            .expect("contribution should be object")
            .insert(
                forbidden_contribution.to_owned(),
                json!("client-supplied-authority"),
            );

        assert!(
            serde_json::from_value::<AccountingSnapshot>(poisoned).is_err(),
            "AccountContribution must reject authority/raw-engagement field `{forbidden_contribution}`"
        );
    }
}
