#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 1 anchor report boundary tests for ron-accounting.
//! RO:WHY — Accounting snapshots may be referenced by anchor dry-run metadata, but must not become balance, payout, reward, finality, or settlement truth.
//! RO:INTERACTS — QuickChainAnchorReport and RewardSnapshotExport artifact helpers.
//! RO:INVARIANTS — report-only; evidence-only; no wallet/ledger mutation; no payout execution; no paid unlock; no outside-chain truth.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects authority flags and unknown fields.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_phase5_anchor_report_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    QuickChainAnchorReport, RewardContributionExport, RewardSnapshotExport,
    RON_ACCOUNTING_QUICKCHAIN_ANCHOR_REPORT_SCHEMA,
};
use serde_json::{json, Value};

const CHECKPOINT_HASH: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

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

fn sample_snapshot() -> RewardSnapshotExport {
    RewardSnapshotExport::new(
        1_777_500_000_000,
        "1000",
        vec![
            RewardContributionExport::new("acct_phase5_a", 100, 50, 10),
            RewardContributionExport::new("acct_phase5_b", 200, 0, 20),
        ],
    )
    .expect("sample snapshot should validate")
}

fn anchor_report() -> QuickChainAnchorReport {
    QuickChainAnchorReport::new_for_snapshot(
        1_777_500_001_000,
        "roc-dev",
        "epoch:phase5:r1",
        "ron-accounting",
        "anchor-dry-run:phase5:r1:accounting",
        CHECKPOINT_HASH,
        &sample_snapshot(),
    )
    .expect("anchor report should build")
}

fn assert_no_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                assert_ne!(
                    key, forbidden,
                    "anchor accounting report must not expose forbidden authority key `{forbidden}`"
                );
                assert_no_key(nested, forbidden);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_key(nested, forbidden);
            }
        }
        _ => {}
    }
}

#[test]
fn anchor_report_references_snapshot_without_becoming_truth() {
    let snapshot = sample_snapshot();
    let report = QuickChainAnchorReport::new_for_snapshot(
        1_777_500_001_000,
        "roc-dev",
        "epoch:phase5:r1",
        "ron-accounting",
        "anchor-dry-run:phase5:r1:accounting",
        CHECKPOINT_HASH,
        &snapshot,
    )
    .expect("anchor report should build");

    report.validate().expect("anchor report should validate");

    assert_eq!(
        report.schema,
        RON_ACCOUNTING_QUICKCHAIN_ANCHOR_REPORT_SCHEMA
    );
    assert_eq!(report.checkpoint_hash, CHECKPOINT_HASH);
    assert_eq!(
        report.accounting_snapshot_cid,
        snapshot
            .canonicalized()
            .expect("snapshot should canonicalize")
            .canonical_cid()
            .expect("snapshot cid should compute")
    );
    assert_eq!(
        report.snapshot_contribution_count,
        snapshot.contribution_count()
    );
    assert!(report.report_only);
    assert!(report.evidence_only);
    assert!(!report.balance_truth);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.payout_side_effect);
    assert!(!report.reward_truth);
    assert!(!report.terminality_truth);
    assert!(!report.external_claim_truth);
    assert!(!report.paid_unlock_authority);

    let value = serde_json::to_value(&report).expect("report should serialize");
    for forbidden in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "wallet_mutation",
        "ledger_mutation",
        "payout_executed",
        "reward_payout",
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
fn anchor_report_rejects_authority_flags_unknown_fields_and_bad_b3() {
    for field in [
        "balance_truth",
        "wallet_side_effect",
        "ledger_side_effect",
        "payout_side_effect",
        "reward_truth",
        "terminality_truth",
        "external_claim_truth",
        "paid_unlock_authority",
    ] {
        let mut report = anchor_report();

        match field {
            "balance_truth" => report.balance_truth = true,
            "wallet_side_effect" => report.wallet_side_effect = true,
            "ledger_side_effect" => report.ledger_side_effect = true,
            "payout_side_effect" => report.payout_side_effect = true,
            "reward_truth" => report.reward_truth = true,
            "terminality_truth" => report.terminality_truth = true,
            "external_claim_truth" => report.external_claim_truth = true,
            "paid_unlock_authority" => report.paid_unlock_authority = true,
            _ => unreachable!("covered above"),
        }

        assert!(
            report.validate().is_err(),
            "authority flag must reject when enabled: {field}"
        );
    }

    let clean = serde_json::to_value(anchor_report()).expect("report should serialize");
    for field in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "wallet_mutation",
        "ledger_mutation",
        "payout_executed",
        "reward_payout",
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
            .insert(field.to_string(), json!("client-supplied-anchor-authority"));

        assert!(
            serde_json::from_value::<QuickChainAnchorReport>(poisoned).is_err(),
            "anchor report DTO must reject unknown authority field: {field}"
        );
    }

    let mut report = anchor_report();
    report.checkpoint_hash = "not-a-b3-hash".to_string();
    assert!(
        report.validate().is_err(),
        "checkpoint_hash must be canonical b3"
    );

    let mut report = anchor_report();
    report.accounting_snapshot_cid = "b3:ABC".to_string();
    assert!(
        report.validate().is_err(),
        "accounting_snapshot_cid must be canonical b3"
    );
}

#[test]
fn anchor_report_rejects_empty_or_zero_context() {
    let mut report = anchor_report();
    report.produced_at_ms = 0;
    assert!(report.validate().is_err(), "produced_at_ms must be nonzero");

    let mut report = anchor_report();
    report.anchor_id.clear();
    assert!(report.validate().is_err(), "anchor_id must be explicit");

    let mut report = anchor_report();
    report.snapshot_contribution_count = 0;
    assert!(
        report.validate().is_err(),
        "snapshot contribution count must be nonzero"
    );
}

#[test]
fn accounting_source_does_not_implement_phase5_anchor_runtime_authority() {
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
            "ron-accounting must not gain Phase 5 anchor/wallet/ledger/runtime dependency: {forbidden_dependency}"
        );
    }

    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find ron-accounting Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "commit_from_anchor",
            "apply_anchor",
            "settle_from_anchor",
            "unlock_from_anchor",
            "anchor_payout",
            "reward_payout_from_anchor",
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
        ] {
            assert!(
                !code.contains(forbidden),
                "ron-accounting source must not implement Phase 5 anchor runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
