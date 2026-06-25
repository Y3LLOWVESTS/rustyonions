#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 3 Round 2 validator lifecycle boundary tests for svc-rewarder.
//! RO:WHY — Validator rotation, revocation, equivocation evidence, replay challenges, downtime/degraded status, and governance parameter updates must not become payout, wallet, ledger, staking, slashing, bridge, or settlement authority.
//! RO:INTERACTS — ComputeEpochRequest, AccountingSnapshot, RewardPolicy, RewardManifest, WalletIssueRequest, source boundary.
//! RO:INVARIANTS — svc-rewarder remains deterministic payout planning only; svc-wallet remains mutation front-door; ron-ledger remains truth.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects validator lifecycle authority smuggling and preserves no direct mutation / no fake receipt / no validator economy boundaries.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase3_validator_lifecycle_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use svc_rewarder::{
    core::{compute_manifest, AmountMinor, ComputeInput},
    http::dto::ComputeEpochRequest,
    inputs::{
        canonical_snapshot_cid, AccountContribution, AccountingSnapshot, ContentCid, RewardPolicy,
    },
    outputs::{IntentResult, RewardManifest, SettlementBatch, WalletIssueRequest},
};

const POLICY_ID: &str = "policy:v1";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const IDEMPOTENCY_SALT: &str = "svc-rewarder|phase3-round2-lifecycle";

const PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS: &[&str] = &[
    "validator_rotation",
    "validator_rotation_epoch",
    "validator_rotation_decision",
    "validator_revocation",
    "validator_revocation_reason",
    "validator_revocation_decision",
    "validator_lifecycle_decision",
    "validator_lifecycle_status",
    "validator_lifecycle_rejection_code",
    "equivocation_evidence",
    "double_attestation_evidence",
    "split_brain_evidence",
    "replay_challenge_evidence",
    "invalid_attestation_evidence",
    "validator_downtime_status",
    "validator_degraded_status",
    "downtime_report",
    "governance_parameter_update",
    "validator_set_parameter_update",
    "quorum_parameter_update",
    "checkpoint_parameter_update",
    "slash_evidence",
    "slashing",
    "staking_power",
    "validator_bond",
    "bonded_economics",
    "validator_reward",
    "bridge_settlement",
    "external_settlement",
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn collect_rs_files(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "target")
        {
            continue;
        }

        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "rs")
        {
            files.push(path);
        }
    }
}

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_no_key_recursive(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert!(
                    key != forbidden,
                    "rewarder JSON artifact must not expose Phase 3 Round 2 lifecycle authority key `{forbidden}`"
                );
                assert_no_key_recursive(nested, forbidden);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_key_recursive(nested, forbidden);
            }
        }
        _ => {}
    }
}

fn snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_b".into(),
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

fn snapshot_value() -> Value {
    serde_json::to_value(snapshot()).expect("snapshot serializes")
}

fn inputs_cid_for(snapshot: AccountingSnapshot) -> ContentCid {
    let cid = canonical_snapshot_cid(snapshot).expect("canonical snapshot cid");
    ContentCid::parse(cid).expect("canonical cid parses")
}

fn valid_compute_request_json() -> Value {
    let snapshot = snapshot_value();
    let parsed_snapshot =
        serde_json::from_value::<AccountingSnapshot>(snapshot.clone()).expect("snapshot parses");
    let inputs_cid = canonical_snapshot_cid(parsed_snapshot).expect("snapshot cid");

    json!({
        "inputs_cid": inputs_cid,
        "policy_id": POLICY_ID,
        "policy_hash": POLICY_HASH,
        "dry_run": true,
        "snapshot": snapshot,
        "policy": {
            "id": POLICY_ID,
            "hash": POLICY_HASH,
            "signed": true,
            "funding_source": "protocol_pool",
            "max_payout_minor_units": "1000",
            "min_payout_minor_units": "1",
            "weight_bps": 10000,
            "rounding": "floor"
        }
    })
}

fn manifest() -> RewardManifest {
    let snapshot = snapshot();
    compute_manifest(
        ComputeInput {
            epoch_id: "epoch-qc-phase3-round2".into(),
            inputs_cid: inputs_cid_for(snapshot.clone()),
            policy: RewardPolicy::dev_default(POLICY_ID, POLICY_HASH),
            snapshot,
            dry_run: true,
            idempotency_salt: IDEMPOTENCY_SALT.into(),
        },
        IntentResult::DryRun,
    )
    .expect("manifest computes")
}

#[test]
fn rewarder_compute_request_rejects_validator_lifecycle_authority_fields() {
    let clean = valid_compute_request_json();

    serde_json::from_value::<ComputeEpochRequest>(clean.clone())
        .expect("clean compute request should deserialize");

    for field in PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("compute request JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-lifecycle-authority"),
            );

        assert!(
            serde_json::from_value::<ComputeEpochRequest>(poisoned).is_err(),
            "ComputeEpochRequest must reject Phase 3 Round 2 lifecycle authority field: {field}"
        );
    }
}

#[test]
fn rewarder_input_dtos_reject_validator_lifecycle_authority_fields() {
    let clean_snapshot = snapshot_value();
    let clean_policy = json!({
        "id": POLICY_ID,
        "hash": POLICY_HASH,
        "signed": true,
        "funding_source": "protocol_pool",
        "max_payout_minor_units": "1000",
        "min_payout_minor_units": "1",
        "weight_bps": 10000,
        "rounding": "floor"
    });

    serde_json::from_value::<AccountingSnapshot>(clean_snapshot.clone())
        .expect("clean accounting snapshot should deserialize");
    serde_json::from_value::<RewardPolicy>(clean_policy.clone())
        .expect("clean reward policy should deserialize");

    for field in PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS {
        let mut poisoned_snapshot = clean_snapshot.clone();
        poisoned_snapshot
            .as_object_mut()
            .expect("snapshot JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-lifecycle-authority"),
            );

        assert!(
            serde_json::from_value::<AccountingSnapshot>(poisoned_snapshot).is_err(),
            "AccountingSnapshot must reject Phase 3 Round 2 lifecycle authority field: {field}"
        );

        let mut poisoned_policy = clean_policy.clone();
        poisoned_policy
            .as_object_mut()
            .expect("policy JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-lifecycle-authority"),
            );

        assert!(
            serde_json::from_value::<RewardPolicy>(poisoned_policy).is_err(),
            "RewardPolicy must reject Phase 3 Round 2 lifecycle authority field: {field}"
        );

        let mut nested_snapshot = clean_snapshot.clone();
        nested_snapshot["contributions"][0]
            .as_object_mut()
            .expect("contribution JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-lifecycle-authority"),
            );

        assert!(
            serde_json::from_value::<AccountingSnapshot>(nested_snapshot).is_err(),
            "AccountingSnapshot contribution must reject Phase 3 Round 2 lifecycle authority field: {field}"
        );
    }
}

#[test]
fn rewarder_outputs_remain_planning_artifacts_not_lifecycle_or_payout_execution_authority() {
    let manifest = manifest();
    let manifest_json = serde_json::to_value(&manifest).expect("manifest should serialize");
    let settlement =
        SettlementBatch::from_manifest(&manifest).expect("settlement batch should plan");
    let wallet_batch = settlement.to_wallet_issue_batch();
    let wallet_batch_json =
        serde_json::to_value(&wallet_batch).expect("wallet batch should serialize");

    for field in PHASE3_ROUND2_LIFECYCLE_AUTHORITY_KEYS {
        assert_no_key_recursive(&manifest_json, field);
        assert_no_key_recursive(&wallet_batch_json, field);

        let mut poisoned_manifest = manifest_json
            .as_object()
            .expect("manifest should be object")
            .clone();
        poisoned_manifest.insert(
            (*field).to_string(),
            json!("client-supplied-lifecycle-authority"),
        );

        assert!(
            serde_json::from_value::<RewardManifest>(Value::Object(poisoned_manifest)).is_err(),
            "RewardManifest must reject Phase 3 Round 2 lifecycle authority field: {field}"
        );

        let mut poisoned_issue = serde_json::to_value(
            wallet_batch
                .requests
                .first()
                .expect("wallet batch should contain issue requests"),
        )
        .expect("wallet issue request should serialize");

        poisoned_issue
            .as_object_mut()
            .expect("wallet issue request should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-lifecycle-authority"),
            );

        assert!(
            serde_json::from_value::<WalletIssueRequest>(poisoned_issue).is_err(),
            "WalletIssueRequest handoff must reject Phase 3 Round 2 lifecycle authority field: {field}"
        );
    }

    for forbidden in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "wallet_mutation",
        "ledger_mutation",
        "payout_executed",
        "paid_unlock",
        "settlement_finality",
        "finalized",
        "anchored",
    ] {
        assert_no_key_recursive(&manifest_json, forbidden);
        assert_no_key_recursive(&wallet_batch_json, forbidden);
    }
}

#[test]
fn rewarder_source_does_not_construct_validator_lifecycle_runtime_or_economic_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-rewarder Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "validator_lifecycle_decision",
            "validator_lifecycle_status",
            "validator_rotation_decision",
            "validator_revocation_decision",
            "equivocation_evidence",
            "double_attestation_evidence",
            "split_brain_evidence",
            "replay_challenge_evidence",
            "invalid_attestation_evidence",
            "downtime_report",
            "validator_degraded_status",
            "governance_parameter_update",
            "admit_validator",
            "rotate_validator",
            "revoke_validator",
            "slash_validator",
            "stake_validator",
            "validator_reward",
            "mint_from_validation",
            "reward_from_validator_signature",
            "payout_from_lifecycle",
            "bridge_settlement",
            "external_settlement",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-rewarder source must not construct Phase 3 Round 2 validator lifecycle/economy authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
