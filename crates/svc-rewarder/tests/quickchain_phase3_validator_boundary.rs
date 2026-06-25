//! RO:WHAT — Phase 3 Round 1 passport-gated validator boundary tests for svc-rewarder.
//! RO:WHY — svc-rewarder plans deterministic payouts; validator identity, passport admission, registry membership, and validator capability authorization must not become payout, wallet, ledger, staking, slashing, or settlement authority.
//! RO:INTERACTS — ComputeEpochRequest, AccountingSnapshot, RewardPolicy, RewardManifest, WalletIssueRequest, source/Cargo boundary.
//! RO:INVARIANTS — rewarder remains planning-only; no validator admission, passport registry authority, direct wallet/ledger mutation, bonded economics, staking, slashing, roots, checkpoints, bridge, or external settlement.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects Phase 3 validator/passport authority fields and prevents validator-economy creep.
//! RO:TEST — cargo test -p svc-rewarder --test quickchain_phase3_validator_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use svc_rewarder::{
    core::{compute_manifest, AmountMinor, ComputeInput},
    http::dto::ComputeEpochRequest,
    inputs::{
        canonical_snapshot_cid, AccountContribution, AccountingSnapshot, ContentCid,
        RewardFundingSource, RewardPolicy,
    },
    outputs::{IntentResult, RewardManifest, WalletIssueRequest},
};

const POLICY_ID: &str = "policy:v1";
const POLICY_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const INPUTS_CID_HEX: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const PHASE3_VALIDATOR_AUTHORITY_KEYS: &[&str] = &[
    "validator_passport_subject",
    "validator_capability",
    "validator_capability_scope",
    "validator_capability_id",
    "validator_set_hash",
    "validator_set_version",
    "validator_registry_epoch",
    "validator_lifecycle_status",
    "validator_admission_rule",
    "validator_revocation_rule",
    "validator_rotation_epoch",
    "passport_registry_proof",
    "passport_admission_proof",
    "passport_revocation_proof",
    "registry_membership_proof",
    "registry_admission_proof",
    "capability_not_before_ms",
    "capability_expires_at_ms",
    "capability_rotation_proof",
    "passport_required",
    "bond_required",
    "bonded_economics",
    "validator_bond",
    "staking_power",
    "slash_evidence",
    "slashing",
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

fn strip_line_comments(input: &str) -> String {
    input
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

fn valid_snapshot_value() -> Value {
    json!({
        "produced_at_millis": 1,
        "pool_minor_units": "1000",
        "contributions": [
            {
                "account": "acct_phase3_rewarder_a",
                "bytes_stored": 100,
                "bytes_served": 40,
                "uptime_seconds": 10
            }
        ]
    })
}

fn valid_inputs_cid(snapshot: &Value) -> String {
    let parsed = serde_json::from_value::<AccountingSnapshot>(snapshot.clone())
        .expect("valid snapshot parses");
    canonical_snapshot_cid(parsed).expect("snapshot cid")
}

fn valid_compute_body() -> Value {
    let snapshot = valid_snapshot_value();
    let inputs_cid = valid_inputs_cid(&snapshot);

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

fn valid_policy() -> RewardPolicy {
    RewardPolicy {
        id: POLICY_ID.to_string(),
        hash: POLICY_HASH.to_string(),
        signed: true,
        funding_source: RewardFundingSource::ProtocolPool,
        max_payout_minor_units: AmountMinor(1_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".to_string(),
    }
}

fn valid_snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![AccountContribution {
            account: "acct_phase3_rewarder_a".to_string(),
            bytes_stored: 100,
            bytes_served: 40,
            uptime_seconds: 10,
        }],
    }
}

fn valid_manifest() -> RewardManifest {
    let input = ComputeInput {
        epoch_id: "epoch-phase3-rewarder-boundary".to_string(),
        inputs_cid: ContentCid::parse(format!("b3:{INPUTS_CID_HEX}")).expect("valid cid"),
        policy: valid_policy(),
        snapshot: valid_snapshot(),
        dry_run: true,
        idempotency_salt: "svc-rewarder|phase3-validator-boundary".to_string(),
    };

    compute_manifest(input, IntentResult::DryRun).expect("manifest should compute")
}

fn assert_no_phase3_authority_key_recursive(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                for forbidden in [
                    "validator",
                    "passport",
                    "registry",
                    "capability",
                    "bond",
                    "stake",
                    "slash",
                    "staking",
                    "slashing",
                ] {
                    assert!(
                        !key.contains(forbidden),
                        "rewarder planning artifact must not expose Phase 3 authority key `{key}`"
                    );
                }
                assert_no_phase3_authority_key_recursive(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_phase3_authority_key_recursive(item);
            }
        }
        _ => {}
    }
}

#[test]
fn rewarder_compute_request_rejects_phase3_validator_passport_authority_fields() {
    for field in PHASE3_VALIDATOR_AUTHORITY_KEYS {
        let mut body = valid_compute_body();
        body.as_object_mut()
            .expect("compute body should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );

        assert!(
            serde_json::from_value::<ComputeEpochRequest>(body).is_err(),
            "ComputeEpochRequest must reject Phase 3 validator/passport authority field: {field}"
        );
    }
}

#[test]
fn rewarder_nested_snapshot_and_policy_reject_phase3_validator_authority_fields() {
    for field in PHASE3_VALIDATOR_AUTHORITY_KEYS {
        let mut policy = json!({
            "id": POLICY_ID,
            "hash": POLICY_HASH,
            "signed": true,
            "funding_source": "protocol_pool",
            "max_payout_minor_units": "1000",
            "min_payout_minor_units": "1",
            "weight_bps": 10000,
            "rounding": "floor"
        });
        policy
            .as_object_mut()
            .expect("policy JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );
        assert!(
            serde_json::from_value::<RewardPolicy>(policy).is_err(),
            "RewardPolicy must reject Phase 3 validator/passport authority field: {field}"
        );

        let mut snapshot = valid_snapshot_value();
        snapshot
            .as_object_mut()
            .expect("snapshot JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );
        assert!(
            serde_json::from_value::<AccountingSnapshot>(snapshot).is_err(),
            "AccountingSnapshot must reject Phase 3 validator/passport authority field: {field}"
        );

        let mut contribution = valid_snapshot_value();
        contribution["contributions"][0]
            .as_object_mut()
            .expect("contribution JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );
        assert!(
            serde_json::from_value::<AccountingSnapshot>(contribution).is_err(),
            "AccountContribution must reject nested Phase 3 validator/passport authority field: {field}"
        );
    }
}

#[test]
fn reward_manifest_remains_planning_artifact_not_validator_membership_or_passport_authority() {
    let manifest = valid_manifest();
    let value = serde_json::to_value(&manifest).expect("manifest should serialize");

    assert_no_phase3_authority_key_recursive(&value);

    let object = value
        .as_object()
        .expect("manifest JSON should be an object");

    for required in [
        "version",
        "epoch_id",
        "run_key",
        "commitment",
        "status",
        "inputs_cid",
        "totals",
        "policy",
        "invariants",
        "ledger",
        "payouts",
        "attestation",
    ] {
        assert!(
            object.contains_key(required),
            "manifest should preserve planning artifact field: {required}"
        );
    }
}

#[test]
fn reward_manifest_rejects_phase3_validator_passport_authority_fields() {
    for field in PHASE3_VALIDATOR_AUTHORITY_KEYS {
        let mut value = serde_json::to_value(valid_manifest()).expect("manifest JSON");
        value
            .as_object_mut()
            .expect("manifest JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );

        assert!(
            serde_json::from_value::<RewardManifest>(value).is_err(),
            "RewardManifest must reject Phase 3 validator/passport authority field: {field}"
        );
    }
}

#[test]
fn rewarder_wallet_handoff_preview_has_no_phase3_validator_authority_keys() {
    let issue = WalletIssueRequest {
        to: "acct_phase3_rewarder_preview".to_string(),
        asset: "roc".to_string(),
        amount_minor: "123".to_string(),
        idempotency_key: Some("idem_phase3_rewarder_preview".to_string()),
        memo: Some("svc-rewarder:phase3:preview".to_string()),
    };

    let value = serde_json::to_value(issue).expect("wallet issue preview should serialize");

    assert_no_phase3_authority_key_recursive(&value);
}

#[test]
fn rewarder_manifest_keeps_passport_registry_and_auth_crates_out_of_planning_path() {
    let manifest = read(crate_dir().join("Cargo.toml"));

    for forbidden in [
        "svc-passport",
        "svc_passport",
        "svc-registry",
        "svc_registry",
        "ron-auth",
        "ron_auth",
        "ron-ledger",
        "ron_ledger",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "svc-rewarder must not link Phase 3 passport/registry/auth/ledger authority crate in this round: {forbidden}"
        );
    }
}

#[test]
fn rewarder_source_does_not_implement_phase3_validator_or_passport_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-rewarder Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "QuickChainValidator",
            "ValidatorCapability",
            "ValidatorSet",
            "ValidatorAdmission",
            "ValidatorRevocation",
            "validator_set_hash",
            "validator_passport_subject",
            "validator_capability",
            "validator_capability_scope",
            "validator_registry_epoch",
            "passport_admission_proof",
            "passport_revocation_proof",
            "registry_membership_proof",
            "registry_admission_proof",
            "passport_required",
            "bond_required",
            "bonded_economics",
            "validator_bond",
            "staking_power",
            "admit_validator",
            "revoke_validator",
            "rotate_validator",
            "slash_validator",
            "mint_from_validation",
            "reward_from_validator_signature",
            "svc_passport",
            "svc_registry",
            "ron_auth::",
            "ron_ledger::",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-rewarder source must not implement Phase 3 validator/passport authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
