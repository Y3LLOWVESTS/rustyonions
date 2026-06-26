#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 2 disputed-bond report boundary tests for ron-accounting.
//! RO:WHY — Accounting may summarize challenge/freeze/appeal simulation state,
//! but must not become balance truth, wallet side effect, ledger side effect,
//! payout side effect, finality, bridge, liquidity, or public staking authority.
//! RO:INTERACTS — QuickChainBondDisputeReport and source/Cargo boundaries.
//! RO:INVARIANTS — reports are derivative and read-only; integer strings only;
//! windows are explicit; unknown authority fields reject.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents Phase 4 Round 2 simulation from becoming accounting authority.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_phase4_bond_dispute_report_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    QuickChainBondDisputeReport, QuickChainBondDisputeReportStatus,
    RON_ACCOUNTING_QUICKCHAIN_BOND_DISPUTE_REPORT_SCHEMA,
};
use serde_json::json;

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

fn valid_report() -> QuickChainBondDisputeReport {
    QuickChainBondDisputeReport::new_dispute_report(
        1_777_310_851_000,
        "roc-dev",
        "epoch:phase4:r2",
        "ron-accounting",
        "bond-dispute:phase4:r2:alice",
        "bond-account:phase4:r2:alice",
        QuickChainBondDisputeReportStatus::FrozenPendingAppeal,
        "500",
        "125",
        false,
        true,
    )
    .expect("valid read-only disputed-bond report should build")
}

#[test]
fn dispute_report_is_read_only_window_bound_and_not_truth() {
    let report = valid_report();

    assert_eq!(
        report.schema,
        RON_ACCOUNTING_QUICKCHAIN_BOND_DISPUTE_REPORT_SCHEMA
    );
    assert_eq!(report.chain_id, "roc-dev");
    assert_eq!(report.epoch_id, "epoch:phase4:r2");
    assert_eq!(report.dispute_id, "bond-dispute:phase4:r2:alice");
    assert_eq!(report.bond_account_id, "bond-account:phase4:r2:alice");
    assert_eq!(
        report.status,
        QuickChainBondDisputeReportStatus::FrozenPendingAppeal
    );
    assert_eq!(report.disputed_minor, "500");
    assert_eq!(report.frozen_minor, "125");
    assert!(!report.challenge_window_open);
    assert!(report.appeal_window_open);
    assert!(report.report_only);
    assert!(!report.balance_truth);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.payout_side_effect);
    assert!(!report.terminality_truth);
    assert!(!report.external_claim_truth);

    report
        .validate()
        .expect("valid read-only disputed-bond report should validate");
}

#[test]
fn dispute_report_rejects_authority_flags_and_inconsistent_state() {
    let mut balance_truth = valid_report();
    balance_truth.balance_truth = true;
    assert!(
        balance_truth.validate().is_err(),
        "accounting dispute report must not claim balance truth"
    );

    let mut wallet_side_effect = valid_report();
    wallet_side_effect.wallet_side_effect = true;
    assert!(
        wallet_side_effect.validate().is_err(),
        "accounting dispute report must not claim wallet side effects"
    );

    let mut ledger_side_effect = valid_report();
    ledger_side_effect.ledger_side_effect = true;
    assert!(
        ledger_side_effect.validate().is_err(),
        "accounting dispute report must not claim ledger side effects"
    );

    let mut payout_side_effect = valid_report();
    payout_side_effect.payout_side_effect = true;
    assert!(
        payout_side_effect.validate().is_err(),
        "accounting dispute report must not claim payout side effects"
    );

    let mut terminality_truth = valid_report();
    terminality_truth.terminality_truth = true;
    assert!(
        terminality_truth.validate().is_err(),
        "accounting dispute report must not claim terminality truth"
    );

    let mut external_claim_truth = valid_report();
    external_claim_truth.external_claim_truth = true;
    assert!(
        external_claim_truth.validate().is_err(),
        "accounting dispute report must not claim external-claim truth"
    );

    let mut over_frozen = valid_report();
    over_frozen.frozen_minor = "501".to_owned();
    assert!(
        over_frozen.validate().is_err(),
        "frozen_minor must not exceed disputed_minor"
    );

    let terminal_with_window = QuickChainBondDisputeReport::new_dispute_report(
        1_777_310_851_000,
        "roc-dev",
        "epoch:phase4:r2",
        "ron-accounting",
        "bond-dispute:phase4:r2:terminal",
        "bond-account:phase4:r2:alice",
        QuickChainBondDisputeReportStatus::ResolvedPenaltyRejected,
        "500",
        "0",
        true,
        false,
    )
    .expect_err("terminal dispute report must not carry open windows");
    assert!(
        terminal_with_window
            .to_string()
            .contains("terminal dispute report"),
        "expected terminal-window validation error, got {terminal_with_window}"
    );
}

#[test]
fn dispute_report_rejects_unknown_authority_fields() {
    let clean = serde_json::to_value(valid_report()).expect("report should serialize");

    for (field, value) in [
        ("wallet_receipt", json!("tx_fake_dispute")),
        ("ledger_receipt", json!("tx_fake_ledger")),
        ("balance_minor", json!("500")),
        ("bond_truth", json!(true)),
        ("slash_truth", json!(true)),
        ("wallet_mutation", json!(true)),
        ("ledger_mutation", json!(true)),
        ("payout_executed", json!(true)),
        ("execute_slash", json!(true)),
        ("commit_slash_decision", json!(true)),
        ("capture_bond", json!(true)),
        ("settlement_status", json!("finalized")),
        ("finalized", json!(true)),
        ("finality_truth", json!(true)),
        ("settlement_truth", json!(true)),
        ("anchored", json!(true)),
        ("bridge_settlement", json!("solana")),
        ("external_settlement", json!("rox")),
    ] {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("report JSON should be object")
            .insert(field.to_owned(), value);

        assert!(
            serde_json::from_value::<QuickChainBondDisputeReport>(poisoned).is_err(),
            "disputed-bond report DTO must reject unknown authority field: {field}"
        );
    }
}

#[test]
fn accounting_source_does_not_construct_phase4_round2_runtime_authority() {
    let manifest = read(crate_dir().join("Cargo.toml"));

    for forbidden_dependency in [
        "ron-proto",
        "ron_proto",
        "ron-ledger",
        "ron_ledger",
        "svc-wallet",
        "svc_wallet",
    ] {
        assert!(
            !manifest.contains(forbidden_dependency),
            "ron-accounting must not gain validator/wallet/ledger authority dependency: {forbidden_dependency}"
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
            "execute_slash",
            "commit_slash_decision",
            "apply_slash",
            "capture_bond",
            "freeze_wallet_balance",
            "wallet_mutation",
            "ledger_mutation",
            "payout_executed",
            "validator_reward",
            "stake_validator",
            "public_staking_market",
            "liquidity_pool",
            "bridge_settlement",
            "external_settlement",
            "balance_truth: true",
            "wallet_side_effect: true",
            "ledger_side_effect: true",
            "payout_side_effect: true",
            "terminality_truth: true",
            "external_claim_truth: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "ron-accounting source must not construct Phase 4 Round 2 runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
