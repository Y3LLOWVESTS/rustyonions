#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

//! RO:WHAT — Internal ROC Beta Phase 5 economics config label non-authority tests for ron-accounting.
//! RO:WHY — Proves accounting may label config schema/version/hash only without becoming balance, receipt, payout, wallet, ledger, unlock, bridge, staking, liquidity, or external-settlement truth.
//! RO:INTERACTS — InternalRocEconomicsConfigLabel and `configs/roc-economics.toml`.
//! RO:INVARIANTS — report-only; label-only; no wallet/ledger side effects; no receipt/balance truth.
//! RO:METRICS — none.
//! RO:CONFIG — reads canonical `configs/roc-economics.toml` fixture.
//! RO:SECURITY — rejects authority flags, unknown fields, enabled bridge, and enabled staking.
//! RO:TEST — `cargo test -p ron-accounting --test internal_roc_beta_phase5_config_label_non_authority`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use ron_accounting::{
    InternalRocEconomicsConfigLabel, RON_ACCOUNTING_INTERNAL_ROC_ECONOMICS_CONFIG_LABEL_SCHEMA,
};
use serde_json::{json, Value};

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

fn canonical_config_bytes() -> Vec<u8> {
    fs::read(workspace_root().join("configs").join("roc-economics.toml"))
        .expect("canonical economics config should be readable")
}

fn canonical_label() -> InternalRocEconomicsConfigLabel {
    InternalRocEconomicsConfigLabel::from_toml_bytes(
        "configs/roc-economics.toml",
        &canonical_config_bytes(),
    )
    .expect("canonical config should produce a read-only label")
}

fn assert_no_authority_keys(value: &Value) {
    let object = value
        .as_object()
        .expect("label should serialize as a JSON object");

    for forbidden in [
        "operation_id",
        "idempotency_key",
        "account_sequence",
        "hold_id",
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "accounting_root",
        "balance_minor",
        "wallet_balance",
        "paid_unlock",
        "finality",
        "settlement",
        "bridge_txid",
        "staking_position",
        "liquidity_pool",
    ] {
        assert!(
            !object.contains_key(forbidden),
            "accounting config label must not expose authority key `{forbidden}`"
        );
    }
}

#[test]
fn accounting_labels_config_version_hash_and_source_only() {
    let label = canonical_label();

    assert_eq!(
        label.label_schema,
        RON_ACCOUNTING_INTERNAL_ROC_ECONOMICS_CONFIG_LABEL_SCHEMA
    );
    assert_eq!(label.config_schema, "internal_roc.economics-config.v1");
    assert_eq!(label.config_version, 1);
    assert_eq!(label.config_source, "configs/roc-economics.toml");
    assert!(label.config_b3.starts_with("b3:"));
    assert_eq!(label.config_b3.len(), 67);

    assert!(label.report_only);
    assert!(label.label_only);
    assert!(label.bridge_inert);
    assert!(label.staking_inert);
    assert!(!label.balance_truth);
    assert!(!label.receipt_truth);
    assert!(!label.wallet_side_effect);
    assert!(!label.ledger_side_effect);
    assert!(!label.payout_side_effect);
    assert!(!label.paid_unlock_authority);

    let value = serde_json::to_value(&label).expect("label should serialize");
    assert_no_authority_keys(&value);
}

#[test]
fn accounting_config_label_rejects_authority_flags_and_unknown_fields() {
    for flag in [
        "balance_truth",
        "receipt_truth",
        "wallet_side_effect",
        "ledger_side_effect",
        "payout_side_effect",
        "paid_unlock_authority",
    ] {
        let mut value = serde_json::to_value(canonical_label()).expect("label should serialize");
        value
            .as_object_mut()
            .expect("label JSON should be an object")
            .insert(flag.to_owned(), json!(true));

        let label = serde_json::from_value::<InternalRocEconomicsConfigLabel>(value)
            .expect("known authority flag should parse before validation");
        assert!(
            label.validate().is_err(),
            "authority flag `{flag}` must fail validation"
        );
    }

    let mut value = serde_json::to_value(canonical_label()).expect("label should serialize");
    value
        .as_object_mut()
        .expect("label JSON should be an object")
        .insert("wallet_mutation".to_owned(), json!("forbidden"));

    assert!(
        serde_json::from_value::<InternalRocEconomicsConfigLabel>(value).is_err(),
        "unknown wallet-mutation shaped field must be rejected"
    );
}

#[test]
fn accounting_config_label_rejects_enabled_future_bridge_or_staking() {
    let raw = String::from_utf8(canonical_config_bytes()).expect("config should be UTF-8");

    let bridge_enabled = raw.replacen(
        "[future_bridge]\nenabled = false",
        "[future_bridge]\nenabled = true",
        1,
    );
    assert!(
        InternalRocEconomicsConfigLabel::from_toml_bytes(
            "configs/roc-economics.toml",
            bridge_enabled.as_bytes(),
        )
        .is_err(),
        "accounting must reject enabled bridge config labels"
    );

    let staking_enabled = raw.replacen(
        "[future_staking]\nenabled = false",
        "[future_staking]\nenabled = true",
        1,
    );
    assert!(
        InternalRocEconomicsConfigLabel::from_toml_bytes(
            "configs/roc-economics.toml",
            staking_enabled.as_bytes(),
        )
        .is_err(),
        "accounting must reject enabled staking config labels"
    );
}

#[test]
fn accounting_source_keeps_economics_config_label_non_authoritative() {
    let code = read(
        crate_dir()
            .join("src")
            .join("accounting")
            .join("economics_config.rs"),
    )
    .to_ascii_lowercase();

    for forbidden in [
        "issue_from_economics_config",
        "transfer_from_economics_config",
        "burn_from_economics_config",
        "capture_from_economics_config",
        "release_from_economics_config",
        "mutate_from_economics_config",
        "unlock_from_economics_config",
        "create_receipt_from_economics_config",
        "balance_truth: true",
        "receipt_truth: true",
        "wallet_side_effect: true",
        "ledger_side_effect: true",
        "payout_side_effect: true",
        "paid_unlock_authority: true",
    ] {
        assert!(
            !code.contains(forbidden),
            "economics config label source must not construct authority via `{forbidden}`"
        );
    }
}
