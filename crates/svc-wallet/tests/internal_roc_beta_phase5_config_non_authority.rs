#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Internal ROC Beta Phase 5 economics config non-authority tests for svc-wallet.
//! RO:WHY — Proves canonical tokenomics config cannot directly become wallet mutation, receipt, balance, payout, finality, bridge, staking, liquidity, or external-settlement authority.
//! RO:INTERACTS — `configs/roc-economics.toml`, wallet DTOs, wallet accounting observer seam.
//! RO:INVARIANTS — svc-wallet remains mutation front-door; config is not a wallet request; accounting observes after receipt truth only.
//! RO:METRICS — none.
//! RO:CONFIG — reads canonical `configs/roc-economics.toml` fixture.
//! RO:SECURITY — no fake receipt, fake balance, silent spend, bridge, staking, or external settlement.
//! RO:TEST — `cargo test -p svc-wallet --test internal_roc_beta_phase5_config_non_authority`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use svc_wallet::{
    accounting::client::{AccountingEvent, NoopAccountingClient},
    dto::requests::{AmountMinor, IssueRequest, TransferRequest},
};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_dir()
        .parent()
        .and_then(Path::parent)
        .expect("crate should be under workspace/crates")
        .to_path_buf()
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn canonical_config_text() -> String {
    read(workspace_root().join("configs").join("roc-economics.toml"))
}

fn assert_no_wallet_authority_keys(value: &Value) {
    match value {
        Value::Object(object) => {
            for forbidden in [
                "economics_config",
                "economics_config_hash",
                "tokenomics_config",
                "config_receipt",
                "config_balance",
                "config_finality",
                "config_unlock",
                "balance_truth",
                "receipt_truth",
                "paid_unlock_authority",
                "wallet_side_effect",
                "ledger_side_effect",
                "payout_side_effect",
                "bridge_txid",
                "staking_position_id",
                "liquidity_pool_id",
                "outside_settlement_claim",
            ] {
                assert!(
                    !object.contains_key(forbidden),
                    "wallet DTO must not expose config-derived authority key `{forbidden}`"
                );
            }

            for nested in object.values() {
                assert_no_wallet_authority_keys(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_no_wallet_authority_keys(nested);
            }
        }
        _ => {}
    }
}

#[test]
fn canonical_economics_config_is_not_a_wallet_request_or_receipt() {
    let config = canonical_config_text();

    assert!(config.contains("schema = \"internal_roc.economics-config.v1\""));
    assert!(config.contains("[future_bridge]"));
    assert!(config.contains("[future_staking]"));
    assert!(config.contains("enabled = false"));

    assert!(
        serde_json::from_str::<IssueRequest>(&config).is_err(),
        "TOML config bytes must not parse as an issue request"
    );
    assert!(
        serde_json::from_str::<TransferRequest>(&config).is_err(),
        "TOML config bytes must not parse as a transfer request"
    );

    for forbidden in [
        "wallet_side_effect = true",
        "ledger_side_effect = true",
        "receipt_truth = true",
        "balance_truth = true",
        "paid_unlock_authority = true",
    ] {
        assert!(
            !config.contains(forbidden),
            "canonical economics config must not carry wallet authority flag `{forbidden}`"
        );
    }
}

#[test]
fn wallet_request_dtos_reject_config_authority_poison_fields() {
    let clean_issue = json!({
        "to": "acct_phase5_config_label",
        "asset": "roc",
        "amount_minor": "1",
        "idempotency_key": "phase5-config-label-issue",
        "memo": "ordinary wallet issue path; config is not authority"
    });

    let issue = serde_json::from_value::<IssueRequest>(clean_issue.clone())
        .expect("clean issue request should parse");
    assert_eq!(issue.to, "acct_phase5_config_label");
    assert_eq!(issue.amount_minor.get(), 1);

    for poisoned_key in [
        "economics_config_hash",
        "tokenomics_config_receipt",
        "balance_truth",
        "receipt_truth",
        "paid_unlock_authority",
        "bridge_settlement",
        "staking_position_id",
        "liquidity_pool_id",
    ] {
        let mut poisoned = clean_issue.clone();
        poisoned
            .as_object_mut()
            .expect("issue JSON should be an object")
            .insert(poisoned_key.to_owned(), json!("forbidden"));

        assert!(
            serde_json::from_value::<IssueRequest>(poisoned).is_err(),
            "IssueRequest must reject config-authority poison field `{poisoned_key}`"
        );
    }
}

#[test]
fn wallet_accounting_observation_remains_derivative_not_config_authority() {
    let event = AccountingEvent {
        op: "issue",
        asset: "roc".to_owned(),
        amount_minor: 42,
    };

    assert_eq!(event.op, "issue");
    assert_eq!(event.asset, "roc");
    assert_eq!(event.amount_minor, 42);

    NoopAccountingClient.record(event.clone());

    let event_debug = format!("{event:?}").to_ascii_lowercase();
    for forbidden in [
        "economics_config_hash",
        "balance_truth",
        "receipt_truth",
        "paid_unlock_authority",
        "wallet_side_effect",
        "ledger_side_effect",
        "payout_side_effect",
        "bridge",
        "staking",
        "liquidity",
        "outside_settlement",
    ] {
        assert!(
            !event_debug.contains(forbidden),
            "accounting observation must not carry config-derived wallet authority `{forbidden}`"
        );
    }
}

#[test]
fn wallet_receipt_shape_stays_backend_derived_not_config_derived() {
    let request = IssueRequest {
        to: "acct_phase5_receipt_shape".to_owned(),
        asset: "roc".to_owned(),
        amount_minor: AmountMinor(7),
        idempotency_key: Some("phase5-receipt-shape".to_owned()),
        memo: Some("config_version=1 label only".to_owned()),
    };

    let value = serde_json::to_value(&request).expect("request should serialize");
    assert_no_wallet_authority_keys(&value);
}

#[test]
fn wallet_source_does_not_construct_config_based_mutation_paths() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-wallet Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path)).to_ascii_lowercase();

        for forbidden in [
            "issue_from_economics_config",
            "transfer_from_economics_config",
            "burn_from_economics_config",
            "hold_from_economics_config",
            "capture_from_economics_config",
            "release_from_economics_config",
            "mutate_from_economics_config",
            "receipt_from_economics_config",
            "balance_from_economics_config",
            "unlock_from_economics_config",
            "wallet_side_effect: true",
            "ledger_side_effect: true",
            "receipt_truth: true",
            "balance_truth: true",
            "paid_unlock_authority: true",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-wallet source must not construct config-derived authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
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
