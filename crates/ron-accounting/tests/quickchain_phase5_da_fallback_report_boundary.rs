#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 2 DA/archive/challenge fallback report boundary tests for ron-accounting.
//! RO:WHY — Accounting snapshots may be referenced by DA fallback metadata, but must not become balance, payout, reward, finality, deletion, paid unlock, or outside-truth authority.
//! RO:INTERACTS — QuickChainDaFallbackReport and RewardSnapshotExport artifact helpers.
//! RO:INVARIANTS — report-only; evidence-only; no wallet/ledger mutation; no payout execution; no paid unlock; deletion shortcuts blocked.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects authority flags and unknown fields.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_phase5_da_fallback_report_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    QuickChainDaFallbackReport, RewardContributionExport, RewardSnapshotExport,
    RON_ACCOUNTING_QUICKCHAIN_DA_FALLBACK_REPORT_SCHEMA,
};
use serde_json::{json, Value};

const CHECKPOINT_HASH: &str = "b3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const DA_ROOT: &str = "b3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

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

fn assert_no_key(value: &Value, key: &str) {
    match value {
        Value::Object(map) => {
            assert!(
                !map.contains_key(key),
                "serialized accounting DA fallback report must not expose authority key `{key}`: {value}"
            );
            for child in map.values() {
                assert_no_key(child, key);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_no_key(child, key);
            }
        }
        _ => {}
    }
}

fn sample_snapshot() -> RewardSnapshotExport {
    RewardSnapshotExport::new(
        1_777_600_000_000,
        "2000",
        vec![
            RewardContributionExport::new("acct_phase5_da_a", 100, 50, 10),
            RewardContributionExport::new("acct_phase5_da_b", 200, 0, 20),
        ],
    )
    .expect("sample snapshot should validate")
}

fn fallback_report() -> QuickChainDaFallbackReport {
    QuickChainDaFallbackReport::new_for_snapshot(
        1_777_600_001_000,
        "roc-dev",
        "epoch:phase5:r2",
        "ron-accounting:test",
        "fallback:plan:phase5:r2:accounting",
        CHECKPOINT_HASH,
        DA_ROOT,
        Some("chunk:accounting-snapshot:0001".to_string()),
        &sample_snapshot(),
    )
    .expect("fallback report should validate")
}

#[test]
fn accounting_da_fallback_report_is_read_only_and_blocks_pruning_authority() {
    let report = fallback_report();

    assert_eq!(
        report.schema,
        RON_ACCOUNTING_QUICKCHAIN_DA_FALLBACK_REPORT_SCHEMA
    );
    assert_eq!(report.checkpoint_hash, CHECKPOINT_HASH);
    assert_eq!(report.data_availability_root, DA_ROOT);
    assert!(report.accounting_snapshot_cid.starts_with("b3:"));
    assert_eq!(report.snapshot_contribution_count, 2);

    assert!(report.report_only);
    assert!(report.evidence_only);
    assert!(report.archive_fallback_checked);
    assert!(report.missing_data_challenge_checked);
    assert!(report.restore_path_checked);
    assert!(report.pruning_blocked);

    assert!(!report.balance_truth);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.payout_side_effect);
    assert!(!report.reward_truth);
    assert!(!report.terminality_truth);
    assert!(!report.external_claim_truth);
    assert!(!report.paid_unlock_authority);
    assert!(!report.pruning_authority);
    assert!(!report.outside_data_availability_truth);
    assert!(!report.outside_settlement);

    let value = serde_json::to_value(&report).expect("report should serialize");

    for forbidden in [
        "wallet_mutation",
        "ledger_mutation",
        "balance_mutation",
        "payout_executed",
        "paid_unlock",
        "settlement_status",
        "finalized",
        "anchored",
        "external_settlement",
        "bridge_settlement",
        "solana",
        "rox",
    ] {
        assert_no_key(&value, forbidden);
    }
}

#[test]
fn accounting_da_fallback_report_rejects_authority_flags_and_unknown_fields() {
    let clean = serde_json::to_value(fallback_report()).expect("report should serialize");

    for field in [
        "report_only",
        "evidence_only",
        "archive_fallback_checked",
        "missing_data_challenge_checked",
        "restore_path_checked",
        "pruning_blocked",
    ] {
        let mut poisoned = serde_json::from_value::<QuickChainDaFallbackReport>(clean.clone())
            .expect("clean report should deserialize");

        match field {
            "report_only" => poisoned.report_only = false,
            "evidence_only" => poisoned.evidence_only = false,
            "archive_fallback_checked" => poisoned.archive_fallback_checked = false,
            "missing_data_challenge_checked" => poisoned.missing_data_challenge_checked = false,
            "restore_path_checked" => poisoned.restore_path_checked = false,
            "pruning_blocked" => poisoned.pruning_blocked = false,
            _ => unreachable!("covered above"),
        }

        assert!(
            poisoned.validate().is_err(),
            "required blocker/check flag must reject when disabled: {field}"
        );
    }

    for field in [
        "balance_truth",
        "wallet_side_effect",
        "ledger_side_effect",
        "payout_side_effect",
        "reward_truth",
        "terminality_truth",
        "external_claim_truth",
        "paid_unlock_authority",
        "pruning_authority",
        "outside_data_availability_truth",
        "outside_settlement",
    ] {
        let mut poisoned = serde_json::from_value::<QuickChainDaFallbackReport>(clean.clone())
            .expect("clean report should deserialize");

        match field {
            "balance_truth" => poisoned.balance_truth = true,
            "wallet_side_effect" => poisoned.wallet_side_effect = true,
            "ledger_side_effect" => poisoned.ledger_side_effect = true,
            "payout_side_effect" => poisoned.payout_side_effect = true,
            "reward_truth" => poisoned.reward_truth = true,
            "terminality_truth" => poisoned.terminality_truth = true,
            "external_claim_truth" => poisoned.external_claim_truth = true,
            "paid_unlock_authority" => poisoned.paid_unlock_authority = true,
            "pruning_authority" => poisoned.pruning_authority = true,
            "outside_data_availability_truth" => poisoned.outside_data_availability_truth = true,
            "outside_settlement" => poisoned.outside_settlement = true,
            _ => unreachable!("covered above"),
        }

        assert!(
            poisoned.validate().is_err(),
            "authority flag must reject when enabled: {field}"
        );
    }

    for field in [
        "wallet_mutation",
        "ledger_mutation",
        "balance_minor",
        "available_minor",
        "wallet_receipt_created",
        "paid_unlock",
        "settlement_status",
        "finalized",
        "anchored",
        "external_settlement",
        "bridge_settlement",
        "solana",
        "rox",
    ] {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("report JSON should be object")
            .insert(field.to_string(), json!("client-supplied-da-authority"));

        assert!(
            serde_json::from_value::<QuickChainDaFallbackReport>(poisoned).is_err(),
            "report DTO must reject unknown authority field: {field}"
        );
    }
}

#[test]
fn accounting_da_fallback_report_rejects_bad_hashes_timestamps_and_empty_snapshot() {
    let mut report = fallback_report();
    report.checkpoint_hash = "not-a-b3-hash".to_string();
    assert!(
        report.validate().is_err(),
        "checkpoint hash must be canonical b3"
    );

    let mut report = fallback_report();
    report.data_availability_root = "b3:ABC".to_string();
    assert!(
        report.validate().is_err(),
        "DA root must be canonical b3 lowercase"
    );

    let mut report = fallback_report();
    report.challenged_chunk_id = Some("bad chunk id with spaces".to_string());
    assert!(
        report.validate().is_err(),
        "challenged chunk id must be bounded visible token"
    );

    let mut report = fallback_report();
    report.produced_at_ms = 0;
    assert!(report.validate().is_err(), "produced_at_ms must be nonzero");

    let mut report = fallback_report();
    report.snapshot_contribution_count = 0;
    assert!(
        report.validate().is_err(),
        "snapshot contribution count must be nonzero"
    );
}

#[test]
fn accounting_manifest_does_not_add_wallet_ledger_or_proto_authority_for_da_fallback_reports() {
    let manifest = read(crate_dir().join("Cargo.toml"));

    for forbidden_dependency in [
        "ron-proto",
        "ron_proto",
        "ron-ledger",
        "ron_ledger",
        "svc-wallet",
        "svc_wallet",
        "solana",
        "spl-token",
        "anchor-lang",
    ] {
        assert!(
            !manifest.contains(forbidden_dependency),
            "ron-accounting must not gain DA fallback wallet/ledger/runtime dependency: {forbidden_dependency}"
        );
    }
}

#[test]
fn accounting_source_does_not_implement_da_fallback_runtime_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find ron-accounting Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "commit_from_da",
            "apply_da",
            "settle_from_da",
            "unlock_from_da",
            "prune_from_da",
            "da_payout",
            "reward_payout_from_da",
            "wallet_mutation",
            "ledger_mutation",
            "payout_executed",
            "bridge_settlement",
            "external_settlement",
            "balance_truth: true",
            "wallet_side_effect: true",
            "ledger_side_effect: true",
            "payout_side_effect: true",
            "reward_truth: true",
            "terminality_truth: true",
            "external_claim_truth: true",
            "paid_unlock_authority: true",
            "pruning_authority: true",
            "outside_data_availability_truth: true",
            "outside_settlement: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "ron-accounting source must not implement DA fallback runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
