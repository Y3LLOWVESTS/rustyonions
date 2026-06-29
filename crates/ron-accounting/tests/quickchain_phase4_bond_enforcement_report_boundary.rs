#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 3 controlled bond enforcement report boundary tests for ron-accounting.
//! RO:WHY — Accounting may summarize controlled internal enforcement facts, but
//! must not become balance truth, wallet side effect, ledger side effect, payout
//! side effect, finality, public market, liquidity, bridge, or external settlement authority.
//! RO:INTERACTS — QuickChainBondEnforcementReport and source/Cargo boundaries.
//! RO:INVARIANTS — reports are derivative and read-only; integer strings only;
//! action semantics are explicit; unknown authority fields reject.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents Phase 4 Round 3 enforcement from becoming accounting authority.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_phase4_bond_enforcement_report_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    QuickChainBondEnforcementReport, QuickChainBondEnforcementReportAction,
    RON_ACCOUNTING_QUICKCHAIN_BOND_ENFORCEMENT_REPORT_SCHEMA,
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

fn reserve_report() -> QuickChainBondEnforcementReport {
    QuickChainBondEnforcementReport::new_enforcement_report(
        1_777_309_851_000,
        "roc-dev",
        "epoch:phase4:r3",
        "ron-accounting",
        "bond-enforcement:phase4:r3:reserve",
        "bond-account:phase4:r3:alice",
        QuickChainBondEnforcementReportAction::ReserveSlash,
        "50",
        "0",
        "0",
        "100",
        "50",
        "0",
        "50",
    )
    .expect("valid reserve report should build")
}

fn release_report() -> QuickChainBondEnforcementReport {
    QuickChainBondEnforcementReport::new_enforcement_report(
        1_777_309_851_001,
        "roc-dev",
        "epoch:phase4:r3",
        "ron-accounting",
        "bond-enforcement:phase4:r3:release",
        "bond-account:phase4:r3:alice",
        QuickChainBondEnforcementReportAction::ReleaseSlashReserve,
        "50",
        "0",
        "50",
        "100",
        "100",
        "0",
        "0",
    )
    .expect("valid release report should build")
}

fn capture_report() -> QuickChainBondEnforcementReport {
    QuickChainBondEnforcementReport::new_enforcement_report(
        1_777_309_851_002,
        "roc-dev",
        "epoch:phase4:r3",
        "ron-accounting",
        "bond-enforcement:phase4:r3:capture",
        "bond-account:phase4:r3:alice",
        QuickChainBondEnforcementReportAction::CaptureSlashReserve,
        "50",
        "50",
        "0",
        "50",
        "50",
        "0",
        "0",
    )
    .expect("valid capture report should build")
}

#[test]
fn enforcement_reports_are_read_only_and_conserve_resulting_components() {
    for report in [reserve_report(), release_report(), capture_report()] {
        assert_eq!(
            report.schema,
            RON_ACCOUNTING_QUICKCHAIN_BOND_ENFORCEMENT_REPORT_SCHEMA
        );
        assert_eq!(report.chain_id, "roc-dev");
        assert_eq!(report.epoch_id, "epoch:phase4:r3");
        assert_eq!(report.report_source, "ron-accounting");
        assert!(report.report_only);
        assert!(!report.balance_truth);
        assert!(!report.wallet_side_effect);
        assert!(!report.ledger_side_effect);
        assert!(!report.payout_side_effect);
        assert!(!report.terminality_truth);
        assert!(!report.external_claim_truth);
        assert!(!report.public_market);
        assert!(!report.liquidity_enabled);

        report
            .validate()
            .expect("valid enforcement report should validate");

        let encoded = serde_json::to_string(&report).expect("report should serialize");
        assert!(
            encoded.contains(r#""schema":"ron-accounting.quickchain-bond-enforcement-report.v1""#)
        );
        assert!(encoded.contains(r#""report_only":true"#));
        assert!(encoded.contains(r#""balance_truth":false"#));
    }
}

#[test]
fn enforcement_report_action_amount_semantics_are_strict() {
    let mut bad_reserve = reserve_report();
    bad_reserve.captured_minor = "1".to_owned();
    assert!(
        bad_reserve.validate().is_err(),
        "reserve report must not carry captured amount"
    );

    let mut bad_release = release_report();
    bad_release.released_minor = "49".to_owned();
    assert!(
        bad_release.validate().is_err(),
        "release report must release exactly amount_minor"
    );

    let mut bad_capture = capture_report();
    bad_capture.captured_minor = "49".to_owned();
    assert!(
        bad_capture.validate().is_err(),
        "capture report must capture exactly amount_minor"
    );

    let mut component_drift = capture_report();
    component_drift.resulting_locked_minor = "10".to_owned();
    component_drift.resulting_available_to_unlock_minor = "11".to_owned();
    assert!(
        component_drift.validate().is_err(),
        "resulting components must not exceed resulting locked amount"
    );
}

#[test]
fn enforcement_report_rejects_authority_flags_and_bad_money() {
    let mut report = reserve_report();
    report.report_only = false;
    assert!(report.validate().is_err());

    let mut balance_truth = reserve_report();
    balance_truth.balance_truth = true;
    assert!(balance_truth.validate().is_err());

    let mut wallet = reserve_report();
    wallet.wallet_side_effect = true;
    assert!(wallet.validate().is_err());

    let mut ledger = reserve_report();
    ledger.ledger_side_effect = true;
    assert!(ledger.validate().is_err());

    let mut payout = reserve_report();
    payout.payout_side_effect = true;
    assert!(payout.validate().is_err());

    let mut terminality = reserve_report();
    terminality.terminality_truth = true;
    assert!(terminality.validate().is_err());

    let mut external = reserve_report();
    external.external_claim_truth = true;
    assert!(external.validate().is_err());

    let mut public_market = reserve_report();
    public_market.public_market = true;
    assert!(public_market.validate().is_err());

    let mut liquidity = reserve_report();
    liquidity.liquidity_enabled = true;
    assert!(liquidity.validate().is_err());

    let mut zero = reserve_report();
    zero.amount_minor = "0".to_owned();
    assert!(zero.validate().is_err());

    let mut noncanonical = reserve_report();
    noncanonical.amount_minor = "050".to_owned();
    assert!(noncanonical.validate().is_err());

    let mut floatish = reserve_report();
    floatish.amount_minor = "50.0".to_owned();
    assert!(floatish.validate().is_err());
}

#[test]
fn enforcement_report_rejects_unknown_authority_fields() {
    for field in [
        "wallet_receipt",
        "ledger_receipt",
        "balance_minor",
        "wallet_mutation",
        "ledger_mutation",
        "payout_executed",
        "paid_unlock",
        "settlement_status",
        "finalized",
        "anchored",
        "public_staking_market",
        "liquidity_pool",
        "bridge_settlement",
        "external_settlement",
    ] {
        let mut poisoned = serde_json::to_value(reserve_report()).expect("report to json");
        poisoned[field] = json!(true);

        assert!(
            serde_json::from_value::<QuickChainBondEnforcementReport>(poisoned).is_err(),
            "enforcement report DTO must reject unknown authority field: {field}"
        );
    }
}

#[test]
fn accounting_manifest_does_not_link_wallet_ledger_or_proto_authority_for_enforcement_reports() {
    let manifest = read(crate_dir().join("Cargo.toml"));

    for forbidden in [
        "svc-wallet",
        "svc_wallet",
        "ron-ledger",
        "ron_ledger",
        "ron-proto",
        "ron_proto",
        "svc-passport",
        "svc-registry",
        "ron-auth",
        "solana",
        "spl-token",
        "anchor-lang",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "ron-accounting must not link authority crate/runtime for Phase 4 enforcement reports: {forbidden}"
        );
    }
}

#[test]
fn accounting_source_does_not_implement_phase4_enforcement_runtime_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find ron-accounting Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

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
            "public_market: true",
            "liquidity_enabled: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "ron-accounting source must not construct Phase 4 Round 3 enforcement authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
