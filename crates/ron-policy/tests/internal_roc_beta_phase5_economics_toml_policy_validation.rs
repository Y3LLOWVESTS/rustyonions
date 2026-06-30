//! RO:WHAT — Internal ROC Beta Phase 5 economics TOML validation tests for ron-policy.
//!
//! RO:WHY — Proves policy validates config shape/gates only and does not become receipt, balance, or payout truth.
//! RO:INTERACTS — `ron_policy::economics::internal_roc` and `configs/roc-economics.toml`.
//! RO:INVARIANTS — no floats; exact bps totals; explicit remainder sink; bridge/staking inert.
//! RO:METRICS — none.
//! RO:CONFIG — reads canonical `configs/roc-economics.toml` fixture.
//! RO:SECURITY — no wallet/ledger mutation, no paid entitlement truth, no bridge/staking runtime.
//! RO:TEST — `cargo test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation`.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use ron_policy::economics::{
    load_internal_roc_economics_toml, load_internal_roc_economics_toml_str,
    validate_internal_roc_economics_config, InternalRocRemainderSink,
    INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA,
};

const ROC_ECONOMICS_TOML: &str = include_str!("../../../configs/roc-economics.toml");

#[test]
fn canonical_internal_roc_economics_toml_validates_as_policy_gate_only() {
    let config = load_internal_roc_economics_toml(ROC_ECONOMICS_TOML.as_bytes())
        .expect("canonical internal ROC economics TOML validates");
    let validation = validate_internal_roc_economics_config(&config).expect("validation marker");

    assert_eq!(config.schema, INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA);
    assert_eq!(config.version, 1);
    assert_eq!(config.units.basis_point_denominator, 10_000);
    assert_eq!(
        config.rounding.remainder_sink,
        InternalRocRemainderSink::Treasury
    );
    assert!(validation.policy_validated_only);
    assert!(validation.bridge_inert);
    assert!(validation.staking_inert);
    assert!(!config.future_bridge.enabled);
    assert!(!config.future_staking.enabled);
}

#[test]
fn canonical_internal_roc_economics_toml_rejects_unknown_fields() {
    let poisoned = format!("{ROC_ECONOMICS_TOML}\nreceipt_truth = true\n");

    let err = load_internal_roc_economics_toml_str(&poisoned)
        .expect_err("unknown receipt truth field must reject");

    assert!(
        err.to_string().contains("unknown field") || err.to_string().contains("parse error"),
        "unexpected error: {err}"
    );
}

#[test]
fn canonical_internal_roc_economics_toml_rejects_float_or_numeric_money() {
    let poisoned =
        ROC_ECONOMICS_TOML.replace("minimum_price_minor = \"1\"", "minimum_price_minor = 1.5");

    let err = load_internal_roc_economics_toml_str(&poisoned).expect_err("float money must reject");

    assert!(
        err.to_string().contains("invalid type") || err.to_string().contains("parse error"),
        "unexpected error: {err}"
    );
}

#[test]
fn canonical_internal_roc_economics_toml_rejects_invalid_bps_totals() {
    let poisoned = ROC_ECONOMICS_TOML.replace(
        "label = \"creator\"\naccount_role = \"creator\"\nbps = 8500",
        "label = \"creator\"\naccount_role = \"creator\"\nbps = 8400",
    );

    let err = load_internal_roc_economics_toml_str(&poisoned)
        .expect_err("invalid paid-content bps total must reject");

    assert!(
        err.to_string().contains("paid_content.default_splits")
            && err.to_string().contains("10000"),
        "unexpected error: {err}"
    );
}

#[test]
fn canonical_internal_roc_economics_toml_requires_remainder_sink() {
    let poisoned = ROC_ECONOMICS_TOML.replace("remainder_sink = \"treasury\"\n", "");

    let err = load_internal_roc_economics_toml_str(&poisoned)
        .expect_err("missing remainder sink must reject");

    assert!(
        err.to_string().contains("missing field") || err.to_string().contains("remainder_sink"),
        "unexpected error: {err}"
    );
}

#[test]
fn canonical_internal_roc_economics_toml_keeps_bridge_and_staking_inert() {
    let bridge_enabled = ROC_ECONOMICS_TOML.replacen("enabled = false", "enabled = true", 1);
    let err = load_internal_roc_economics_toml_str(&bridge_enabled)
        .expect_err("enabled bridge placeholder must reject");
    assert!(
        err.to_string().contains("future_bridge.enabled"),
        "unexpected error: {err}"
    );

    let staking_enabled = ROC_ECONOMICS_TOML.replacen("enabled = false", "enabled = true", 2);
    let err = load_internal_roc_economics_toml_str(&staking_enabled)
        .expect_err("enabled staking placeholder must reject");
    assert!(
        err.to_string().contains("enabled must remain false/inert"),
        "unexpected error: {err}"
    );
}

#[test]
fn policy_validation_output_does_not_claim_authority_truth() {
    let config = load_internal_roc_economics_toml(ROC_ECONOMICS_TOML.as_bytes())
        .expect("canonical internal ROC economics TOML validates");
    let validation = validate_internal_roc_economics_config(&config).expect("validation marker");
    let encoded = serde_json::to_string(&validation).expect("validation marker serializes");

    for forbidden in [
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "balance",
        "balance_minor",
        "wallet_balance",
        "ledger_balance",
        "finality",
        "finalized",
        "unlock_granted",
        "payout_executed",
        "wallet_mutation",
        "ledger_mutation",
        "bridge_runtime",
        "staking_runtime",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "policy validation output must not claim authority field {forbidden}"
        );
    }
}
