//! RO:WHAT — Phase 2 Round 2 committee-readiness boundary tests for ron-accounting.
//! RO:WHY — Accounting snapshots may be verifier inputs later, but accounting must not become committee, quorum, attestation, fork-choice, finality, settlement, or validator-economy authority.
//! RO:INTERACTS — RewardSnapshotExport, ProjectedRewardSnapshot, RewardProjectionReport, crate source boundary.
//! RO:INVARIANTS — accounting remains derivative metering/snapshot infrastructure; committee attestations are not accounting artifacts or balance truth.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects Phase 2 Round 2 committee/quorum poison fields and blocks validator-economy authority creep.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_phase2_committee_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    ProjectedRewardSnapshot, RewardContributionExport, RewardProjectionReport, RewardSnapshotExport,
};
use serde_json::{json, Value};

const COMMITTEE_AUTHORITY_KEYS: &[&str] = &[
    "committee_member_id",
    "committee_epoch",
    "committee_round",
    "committee_signature",
    "committee_signatures",
    "signed_verification_attestation",
    "verification_attestation",
    "attestation_signature",
    "attestation_public_key",
    "attestation_weight",
    "quorum_certificate",
    "quorum_threshold",
    "quorum_reached",
    "validator_signature",
    "validator_set",
    "validator_index",
    "fork_choice",
    "double_attestation_evidence",
    "equivocation_evidence",
    "bonded_stake",
    "stake_weight",
    "slash_evidence",
    "slashing",
    "external_anchor",
    "external_settlement",
    "bridge_finality",
    "settlement_finality",
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
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with("///"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sample_snapshot() -> RewardSnapshotExport {
    RewardSnapshotExport::new(
        1_777_500_000_000,
        "1000",
        vec![
            RewardContributionExport::new("acct_phase2_round2_storage_a", 4096, 2048, 60),
            RewardContributionExport::new("acct_phase2_round2_storage_b", 8192, 1024, 30),
        ],
    )
    .expect("sample reward snapshot should validate")
}

fn sample_projected_snapshot() -> ProjectedRewardSnapshot {
    let snapshot = sample_snapshot();
    let snapshot_cid = snapshot
        .canonical_cid()
        .expect("sample snapshot CID should compute");

    ProjectedRewardSnapshot {
        snapshot,
        snapshot_cid,
        report: RewardProjectionReport {
            input_slices: 1,
            input_rows: 2,
            projected_accounts: 2,
            bytes_stored: 12_288,
            bytes_served: 3_072,
            uptime_seconds: 90,
            ignored_rows: 0,
        },
    }
}

fn assert_b3_artifact_cid(value: &str) {
    assert_eq!(value.len(), 67, "expected b3:<64 lowercase hex>");
    assert!(value.starts_with("b3:"), "expected b3 prefix");
    assert!(
        value[3..]
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "expected lowercase hex only"
    );
}

fn assert_no_key_recursive(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert!(
                    key != forbidden,
                    "accounting artifact must not expose Phase 2 Round 2 committee authority key `{forbidden}`"
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

#[test]
fn accounting_artifacts_reject_committee_attestation_poison_fields() {
    let clean_snapshot =
        serde_json::to_value(sample_snapshot()).expect("reward snapshot should serialize");
    let clean_projected = serde_json::to_value(sample_projected_snapshot())
        .expect("projected snapshot should serialize");

    for field in COMMITTEE_AUTHORITY_KEYS {
        let mut poisoned_snapshot = clean_snapshot.clone();
        poisoned_snapshot
            .as_object_mut()
            .expect("snapshot JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-committee-authority"),
            );

        assert!(
            serde_json::from_value::<RewardSnapshotExport>(poisoned_snapshot).is_err(),
            "RewardSnapshotExport must reject Phase 2 Round 2 committee poison field: {field}"
        );

        let mut poisoned_contribution = clean_snapshot.clone();
        poisoned_contribution["contributions"][0]
            .as_object_mut()
            .expect("contribution JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-committee-authority"),
            );

        assert!(
            serde_json::from_value::<RewardSnapshotExport>(poisoned_contribution).is_err(),
            "RewardContributionExport must reject nested Phase 2 Round 2 committee poison field: {field}"
        );

        let mut poisoned_projected = clean_projected.clone();
        poisoned_projected
            .as_object_mut()
            .expect("projected JSON should be object")
            .insert(
                (*field).to_string(),
                json!("client-supplied-committee-authority"),
            );

        assert!(
            serde_json::from_value::<ProjectedRewardSnapshot>(poisoned_projected).is_err(),
            "ProjectedRewardSnapshot must reject Phase 2 Round 2 committee poison field: {field}"
        );
    }
}

#[test]
fn accounting_snapshot_cid_remains_artifact_not_committee_attestation() {
    let projected = sample_projected_snapshot();

    assert_b3_artifact_cid(&projected.snapshot_cid);

    let value = serde_json::to_value(&projected).expect("projected snapshot should serialize");

    for forbidden in COMMITTEE_AUTHORITY_KEYS {
        assert_no_key_recursive(&value, forbidden);
    }

    let canonical_bytes = projected
        .snapshot
        .canonical_bytes()
        .expect("snapshot canonical bytes should compute");
    let canonical_json =
        String::from_utf8(canonical_bytes).expect("snapshot canonical bytes should be UTF-8 JSON");

    for forbidden in [
        "committee_member_id",
        "committee_signature",
        "signed_verification_attestation",
        "quorum_certificate",
        "validator_signature",
        "fork_choice",
        "bonded_stake",
        "slash_evidence",
        "external_settlement",
        "settlement_finality",
    ] {
        assert!(
            !canonical_json.contains(forbidden),
            "accounting canonical bytes must not contain Phase 2 Round 2 committee/finality authority vocabulary: {forbidden}"
        );
    }
}

#[test]
fn accounting_source_does_not_implement_committee_or_validator_economy_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find ron-accounting Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "QuickChainCommittee",
            "CommitteeAttestation",
            "SignedVerificationAttestation",
            "QuorumCertificate",
            "committee_member_id",
            "committee_epoch",
            "committee_round",
            "signed_verification_attestation",
            "verification_attestation",
            "attestation_signature",
            "quorum_certificate",
            "quorum_threshold",
            "quorum_reached",
            "validator_set",
            "validator_signature",
            "fork_choice",
            "double_attestation_evidence",
            "equivocation_evidence",
            "bonded_stake",
            "stake_weight",
            "slash_evidence",
            "bridge_finality",
            "settlement_finality",
        ] {
            assert!(
                !code.contains(forbidden),
                "ron-accounting source must not implement Phase 2 Round 2 committee/validator-economy authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
