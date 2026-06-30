//! RO:WHAT — Internal ROC Beta rewarder planning/non-authority tests.
//! RO:WHY — Internal ROC Beta value loop; Concerns: ECON/GOV/SEC. svc-rewarder may plan capped payouts but must not become wallet, ledger, receipt, balance, entitlement, finality, bridge, staking, or liquidity authority.
//! RO:INTERACTS — ComputeInput, RewardPolicy, AccountingSnapshot, RewardManifest, SettlementBatch, WalletIssueBatch.
//! RO:INVARIANTS — rewarder output is deterministic planning material only; svc-wallet remains mutation front-door; ron-ledger remains truth.
//! RO:METRICS — none.
//! RO:CONFIG — no config change.
//! RO:SECURITY — rejects authority-poison fields and keeps wallet issue batch as handoff shape only.
//! RO:TEST — cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority.

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

const POLICY_ID: &str = "policy:internal-roc-beta";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const IDEMPOTENCY_SALT: &str = "internal-roc-beta|rewarder-policy-boundary";

#[test]
fn paid_content_reward_plan_is_deterministic_planning_not_receipt_or_balance_truth() {
    let snapshot = beta_snapshot();
    let manifest = compute_manifest(beta_input(snapshot.clone()), IntentResult::DryRun)
        .expect("rewarder planning manifest should compute");

    assert_eq!(manifest.epoch_id, "epoch-internal-roc-beta-paid-content");
    assert_b3_hash(&manifest.run_key);
    assert_b3_hash(&manifest.commitment);
    assert_eq!(manifest.inputs_cid, inputs_cid_for(snapshot).to_string());

    assert!(
        !manifest.ledger.emitted,
        "dry-run reward planning must not emit wallet or ledger effects"
    );
    assert_eq!(
        manifest.ledger.result, "dry_run",
        "rewarder planning result must stay dry_run when no wallet execution occurred"
    );

    assert!(
        manifest.totals.payout_minor_units <= manifest.totals.pool_minor_units,
        "reward plan must conserve pool budget"
    );
    assert_eq!(
        manifest
            .totals
            .payout_minor_units
            .checked_add(manifest.totals.residual_minor_units)
            .expect("payout + residual should not overflow"),
        manifest.totals.pool_minor_units,
        "payout + residual must equal pool"
    );

    let accounts = manifest
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
            "acct_post_creator"
        ],
        "rewarder payout plan must be sorted deterministically by account"
    );

    assert_json_has_no_authority_keys(&serde_json::to_value(&manifest).expect("manifest JSON"));
}

#[test]
fn wallet_issue_batch_is_handoff_shape_not_payout_execution_receipt() {
    let manifest =
        compute_manifest(beta_input(beta_snapshot()), IntentResult::Accepted).expect("manifest");
    let batch = SettlementBatch::from_manifest(&manifest).expect("settlement planning batch");

    assert_eq!(batch.run_key, manifest.run_key);
    assert_eq!(batch.epoch_id, manifest.epoch_id);
    assert_eq!(batch.manifest_commitment, manifest.commitment);
    assert_eq!(batch.funding_source, RewardFundingSource::ProtocolPool);
    assert_eq!(batch.total_minor_units, manifest.totals.payout_minor_units);
    assert_eq!(batch.intents.len(), manifest.payouts.len());

    for intent in &batch.intents {
        assert_eq!(intent.asset, ROC_ASSET);
        assert_eq!(intent.run_key, manifest.run_key);
        assert_eq!(intent.epoch_id, manifest.epoch_id);
        assert_eq!(intent.manifest_commitment, manifest.commitment);
        assert_eq!(intent.funding_source, RewardFundingSource::ProtocolPool);
        assert_bounded_b3_idempotency_key(&intent.idempotency_key);
        assert!(
            intent.idempotency_key.len() <= 67,
            "idempotency key must stay bounded"
        );
        assert!(
            intent.memo.starts_with("svc-rewarder:"),
            "memo should identify rewarder handoff provenance"
        );
    }

    let wallet_batch = batch.to_wallet_issue_batch();

    assert_eq!(wallet_batch.wallet_path, WALLET_ISSUE_PATH);
    assert_eq!(
        wallet_batch.funding_source,
        RewardFundingSource::ProtocolPool
    );
    assert_eq!(
        wallet_batch.total_minor_units,
        manifest.totals.payout_minor_units.get().to_string(),
        "wallet issue batch amount must be string-encoded minor units"
    );
    assert_eq!(wallet_batch.requests.len(), manifest.payouts.len());

    for request in &wallet_batch.requests {
        assert_eq!(request.asset, ROC_ASSET);
        assert!(
            request.amount_minor.parse::<u128>().expect("amount string") > 0,
            "wallet handoff amount must be positive integer minor units"
        );
        assert!(
            request
                .idempotency_key
                .as_deref()
                .is_some_and(|key| key.starts_with("b3:")),
            "wallet handoff must keep idempotency as retry/dedupe material"
        );
        assert!(
            request
                .memo
                .as_deref()
                .is_some_and(|memo| memo.starts_with("svc-rewarder:")),
            "wallet handoff memo should remain provenance only"
        );
    }

    assert_json_has_no_authority_keys(
        &serde_json::to_value(&wallet_batch).expect("wallet handoff JSON"),
    );
}

#[test]
fn reward_policy_rejects_paid_content_authority_poison_fields() {
    for forbidden_field in [
        "wallet_mutation",
        "ledger_mutation",
        "balance_truth",
        "receipt_truth",
        "paid_unlock_authority",
        "entitlement_truth",
        "payout_execution_truth",
        "client_finality_claim",
        "cache_unlock_authority",
        "gateway_receipt_truth",
        "omnigate_receipt_truth",
        "bridge_txid",
        "staking_position_id",
        "liquidity_pool_id",
        "external_settlement_id",
        "raw_engagement_mints_roc",
    ] {
        let mut value = beta_policy_json();
        value
            .as_object_mut()
            .expect("policy object")
            .insert(forbidden_field.to_owned(), json!(true));

        assert!(
            serde_json::from_value::<RewardPolicy>(value).is_err(),
            "RewardPolicy must reject forbidden authority field `{forbidden_field}`"
        );
    }
}

#[test]
fn compute_request_rejects_paid_content_authority_poison_fields() {
    for forbidden_field in [
        "wallet_mutation",
        "ledger_mutation",
        "balance_truth",
        "receipt_truth",
        "paid_unlock_authority",
        "entitlement_truth",
        "payout_execution_truth",
        "client_finality_claim",
        "cache_unlock_authority",
        "gateway_receipt_truth",
        "omnigate_receipt_truth",
        "bridge_txid",
        "staking_position_id",
        "liquidity_pool_id",
        "external_settlement_id",
        "raw_engagement_mints_roc",
    ] {
        let mut value = beta_compute_request_json();
        value
            .as_object_mut()
            .expect("compute request object")
            .insert(forbidden_field.to_owned(), json!(true));

        assert!(
            serde_json::from_value::<ComputeEpochRequest>(value).is_err(),
            "ComputeEpochRequest must reject forbidden authority field `{forbidden_field}`"
        );
    }
}

#[test]
fn protocol_pool_planning_requires_signed_policy_and_stays_provenance_only() {
    let mut policy = beta_policy();
    policy.signed = false;

    let err = validate_reward_policy(&policy, POLICY_ID, POLICY_HASH)
        .expect_err("unsigned protocol pool planning policy must reject");

    assert!(
        err.to_string().contains("requires signed policy"),
        "unexpected policy rejection: {err}"
    );

    let mut signed = beta_policy();
    signed.signed = true;

    validate_reward_policy(&signed, POLICY_ID, POLICY_HASH).expect("signed policy should validate");

    assert_eq!(
        signed.funding_source.as_str(),
        "protocol_pool",
        "funding source label is provenance, not mutation authority"
    );
    assert!(signed.funding_source.requires_signed_policy());
}

fn beta_input(snapshot: AccountingSnapshot) -> ComputeInput {
    ComputeInput {
        epoch_id: "epoch-internal-roc-beta-paid-content".to_owned(),
        inputs_cid: inputs_cid_for(snapshot.clone()),
        policy: beta_policy(),
        snapshot,
        dry_run: true,
        idempotency_salt: IDEMPOTENCY_SALT.to_owned(),
    }
}

fn beta_snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1_850_000,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_content_view_creator".to_owned(),
                bytes_stored: 0,
                bytes_served: 250,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_post_creator".to_owned(),
                bytes_stored: 100,
                bytes_served: 25,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_article_creator".to_owned(),
                bytes_stored: 300,
                bytes_served: 30,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_comment_creator".to_owned(),
                bytes_stored: 50,
                bytes_served: 10,
                uptime_seconds: 0,
            },
        ],
    }
}

fn inputs_cid_for(snapshot: AccountingSnapshot) -> ContentCid {
    let cid = canonical_snapshot_cid(snapshot).expect("canonical accounting snapshot CID");
    ContentCid::parse(cid).expect("canonical CID parses")
}

fn beta_policy() -> RewardPolicy {
    RewardPolicy {
        id: POLICY_ID.to_owned(),
        hash: POLICY_HASH.to_owned(),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
        max_payout_minor_units: AmountMinor(1_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_owned(),
    }
}

fn beta_policy_json() -> Value {
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

fn beta_compute_request_json() -> Value {
    let snapshot = beta_snapshot();
    let inputs_cid = canonical_snapshot_cid(snapshot.clone()).expect("snapshot CID");

    json!({
        "inputs_cid": inputs_cid,
        "policy_id": POLICY_ID,
        "policy_hash": POLICY_HASH,
        "dry_run": true,
        "snapshot": snapshot,
        "policy": beta_policy_json()
    })
}

fn assert_b3_hash(value: &str) {
    assert!(
        value.starts_with("b3:"),
        "expected b3:<64 lowercase hex>, got {value}"
    );

    let hex = &value[3..];
    assert_eq!(hex.len(), 64, "expected 64 hex chars, got {value}");
    assert!(
        hex.chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "expected lowercase b3 hex, got {value}"
    );
}

fn assert_json_has_no_authority_keys(value: &Value) {
    const FORBIDDEN_KEYS: &[&str] = &[
        "wallet_mutation",
        "ledger_mutation",
        "balance_truth",
        "receipt_truth",
        "paid_unlock_authority",
        "entitlement_truth",
        "payout_execution_truth",
        "payout_receipt",
        "wallet_receipt",
        "ledger_receipt",
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "balance_minor",
        "wallet_balance",
        "ledger_balance",
        "unlock_granted",
        "finality",
        "finalized",
        "anchored",
        "settlement_status",
        "state_root",
        "checkpoint_root",
        "checkpoint_hash",
        "bridge_txid",
        "staking_position_id",
        "liquidity_pool_id",
        "external_settlement_id",
        "raw_engagement_mints_roc",
    ];

    match value {
        Value::Object(object) => {
            for key in object.keys() {
                assert!(
                    !FORBIDDEN_KEYS.iter().any(|forbidden| key == forbidden),
                    "rewarder planning/hand-off JSON must not expose authority key `{key}`"
                );
            }

            for nested in object.values() {
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

fn assert_bounded_b3_idempotency_key(value: &str) {
    assert!(
        value.starts_with("b3:"),
        "expected bounded b3-prefixed idempotency key, got {value}"
    );

    let hex = &value[3..];
    assert!(
        (32..=64).contains(&hex.len()),
        "expected bounded lowercase hex idempotency key body length 32..=64, got {} in {value}",
        hex.len()
    );
    assert!(
        hex.chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "expected lowercase hex idempotency key body, got {value}"
    );
}
