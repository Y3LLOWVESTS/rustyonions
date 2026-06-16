//! RO:WHAT — QuickChain Phase-0 replay/no-double-issue tests for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/GOV. Deterministic planning must not imply repeated payout authority.
//! RO:INTERACTS — core::compute, inputs, outputs::SettlementBatch, outputs::IntentStore.
//! RO:INVARIANTS — same input same plan; reordered rows same plan; replay is dup/dry-run, not second authority.
//! RO:METRICS — none; pure replay/idempotency tests.
//! RO:CONFIG — uses deterministic test salt.
//! RO:SECURITY — proves idempotency is retry/dedupe safety, not economic authority.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue.

use svc_rewarder::core::{compute_manifest, AmountMinor, ComputeInput};
use svc_rewarder::inputs::{
    canonical_snapshot_cid, AccountContribution, AccountingSnapshot, ContentCid, RewardPolicy,
};
use svc_rewarder::outputs::{IntentResult, IntentStore, SettlementBatch};

const POLICY_ID: &str = "policy:v1";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const IDEMPOTENCY_SALT: &str = "svc-rewarder|quickchain-preflight";

fn snapshot_ab() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 100,
                bytes_served: 50,
                uptime_seconds: 10,
            },
            AccountContribution {
                account: "acct_b".into(),
                bytes_stored: 200,
                bytes_served: 0,
                uptime_seconds: 20,
            },
        ],
    }
}

fn snapshot_ba_with_whitespace() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: " acct_b ".into(),
                bytes_stored: 200,
                bytes_served: 0,
                uptime_seconds: 20,
            },
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 100,
                bytes_served: 50,
                uptime_seconds: 10,
            },
        ],
    }
}

fn content_cid_for(snapshot: AccountingSnapshot) -> ContentCid {
    let cid = canonical_snapshot_cid(snapshot).expect("canonical snapshot cid");
    ContentCid::parse(cid).expect("cid parses")
}

fn manifest_for(
    epoch_id: &str,
    snapshot: AccountingSnapshot,
    egress: IntentResult,
) -> svc_rewarder::outputs::RewardManifest {
    compute_manifest(
        ComputeInput {
            epoch_id: epoch_id.into(),
            inputs_cid: content_cid_for(snapshot.clone()),
            policy: RewardPolicy::dev_default(POLICY_ID, POLICY_HASH),
            snapshot,
            dry_run: matches!(egress, IntentResult::DryRun),
            idempotency_salt: IDEMPOTENCY_SALT.into(),
        },
        egress,
    )
    .expect("manifest computes")
}

#[test]
fn same_snapshot_policy_and_epoch_produce_same_plan_commitment() {
    let first = manifest_for("epoch-replay-1", snapshot_ab(), IntentResult::DryRun);
    let second = manifest_for("epoch-replay-1", snapshot_ab(), IntentResult::DryRun);

    assert_eq!(first.run_key, second.run_key);
    assert_eq!(first.commitment, second.commitment);
    assert_eq!(first.inputs_cid, second.inputs_cid);
    assert_eq!(first.totals, second.totals);
    assert_eq!(first.payouts, second.payouts);
    assert_eq!(first.ledger.result, "dry_run");
    assert_eq!(second.ledger.result, "dry_run");
}

#[test]
fn reordered_snapshot_rows_produce_same_plan() {
    let canonical = manifest_for("epoch-replay-2", snapshot_ab(), IntentResult::DryRun);
    let reordered = manifest_for(
        "epoch-replay-2",
        snapshot_ba_with_whitespace(),
        IntentResult::DryRun,
    );

    assert_eq!(canonical.inputs_cid, reordered.inputs_cid);
    assert_eq!(canonical.run_key, reordered.run_key);
    assert_eq!(canonical.commitment, reordered.commitment);
    assert_eq!(canonical.totals, reordered.totals);
    assert_eq!(canonical.payouts, reordered.payouts);
    assert_eq!(canonical.payouts[0].account, "acct_a");
    assert_eq!(canonical.payouts[1].account, "acct_b");
}

#[test]
fn duplicate_epoch_replay_is_dedupe_not_second_payout_authority() {
    let manifest = manifest_for("epoch-replay-3", snapshot_ab(), IntentResult::Accepted);
    let settlement = SettlementBatch::from_manifest(&manifest).expect("settlement plans");
    let store = IntentStore::default();

    assert_eq!(
        store.emit_batch_once(&settlement, true).as_str(),
        "dry_run",
        "dry-run must not consume or authorize the run key"
    );
    assert_eq!(
        store.emit_batch_once(&settlement, false).as_str(),
        "accepted",
        "first non-dry-run may be accepted by the rewarder-local idempotency seam"
    );
    assert_eq!(
        store.emit_batch_once(&settlement, false).as_str(),
        "dup",
        "replay must be duplicate/dedupe, not second payout authority"
    );
}

#[test]
fn idempotency_keys_are_retry_dedupe_not_operation_identity() {
    let manifest = manifest_for("epoch-replay-4", snapshot_ab(), IntentResult::Accepted);
    let settlement = SettlementBatch::from_manifest(&manifest).expect("settlement plans");
    let wallet_batch = settlement.to_wallet_issue_batch();

    assert_eq!(wallet_batch.requests.len(), 2);

    for request in &wallet_batch.requests {
        let idempotency_key = request.idempotency_key.as_deref().expect("idempotency key");

        assert!(idempotency_key.starts_with("b3:"));
        assert!(idempotency_key.len() <= 64);
        assert!(
            !idempotency_key.contains("operation_id"),
            "rewarder must not pretend to assign durable ledger operation identity"
        );
    }

    let encoded = serde_json::to_string(&wallet_batch).expect("wallet batch serializes");

    for forbidden in [
        "operation_id",
        "account_sequence",
        "receipt_id",
        "finalized",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "wallet preview must not contain {forbidden}"
        );
    }
}
