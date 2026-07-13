//! RO:WHAT — Explicit Internal ROC economics-profile selection tests.
//! RO:WHY — Phase 14D requires profile identity to be validated and hash-bound.
//! RO:INTERACTS — ron-policy economics loader and configs/roc-economics.toml.
//! RO:INVARIANTS — explicit profile; mismatch fails closed; profile affects config identity.
//! RO:METRICS — none.
//! RO:CONFIG — canonical checked-in economics TOML.
//! RO:SECURITY — no wallet, ledger, payout, receipt, or finality authority.
//! RO:TEST — this integration test target.

#![allow(clippy::missing_panics_doc)]

use ron_policy::economics::{
    internal_roc_economics_config_hash, load_internal_roc_economics_toml,
    load_internal_roc_economics_toml_for_profile, validate_internal_roc_economics_config,
    InternalRocEconomicsProfile,
};

const CANONICAL: &str = include_str!("../../../configs/roc-economics.toml");

#[test]
fn canonical_document_carries_explicit_profile() {
    let config = load_internal_roc_economics_toml(CANONICAL.as_bytes())
        .expect("canonical economics must validate");

    assert_eq!(config.profile, InternalRocEconomicsProfile::Canonical);

    let validation = validate_internal_roc_economics_config(&config).expect("validation marker");

    assert_eq!(validation.profile, InternalRocEconomicsProfile::Canonical);
}

#[test]
fn selected_profile_must_match_document() {
    load_internal_roc_economics_toml_for_profile(
        CANONICAL.as_bytes(),
        InternalRocEconomicsProfile::Canonical,
    )
    .expect("canonical selection must accept canonical document");

    let error = load_internal_roc_economics_toml_for_profile(
        CANONICAL.as_bytes(),
        InternalRocEconomicsProfile::Development,
    )
    .expect_err("development selection must reject canonical document");

    assert!(error.to_string().contains("economics profile mismatch"));
}

#[test]
fn unknown_profile_label_rejects() {
    let invalid = CANONICAL.replacen("profile = \"canonical\"", "profile = \"unreviewed\"", 1);

    let error = load_internal_roc_economics_toml(invalid.as_bytes())
        .expect_err("unknown profile must reject");

    assert!(error.to_string().contains("parse error"));
}

#[test]
fn profile_identity_changes_canonical_hash() {
    let canonical = load_internal_roc_economics_toml(CANONICAL.as_bytes())
        .expect("canonical economics must validate");

    let mut development = canonical.clone();
    development.profile = InternalRocEconomicsProfile::Development;

    assert_ne!(
        internal_roc_economics_config_hash(&canonical).unwrap(),
        internal_roc_economics_config_hash(&development).unwrap()
    );
}
