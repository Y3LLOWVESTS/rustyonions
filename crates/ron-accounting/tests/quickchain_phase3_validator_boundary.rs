//! RO:WHAT — Phase 3 Round 1 passport-gated validator boundary tests for ron-accounting.
//! RO:WHY — Accounting snapshots may be future verifier inputs, but accounting must not become validator membership, passport registry, capability, staking, slashing, wallet, or ledger authority.
//! RO:INTERACTS — RewardSnapshotExport, ProjectedRewardSnapshot, RewardProjectionReport, crate source/Cargo boundary.
//! RO:INVARIANTS — accounting remains derivative metering/snapshot infrastructure; no validator admission, passport registry authority, payout execution, staking, or slashing.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects Phase 3 validator/passport authority fields and blocks validator-economy authority creep.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_phase3_validator_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    canonical_snapshot_cid, ProjectedRewardSnapshot, RewardContributionExport,
    RewardProjectionReport, RewardSnapshotExport,
};
use serde_json::{json, Value};

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

fn sample_snapshot() -> RewardSnapshotExport {
    RewardSnapshotExport::new(
        1,
        "1000",
        vec![
            RewardContributionExport::new("acct_phase3_b", 200, 0, 20),
            RewardContributionExport::new("acct_phase3_a", 100, 50, 10),
        ],
    )
    .expect("sample reward snapshot should validate")
}

fn sample_projected_snapshot() -> ProjectedRewardSnapshot {
    let snapshot = sample_snapshot();
    let snapshot_cid = canonical_snapshot_cid(&snapshot).expect("snapshot cid should compute");

    ProjectedRewardSnapshot {
        snapshot,
        snapshot_cid,
        report: RewardProjectionReport {
            input_slices: 1,
            input_rows: 3,
            projected_accounts: 2,
            bytes_stored: 300,
            bytes_served: 50,
            uptime_seconds: 30,
            ignored_rows: 1,
        },
    }
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
                        "accounting artifact must not expose Phase 3 validator/passport authority key `{key}`"
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
fn accounting_reward_artifacts_remain_snapshots_not_validator_membership_or_passport_authority() {
    let projected = sample_projected_snapshot();
    let value = serde_json::to_value(&projected).expect("projected snapshot should serialize");

    assert_no_phase3_authority_key_recursive(&value);

    let object = value
        .as_object()
        .expect("projected snapshot JSON should be an object");

    assert!(
        object.contains_key("snapshot"),
        "accounting may expose a derivative snapshot"
    );
    assert!(
        object.contains_key("snapshot_cid"),
        "accounting may expose an artifact CID"
    );
    assert!(
        object.contains_key("report"),
        "accounting may expose projection report metadata"
    );
}

#[test]
fn accounting_snapshot_dtos_reject_phase3_validator_and_passport_authority_fields() {
    let clean_snapshot = serde_json::to_value(sample_snapshot()).expect("snapshot JSON");
    let clean_projected =
        serde_json::to_value(sample_projected_snapshot()).expect("projected JSON");

    for field in PHASE3_VALIDATOR_AUTHORITY_KEYS {
        let mut poisoned_snapshot = clean_snapshot.clone();
        poisoned_snapshot
            .as_object_mut()
            .expect("snapshot JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );

        assert!(
            serde_json::from_value::<RewardSnapshotExport>(poisoned_snapshot).is_err(),
            "RewardSnapshotExport must reject Phase 3 validator/passport authority field: {field}"
        );

        let mut poisoned_contribution = clean_snapshot.clone();
        poisoned_contribution["contributions"][0]
            .as_object_mut()
            .expect("contribution JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );

        assert!(
            serde_json::from_value::<RewardSnapshotExport>(poisoned_contribution).is_err(),
            "RewardContributionExport must reject nested Phase 3 validator/passport authority field: {field}"
        );

        let mut poisoned_projected = clean_projected.clone();
        poisoned_projected
            .as_object_mut()
            .expect("projected JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );

        assert!(
            serde_json::from_value::<ProjectedRewardSnapshot>(poisoned_projected).is_err(),
            "ProjectedRewardSnapshot must reject Phase 3 validator/passport authority field: {field}"
        );

        let mut poisoned_report = clean_projected.clone();
        poisoned_report["report"]
            .as_object_mut()
            .expect("report JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-validator-authority"),
            );

        assert!(
            serde_json::from_value::<ProjectedRewardSnapshot>(poisoned_report).is_err(),
            "RewardProjectionReport must reject nested Phase 3 validator/passport authority field: {field}"
        );
    }
}

#[test]
fn accounting_manifest_keeps_passport_registry_auth_wallet_and_ledger_crates_out_of_authority_path()
{
    let manifest = read(crate_dir().join("Cargo.toml"));

    for forbidden in [
        "svc-passport",
        "svc_passport",
        "svc-registry",
        "svc_registry",
        "ron-auth",
        "ron_auth",
        "svc-wallet",
        "svc_wallet",
        "ron-ledger",
        "ron_ledger",
        "ron-proto",
        "ron_proto",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "ron-accounting must not link Phase 3 or economic authority crate in this round: {forbidden}"
        );
    }
}

#[test]
fn accounting_source_does_not_implement_phase3_validator_or_passport_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find ron-accounting Rust files"
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
            "svc_passport",
            "svc_registry",
            "ron_auth::",
            "wallet_issue",
            "ledger_commit",
            "mutate_wallet",
            "mutate_ledger",
        ] {
            assert!(
                !code.contains(forbidden),
                "ron-accounting source must not implement Phase 3 validator/passport/economic authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
