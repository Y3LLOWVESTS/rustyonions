#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Phase 4 Round 1 bond report boundary tests for ron-accounting.
//! RO:WHY — Accounting may summarize bond-model facts as read-only reports, but
//! must not become balance truth, bond truth, wallet side effect, ledger side effect,
//! payout side effect, finality, public market, liquidity, bridge, or external
//! settlement authority.
//! RO:INTERACTS — QuickChainBondReport and source/Cargo boundaries.
//! RO:INVARIANTS — reports are derivative and read-only; integer strings only;
//! unknown authority fields reject.
//! RO:METRICS — none.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — prevents Phase 4 economics authority creep into accounting.
//! RO:TEST — cargo test -p ron-accounting --test quickchain_phase4_bond_report_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{QuickChainBondReport, RON_ACCOUNTING_QUICKCHAIN_BOND_REPORT_SCHEMA};
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

fn valid_report() -> QuickChainBondReport {
    QuickChainBondReport::new(
        1_777_309_851_000,
        "roc-dev",
        "epoch:phase4:r1",
        "ron-accounting",
        2,
        "1000",
        "250",
        "0",
    )
    .expect("valid read-only bond report should build")
}

#[test]
fn bond_report_is_read_only_and_canonical_integer_shape() {
    let report = valid_report();

    assert_eq!(report.schema, RON_ACCOUNTING_QUICKCHAIN_BOND_REPORT_SCHEMA);
    assert_eq!(report.chain_id, "roc-dev");
    assert_eq!(report.epoch_id, "epoch:phase4:r1");
    assert_eq!(report.account_count, 2);
    assert_eq!(report.locked_minor, "1000");
    assert_eq!(report.pending_unlock_minor, "250");
    assert_eq!(report.evidence_reserved_minor, "0");
    assert!(report.report_only);
    assert!(!report.balance_truth);
    assert!(!report.wallet_side_effect);
    assert!(!report.ledger_side_effect);
    assert!(!report.payout_side_effect);

    report
        .validate()
        .expect("valid read-only bond report should validate");

    let encoded = serde_json::to_string(&report).expect("report should serialize");
    assert!(encoded.contains(r#""schema":"ron-accounting.quickchain-bond-report.v1""#));
    assert!(encoded.contains(r#""locked_minor":"1000""#));
    assert!(encoded.contains(r#""report_only":true"#));
    assert!(encoded.contains(r#""wallet_side_effect":false"#));
}

#[test]
fn bond_report_rejects_authority_flags_and_component_drift() {
    let mut not_report_only = valid_report();
    not_report_only.report_only = false;
    assert!(
        not_report_only.validate().is_err(),
        "report must stay report-only"
    );

    let mut balance_truth = valid_report();
    balance_truth.balance_truth = true;
    assert!(
        balance_truth.validate().is_err(),
        "accounting report must not claim balance truth"
    );

    let mut wallet_side_effect = valid_report();
    wallet_side_effect.wallet_side_effect = true;
    assert!(
        wallet_side_effect.validate().is_err(),
        "accounting report must not claim wallet side effect"
    );

    let mut ledger_side_effect = valid_report();
    ledger_side_effect.ledger_side_effect = true;
    assert!(
        ledger_side_effect.validate().is_err(),
        "accounting report must not claim ledger side effect"
    );

    let mut payout_side_effect = valid_report();
    payout_side_effect.payout_side_effect = true;
    assert!(
        payout_side_effect.validate().is_err(),
        "accounting report must not claim payout side effect"
    );

    let mut component_drift = valid_report();
    component_drift.locked_minor = "100".to_owned();
    component_drift.pending_unlock_minor = "101".to_owned();
    assert!(
        component_drift.validate().is_err(),
        "pending/evidence components must not exceed locked amount"
    );
}

#[test]
fn bond_report_rejects_float_money_leading_zeroes_and_unknown_authority_fields() {
    let mut float_money = serde_json::to_value(valid_report()).expect("report JSON");
    float_money
        .as_object_mut()
        .expect("report should be object")
        .insert("locked_minor".to_owned(), json!("10.5"));
    assert!(
        serde_json::from_value::<QuickChainBondReport>(float_money)
            .expect("string money field still deserializes")
            .validate()
            .is_err(),
        "report must reject float-like money strings"
    );

    let mut leading_zero = valid_report();
    leading_zero.locked_minor = "010".to_owned();
    assert!(
        leading_zero.validate().is_err(),
        "report must reject noncanonical leading-zero money strings"
    );

    let clean = serde_json::to_value(valid_report()).expect("report JSON");

    for (field, value) in [
        ("wallet_receipt", json!("tx_fake")),
        ("ledger_commit", json!(true)),
        ("balance_minor", json!("999")),
        ("bond_truth", json!(true)),
        ("penalty_evidence", json!("evidence:auto")),
        ("public_staking_market", json!(true)),
        ("liquidity_pool", json!("pool:fake")),
        ("bridge_settlement", json!("solana")),
        ("external_settlement", json!("rox")),
        (
            "state_root",
            json!("b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ),
    ] {
        let mut poisoned = clean.clone();
        poisoned
            .as_object_mut()
            .expect("report JSON should be object")
            .insert(field.to_owned(), value);

        assert!(
            serde_json::from_value::<QuickChainBondReport>(poisoned).is_err(),
            "bond report DTO must reject unknown authority field: {field}"
        );
    }
}

#[test]
fn accounting_manifest_does_not_add_wallet_ledger_or_proto_authority_for_bond_reports() {
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
            "ron-accounting must not link authority crate/runtime for Phase 4 bond reports: {forbidden}"
        );
    }
}

#[test]
fn accounting_source_does_not_implement_phase4_bond_runtime_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find ron-accounting Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "QuickChainValidatorBondAccount",
            "QuickChainSlashEvidence",
            "execute_bond(",
            "apply_bond(",
            "commit_bond(",
            "bond_truth: true",
            "create_receipt(",
            "wallet_issue",
            "ledger_commit",
            "mutate_wallet",
            "mutate_ledger",
            "public_market: true",
            "liquidity_enabled: true",
            "bridge_settlement",
            "external_settlement",
        ] {
            assert!(
                !code.contains(forbidden),
                "ron-accounting source must not implement Phase 4 bond runtime authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}
