//! RO:WHAT — Phase 1 Round 2 tests proving ron-accounting cannot become root-material or balance authority.
//! RO:WHY — Accounting artifacts may be referenced later, but accounting must remain derivative reporting/snapshot infrastructure.
//! RO:INTERACTS — Cargo manifest, src tree, RewardSnapshotExport, ProjectedRewardSnapshot.
//! RO:INVARIANTS — no ron-ledger/ron-proto root builders; no balance/root/finality truth; no wallet/ledger mutation; no proof or settlement authority.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects root/proof poison fields and blocks accidental Phase 1 authority creep.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_phase1_root_material_non_authority.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    ProjectedRewardSnapshot, RewardContributionExport, RewardProjectionReport, RewardSnapshotExport,
};
use serde_json::{json, Value};

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
        1_777_309_851_000,
        "1000",
        vec![
            RewardContributionExport::new("acct_qc1r2_storage_a", 4096, 2048, 60),
            RewardContributionExport::new("acct_qc1r2_storage_b", 8192, 1024, 30),
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
        value.as_bytes()[3..]
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "expected lowercase hex only"
    );
}

fn assert_no_authority_key_recursive(value: &Value, forbidden_fragment: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert!(
                    !key.contains(forbidden_fragment),
                    "accounting artifact must not expose authority key fragment `{forbidden_fragment}` in key `{key}`"
                );
                assert_no_authority_key_recursive(nested, forbidden_fragment);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_authority_key_recursive(nested, forbidden_fragment);
            }
        }
        _ => {}
    }
}

#[test]
fn cargo_manifest_does_not_link_quickchain_root_or_wallet_authority_crates() {
    let manifest = read(crate_dir().join("Cargo.toml"));

    for forbidden in [
        "ron-proto",
        "ron_proto",
        "ron-ledger",
        "ron_ledger",
        "svc-wallet",
        "svc_wallet",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "ron-accounting must not directly link QuickChain root/wallet authority crate: {forbidden}"
        );
    }
}

#[test]
fn accounting_source_never_builds_or_verifies_phase1_root_material() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find ron-accounting Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "ron_proto",
            "ron_ledger",
            "QuickChainTreeMaterial",
            "QuickChainTreeRoot",
            "QuickChainTreeInclusionProof",
            "QuickChainReceiptHashPayload",
            "build_tree_material_batch",
            "compute_tree_root_from_batch",
            "build_tree_inclusion_proof_from_batch",
            "verify_tree_inclusion_proof",
            "QUICKCHAIN_RECEIPT_HASH_PAYLOAD_SCHEMA",
            "QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN",
            "root_hash",
            "receipt_root",
            "state_root",
            "accounting_root",
            "checkpoint_root",
            "validator_signature",
            "settlement_status",
            "finality",
        ] {
            assert!(
                !code.contains(forbidden),
                "ron-accounting source must not build/verify Phase 1 root material or authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}

#[test]
fn accounting_artifacts_reject_phase1_root_material_poison_fields() {
    let clean_snapshot =
        serde_json::to_value(sample_snapshot()).expect("reward snapshot should serialize");
    let clean_projected = serde_json::to_value(sample_projected_snapshot())
        .expect("projected snapshot should serialize");

    for field in [
        "tree",
        "tree_material",
        "sort_key_hex",
        "payload_schema",
        "payload_hash",
        "root_hash",
        "root_node_hash",
        "inclusion_proof",
        "proof_steps",
        "leaf_index",
        "receipt_root",
        "state_root",
        "accounting_root",
        "checkpoint_root",
        "settlement_status",
        "finality",
        "validator_signature",
    ] {
        let mut poisoned_snapshot = clean_snapshot.clone();
        poisoned_snapshot
            .as_object_mut()
            .expect("snapshot JSON should be an object")
            .insert(field.to_string(), json!("client-supplied-root-material"));

        assert!(
            serde_json::from_value::<RewardSnapshotExport>(poisoned_snapshot).is_err(),
            "RewardSnapshotExport must reject Phase 1 root/proof poison field: {field}"
        );

        let mut poisoned_contribution = clean_snapshot.clone();
        poisoned_contribution["contributions"][0]
            .as_object_mut()
            .expect("contribution JSON should be an object")
            .insert(field.to_string(), json!("client-supplied-root-material"));

        assert!(
            serde_json::from_value::<RewardSnapshotExport>(poisoned_contribution).is_err(),
            "RewardContributionExport must reject nested Phase 1 root/proof poison field: {field}"
        );

        let mut poisoned_projected = clean_projected.clone();
        poisoned_projected
            .as_object_mut()
            .expect("projected snapshot JSON should be an object")
            .insert(field.to_string(), json!("client-supplied-root-material"));

        assert!(
            serde_json::from_value::<ProjectedRewardSnapshot>(poisoned_projected).is_err(),
            "ProjectedRewardSnapshot must reject Phase 1 root/proof poison field: {field}"
        );
    }
}

#[test]
fn accounting_snapshot_cid_is_artifact_reference_not_balance_or_root_truth() {
    let projected = sample_projected_snapshot();

    assert_b3_artifact_cid(&projected.snapshot_cid);

    let value = serde_json::to_value(&projected).expect("projected snapshot should serialize");
    let object = value
        .as_object()
        .expect("projected snapshot JSON should be an object");

    assert!(
        object.contains_key("snapshot_cid"),
        "accounting may expose an artifact CID for reporting"
    );

    for forbidden_fragment in [
        "balance",
        "receipt",
        "root",
        "checkpoint",
        "proof",
        "validator",
        "settlement",
        "finality",
        "anchor",
        "bridge",
        "wallet",
        "ledger",
        "mutation",
    ] {
        assert_no_authority_key_recursive(&value, forbidden_fragment);
    }

    let canonical_bytes = projected
        .snapshot
        .canonical_bytes()
        .expect("snapshot canonical bytes should compute");
    let canonical_json =
        String::from_utf8(canonical_bytes).expect("snapshot canonical bytes should be UTF-8 JSON");

    for forbidden in [
        "root_hash",
        "receipt_root",
        "state_root",
        "accounting_root",
        "checkpoint_root",
        "inclusion_proof",
        "finality",
        "settlement",
        "balance",
        "wallet_mutation",
        "ledger_mutation",
    ] {
        assert!(
            !canonical_json.contains(forbidden),
            "accounting canonical bytes must not contain root/balance authority vocabulary: {forbidden}"
        );
    }
}
