//! RO:WHAT — Tests the complete standalone development ROC economics profile.
//! RO:WHY — Phase 14D requires canonical and development profiles to share one strict schema.
//! RO:INTERACTS — ron-policy economics loading, validation, profile selection, and hashing.
//! RO:INVARIANTS — no overlay; dev values preserved; explicit profile; distinct deterministic identity.
//! RO:METRICS — none.
//! RO:CONFIG — configs/roc-economics.toml and configs/roc-economics.dev.toml.
//! RO:SECURITY — policy only; no wallet, ledger, payout, receipt, settlement, or finality authority.
//! RO:TEST — this integration test target.

#![allow(clippy::missing_panics_doc)]

use ron_policy::economics::{
    internal_roc_economics_config_hash, load_internal_roc_economics_toml,
    load_internal_roc_economics_toml_for_profile, InternalRocEconomicsConfig,
    InternalRocEconomicsProfile,
};

const CANONICAL: &str = include_str!("../../../configs/roc-economics.toml");

const DEVELOPMENT: &str = include_str!("../../../configs/roc-economics.dev.toml");

fn development_config() -> InternalRocEconomicsConfig {
    load_internal_roc_economics_toml_for_profile(
        DEVELOPMENT.as_bytes(),
        InternalRocEconomicsProfile::Development,
    )
    .expect("development economics profile must validate")
}

#[test]
fn development_document_is_complete_and_standalone() {
    let config = development_config();

    assert_eq!(config.profile, InternalRocEconomicsProfile::Development);

    assert_eq!(config.units.minor_unit_name, "roc_minor");

    assert_eq!(config.reward_pools.epoch_pool_cap_minor, "1000000");

    assert!(!config.future_bridge.enabled);
    assert!(!config.future_staking.enabled);
}

#[test]
fn development_paid_action_values_are_preserved() {
    let config = development_config();

    assert_eq!(
        config
            .paid_actions
            .price_for("paid_storage_put", 1)
            .unwrap(),
        25
    );

    assert_eq!(
        config
            .paid_actions
            .price_for("paid_storage_pin", 1)
            .unwrap(),
        3
    );

    assert_eq!(
        config
            .paid_actions
            .price_for("paid_content_view", 1)
            .unwrap(),
        5
    );

    assert_eq!(
        config.paid_actions.price_for("paid_song_play", 1).unwrap(),
        8
    );

    assert_eq!(config.paid_actions.price_for("site_visit", 1).unwrap(), 2);
}

#[test]
fn profile_selection_rejects_cross_profile_loading() {
    load_internal_roc_economics_toml_for_profile(
        DEVELOPMENT.as_bytes(),
        InternalRocEconomicsProfile::Development,
    )
    .expect("development selection must accept development document");

    load_internal_roc_economics_toml_for_profile(
        CANONICAL.as_bytes(),
        InternalRocEconomicsProfile::Canonical,
    )
    .expect("canonical selection must accept canonical document");

    load_internal_roc_economics_toml_for_profile(
        DEVELOPMENT.as_bytes(),
        InternalRocEconomicsProfile::Canonical,
    )
    .expect_err("canonical selection must reject development document");

    load_internal_roc_economics_toml_for_profile(
        CANONICAL.as_bytes(),
        InternalRocEconomicsProfile::Development,
    )
    .expect_err("development selection must reject canonical document");
}

#[test]
fn canonical_and_development_profiles_have_distinct_hashes() {
    let canonical = load_internal_roc_economics_toml(CANONICAL.as_bytes())
        .expect("canonical economics must validate");

    let development = development_config();

    assert_ne!(
        internal_roc_economics_config_hash(&canonical).unwrap(),
        internal_roc_economics_config_hash(&development).unwrap()
    );
}
