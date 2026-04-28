//! RO:WHAT — Cross-crate tests for the ron-accounting reward snapshot vector consumed by svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/GOV. Proves sealed accounting snapshots can feed deterministic reward issuance.
//! RO:INTERACTS — ron_accounting::reward_snapshot_interop_vector_v1, svc_rewarder::{inputs, core, outputs}.
//! RO:INVARIANTS — canonical b3 CID agrees; integer-only scores; wallet output is issue-shaped; no ledger mutation.
//! RO:METRICS — none; this is a pure test boundary with no Prometheus side effects.
//! RO:CONFIG — uses dev policy defaults and svc-rewarder idempotency salt.
//! RO:SECURITY — synthetic account IDs only; no bearer tokens, keys, raw payloads, or PII.
//! RO:TEST — cargo test -p svc-rewarder --test unit accounting_interop.

use ron_accounting::{
    canonical_json_for_snapshot, reward_snapshot_interop_vector_v1, RewardSnapshotInteropVector,
    REWARD_SNAPSHOT_VECTOR_EPOCH_ID, REWARD_SNAPSHOT_VECTOR_SCHEMA,
};
use svc_rewarder::core::{compute_manifest, ComputeInput};
use svc_rewarder::inputs::{
    canonical_snapshot_cid, AccountContribution, AccountingSnapshot, ContentCid, RewardPolicy,
};
use svc_rewarder::outputs::{IntentResult, SettlementBatch, ROC_ASSET, WALLET_ISSUE_PATH};

const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const IDEMPOTENCY_SALT: &str = "svc-rewarder|v1";
const WEIGHT_BPS: u128 = 10_000;

fn load_interop_snapshot() -> (RewardSnapshotInteropVector, AccountingSnapshot) {
    let vector = reward_snapshot_interop_vector_v1().expect("ron-accounting interop vector");

    vector.validate().expect("interop vector validates");
    assert_eq!(vector.schema, REWARD_SNAPSHOT_VECTOR_SCHEMA);
    assert_eq!(vector.epoch_id, REWARD_SNAPSHOT_VECTOR_EPOCH_ID);

    let canonical_json =
        canonical_json_for_snapshot(&vector.snapshot).expect("canonical accounting json");
    assert_eq!(canonical_json, vector.canonical_snapshot_json);

    let rewarder_snapshot =
        serde_json::from_str::<AccountingSnapshot>(&canonical_json).expect("rewarder snapshot dto");

    (vector, rewarder_snapshot)
}

fn policy() -> RewardPolicy {
    RewardPolicy::dev_default("policy:v1", POLICY_HASH)
}

fn score(contribution: &AccountContribution) -> u64 {
    contribution.bytes_stored + (contribution.bytes_served / 4) + contribution.uptime_seconds
}

fn compute_live_manifest(
    vector: &RewardSnapshotInteropVector,
    snapshot: AccountingSnapshot,
    snapshot_cid: &str,
) -> svc_rewarder::Result<svc_rewarder::outputs::RewardManifest> {
    compute_manifest(
        ComputeInput {
            epoch_id: vector.epoch_id.clone(),
            inputs_cid: ContentCid::parse(snapshot_cid)?,
            policy: policy(),
            snapshot,
            dry_run: false,
            idempotency_salt: IDEMPOTENCY_SALT.to_string(),
        },
        IntentResult::Accepted,
    )
}

#[test]
fn ron_accounting_vector_is_consumable_by_rewarder_snapshot_dto() {
    let (vector, mut snapshot) = load_interop_snapshot();

    assert_eq!(vector.expected_contribution_count, 2);
    assert_eq!(vector.expected_total_score, 342);
    assert_eq!(snapshot.produced_at_millis, 1);
    assert_eq!(snapshot.pool_minor_units.get(), 1_000);
    assert_eq!(snapshot.contributions.len(), 2);

    snapshot.validate().expect("rewarder snapshot validates");
    snapshot.canonicalize();

    assert_eq!(snapshot.contributions[0].account, "acct_a");
    assert_eq!(snapshot.contributions[0].bytes_stored, 100);
    assert_eq!(snapshot.contributions[0].bytes_served, 50);
    assert_eq!(snapshot.contributions[0].uptime_seconds, 10);

    assert_eq!(snapshot.contributions[1].account, "acct_b");
    assert_eq!(snapshot.contributions[1].bytes_stored, 200);
    assert_eq!(snapshot.contributions[1].bytes_served, 0);
    assert_eq!(snapshot.contributions[1].uptime_seconds, 20);

    let total_score = snapshot
        .contributions
        .iter()
        .map(score)
        .try_fold(0_u64, |acc, value| acc.checked_add(value))
        .expect("score sum should not overflow");

    assert_eq!(total_score, vector.expected_total_score);
}

#[test]
fn ron_accounting_and_rewarder_agree_on_canonical_snapshot_cid() {
    let (vector, snapshot) = load_interop_snapshot();

    let rewarder_cid = canonical_snapshot_cid(snapshot).expect("rewarder canonical cid");
    let parsed = ContentCid::parse(&rewarder_cid).expect("cid parses");

    assert_eq!(rewarder_cid, vector.snapshot_cid);
    assert_eq!(parsed.as_str(), vector.snapshot_cid);
    assert!(rewarder_cid.starts_with("b3:"));
    assert_eq!(rewarder_cid.len(), 67);
}

#[test]
fn interop_vector_computes_expected_reward_manifest_and_wallet_preview() {
    let (vector, snapshot) = load_interop_snapshot();
    let snapshot_cid = canonical_snapshot_cid(snapshot.clone()).expect("snapshot cid");
    let manifest =
        compute_live_manifest(&vector, snapshot, &snapshot_cid).expect("reward manifest");

    assert_eq!(manifest.epoch_id, REWARD_SNAPSHOT_VECTOR_EPOCH_ID);
    assert_eq!(manifest.inputs_cid, vector.snapshot_cid);
    assert_eq!(manifest.totals.pool_minor_units.get(), 1_000);
    assert_eq!(manifest.totals.payout_minor_units.get(), 999);
    assert_eq!(manifest.totals.residual_minor_units.get(), 1);
    assert_eq!(manifest.payouts.len(), 2);

    assert_eq!(manifest.payouts[0].account, "acct_a");
    assert_eq!(manifest.payouts[0].amount_minor_units.get(), 356);
    assert_eq!(manifest.payouts[0].score, u128::from(122_u64) * WEIGHT_BPS);

    assert_eq!(manifest.payouts[1].account, "acct_b");
    assert_eq!(manifest.payouts[1].amount_minor_units.get(), 643);
    assert_eq!(manifest.payouts[1].score, u128::from(220_u64) * WEIGHT_BPS);

    let weighted_score_total = manifest
        .payouts
        .iter()
        .map(|payout| payout.score)
        .try_fold(0_u128, |acc, value| acc.checked_add(value))
        .expect("weighted score sum should not overflow");

    assert_eq!(
        weighted_score_total,
        u128::from(vector.expected_total_score) * WEIGHT_BPS
    );

    let settlement = SettlementBatch::from_manifest(&manifest).expect("settlement batch");
    let wallet_batch = settlement.to_wallet_issue_batch();

    assert_eq!(wallet_batch.run_key, manifest.run_key);
    assert_eq!(wallet_batch.epoch_id, manifest.epoch_id);
    assert_eq!(wallet_batch.manifest_commitment, manifest.commitment);
    assert_eq!(wallet_batch.wallet_path, WALLET_ISSUE_PATH);
    assert_eq!(wallet_batch.total_minor_units, "999");
    assert_eq!(wallet_batch.requests.len(), 2);

    assert_eq!(wallet_batch.requests[0].to, "acct_a");
    assert_eq!(wallet_batch.requests[0].asset, ROC_ASSET);
    assert_eq!(wallet_batch.requests[0].amount_minor, "356");

    assert_eq!(wallet_batch.requests[1].to, "acct_b");
    assert_eq!(wallet_batch.requests[1].asset, ROC_ASSET);
    assert_eq!(wallet_batch.requests[1].amount_minor, "643");

    for request in &wallet_batch.requests {
        let idempotency_key = request
            .idempotency_key
            .as_deref()
            .expect("wallet request idempotency key");

        assert!(!idempotency_key.trim().is_empty());
        assert!(idempotency_key.len() <= 64);

        let memo = request.memo.as_deref().expect("wallet request memo");
        assert!(memo.starts_with("svc-rewarder:"));
    }
}
