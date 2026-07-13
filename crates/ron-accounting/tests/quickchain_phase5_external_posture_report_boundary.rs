#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 5 Round 3 external-posture report boundary tests for ron-accounting.
//! RO:WHY — Accounting snapshots may be referenced by the selected anchor-only posture, but must not become balance, payout, reward, finality, unlock, market, or outside-truth authority.
//! RO:INTERACTS — QuickChainExternalPostureReport and RewardSnapshotExport artifact helpers.
//! RO:INVARIANTS — report-only; evidence-only; anchor-only; no wallet/ledger mutation; no payout execution; no paid unlock; no public market.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — rejects authority flags and unknown fields.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_phase5_external_posture_report_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    QuickChainExternalPostureReport, QuickChainExternalPostureReportStatus,
    RewardContributionExport, RewardSnapshotExport,
    RON_ACCOUNTING_QUICKCHAIN_EXTERNAL_POSTURE_REPORT_SCHEMA,
};
use serde_json::{json, Value};

const CHECKPOINT_HASH: &str = "b3:bcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbcbc";

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

fn posture_report() -> QuickChainExternalPostureReport {
    QuickChainExternalPostureReport::new_for_snapshot(
        1_777_500_000_123,
        "roc-dev",
        "epoch:phase5:round3",
        "ron-accounting",
        "posture:phase5-round3:anchor-only",
        CHECKPOINT_HASH,
        &sample_snapshot(),
    )
    .expect("posture report should validate")
}

#[test]
fn accounting_external_posture_report_is_anchor_only_read_only_and_non_authority() {
    let report = posture_report();

    assert_eq!(
        report.schema,
        RON_ACCOUNTING_QUICKCHAIN_EXTERNAL_POSTURE_REPORT_SCHEMA
    );
    assert_eq!(report.chain_id, "roc-dev");
    assert_eq!(report.epoch_id, "epoch:phase5:round3");
    assert_eq!(report.report_source, "ron-accounting");
    assert_eq!(report.posture_id, "posture:phase5-round3:anchor-only");
    assert_eq!(report.checkpoint_hash, CHECKPOINT_HASH);
    assert_eq!(
        report.status,
        QuickChainExternalPostureReportStatus::AnchorOnlyEvidenceOnly
    );

    assert!(report.anchor_only_selected);
    assert!(report.report_only);
    assert!(report.evidence_only);
    assert!(report.wallet_ledger_truth_canonical);
    assert!(report.accounting_snapshot_cid.starts_with("b3:"));
    assert_eq!(report.snapshot_contribution_count, 2);

    assert!(!report.balance_truth);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.payout_side_effect);
    assert!(!report.reward_truth);
    assert!(!report.terminality_truth);
    assert!(!report.external_claim_truth);
    assert!(!report.paid_unlock_authority);
    assert!(!report.outside_settlement);
    assert!(!report.bridge_authority);
    assert!(!report.outside_program_authority);
    assert!(!report.listing_authority);
    assert!(!report.public_market);
    assert!(!report.liquidity_enabled);
    assert!(!report.bonded_economy_authority);

    let encoded = serde_json::to_string(&report).expect("report should serialize");
    assert!(encoded.contains(r#""schema":"ron-accounting.quickchain-external-posture-report.v1""#));
    assert!(encoded.contains(r#""status":"anchor_only_evidence_only""#));
    assert!(encoded.contains(r#""anchor_only_selected":true"#));
}

#[test]
fn accounting_external_posture_report_rejects_bad_shape_and_missing_truth_flags() {
    let mut report = posture_report();
    report.produced_at_ms = 0;
    assert!(report.validate().is_err(), "produced_at_ms must be nonzero");

    let mut report = posture_report();
    report.snapshot_contribution_count = 0;
    assert!(
        report.validate().is_err(),
        "snapshot contribution count must be nonzero"
    );

    let mut report = posture_report();
    report.anchor_only_selected = false;
    assert!(
        report.validate().is_err(),
        "anchor-only posture selection must be explicit"
    );

    let mut report = posture_report();
    report.report_only = false;
    assert!(report.validate().is_err(), "report-only must be explicit");

    let mut report = posture_report();
    report.evidence_only = false;
    assert!(report.validate().is_err(), "evidence-only must be explicit");

    let mut report = posture_report();
    report.wallet_ledger_truth_canonical = false;
    assert!(
        report.validate().is_err(),
        "wallet/ledger truth must remain canonical"
    );

    let mut report = posture_report();
    report.checkpoint_hash = "b3:ABC".to_string();
    assert!(
        report.validate().is_err(),
        "checkpoint hash must be canonical b3"
    );
}

#[test]
fn accounting_external_posture_report_rejects_authority_flags_and_unknown_fields() {
    let clean = serde_json::to_value(posture_report()).expect("report should serialize");

    for field in [
        "anchor_only_selected",
        "report_only",
        "evidence_only",
        "wallet_ledger_truth_canonical",
    ] {
        let mut poisoned = clean.clone();
        poisoned[field] = Value::Bool(false);

        let decoded = serde_json::from_value::<QuickChainExternalPostureReport>(poisoned)
            .expect("known field should deserialize before validation");

        assert!(
            decoded.validate().is_err(),
            "posture report must reject false required flag {field}"
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
        "outside_settlement",
        "bridge_authority",
        "outside_program_authority",
        "listing_authority",
        "public_market",
        "liquidity_enabled",
        "bonded_economy_authority",
    ] {
        let mut poisoned = clean.clone();
        poisoned[field] = Value::Bool(true);

        let decoded = serde_json::from_value::<QuickChainExternalPostureReport>(poisoned)
            .expect("known authority field should deserialize before validation");

        assert!(
            decoded.validate().is_err(),
            "posture report must reject authority field {field}"
        );
    }

    let mut unknown = clean;
    unknown["external_settlement_authorized"] = json!(true);
    assert!(
        serde_json::from_value::<QuickChainExternalPostureReport>(unknown).is_err(),
        "unknown posture authority fields must reject"
    );
}

#[test]
fn accounting_manifest_does_not_add_wallet_ledger_or_runtime_authority_for_external_posture_reports(
) {
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
            "ron-accounting must not gain external posture wallet/ledger/runtime dependency: {forbidden_dependency}"
        );
    }
}

#[test]
fn accounting_source_does_not_implement_external_posture_runtime_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find ron-accounting Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "commit_from_posture",
            "apply_posture",
            "settle_from_posture",
            "unlock_from_posture",
            "posture_payout",
            "reward_payout_from_posture",
            // Accounting DTOs may carry explicit false-only
            // wallet_mutation/ledger_mutation boundary flags.
            // Reject actual runtime mutation or authorization
            // behavior instead of rejecting defensive field names.
            "mutate_wallet",
            "execute_wallet_mutation",
            "apply_wallet_mutation",
            "authorize_wallet_mutation",
            "mutate_ledger",
            "execute_ledger_mutation",
            "apply_ledger_mutation",
            "authorize_ledger_mutation",
            "payout_executed",
            "bridge_settlement",
            "external_settlement",
            "solana",
            "rox",
            "exchange_facing",
            "balance_truth: true",
            "wallet_side_effect: true",
            "ledger_side_effect: true",
            "payout_side_effect: true",
            "reward_truth: true",
            "terminality_truth: true",
            "external_claim_truth: true",
            "paid_unlock_authority: true",
            "outside_settlement: true",
            "bridge_authority: true",
            "outside_program_authority: true",
            "listing_authority: true",
            "public_market: true",
            "liquidity_enabled: true",
            "bonded_economy_authority: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "ron-accounting source must not implement external posture runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
