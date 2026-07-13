//! RO:WHAT — Tests the paid-action policy embedded in the unified ROC economics config.
//! RO:WHY — Phase 14D requires one validated, deterministic economics identity.
//! RO:INTERACTS — ron-policy Internal ROC and paid-action economics models.
//! RO:INVARIANTS — reviewed values locked; nested validation enforced; hash order stable.
//! RO:METRICS — none.
//! RO:CONFIG — configs/roc-economics.toml.
//! RO:SECURITY — policy only; no wallet, ledger, payout, receipt, or finality authority.
//! RO:TEST — this integration test target.

#![allow(clippy::missing_panics_doc, clippy::too_many_lines)]

use ron_policy::economics::{
    canonical_internal_roc_economics_bytes, internal_roc_economics_config_hash,
    load_internal_roc_economics_toml, validate_internal_roc_economics_config, ActionEconomics,
    InternalRocEconomicsConfig, PricingKind,
};

const CANONICAL: &str = include_str!("../../../configs/roc-economics.toml");

fn canonical_config() -> InternalRocEconomicsConfig {
    load_internal_roc_economics_toml(CANONICAL.as_bytes())
        .expect("canonical economics config must validate")
}

fn split_rows(action: &ActionEconomics) -> Vec<(&str, u16)> {
    action
        .splits
        .iter()
        .map(|split| (split.to.as_str(), split.bps))
        .collect()
}

#[test]
fn canonical_config_locks_reviewed_paid_action_values() {
    let config = canonical_config();
    let policy = &config.paid_actions;

    assert_eq!(policy.version, 1);
    assert_eq!(policy.unit, "roc_minor");
    assert_eq!(policy.default_asset, "roc");
    assert_eq!(policy.remainder_sink, "treasury");

    assert_eq!(
        policy.action_ids(),
        vec![
            "paid_content_view".to_string(),
            "paid_song_play".to_string(),
            "paid_storage_pin".to_string(),
            "paid_storage_put".to_string(),
            "site_visit".to_string(),
        ]
    );

    let storage_put = policy
        .actions
        .get("paid_storage_put")
        .expect("paid_storage_put must exist");

    assert!(storage_put.enabled);
    assert_eq!(storage_put.pricing_kind, PricingKind::PerBytePlusMinimum);
    assert_eq!(storage_put.price_per_byte_minor, Some(1));
    assert_eq!(storage_put.minimum_charge_minor, 70);
    assert_eq!(storage_put.max_spend_minor, 100_000);
    assert_eq!(storage_put.max_hold_multiplier_bps, 12_000);
    assert_eq!(
        split_rows(storage_put),
        vec![("storage_provider", 9_500), ("treasury", 500),]
    );

    assert_eq!(policy.price_for("paid_storage_put", 1).unwrap(), 84);
    assert_eq!(policy.price_for("paid_storage_put", 48).unwrap(), 84);
    assert_eq!(policy.price_for("paid_storage_put", 100).unwrap(), 120);

    let storage_pin = policy
        .actions
        .get("paid_storage_pin")
        .expect("paid_storage_pin must exist");

    assert!(storage_pin.enabled);
    assert_eq!(storage_pin.pricing_kind, PricingKind::Flat);
    assert_eq!(storage_pin.price_minor, Some(25));
    assert_eq!(storage_pin.minimum_charge_minor, 25);
    assert_eq!(storage_pin.max_spend_minor, 100_000);
    assert_eq!(storage_pin.max_hold_multiplier_bps, 10_000);
    assert_eq!(
        split_rows(storage_pin),
        vec![("storage_provider", 8_500), ("treasury", 1_500),]
    );

    let content_view = policy
        .actions
        .get("paid_content_view")
        .expect("paid_content_view must exist");

    assert!(content_view.enabled);
    assert_eq!(content_view.pricing_kind, PricingKind::Flat);
    assert_eq!(content_view.price_minor, Some(5));
    assert_eq!(content_view.minimum_charge_minor, 5);
    assert_eq!(content_view.max_spend_minor, 100_000);
    assert_eq!(content_view.max_hold_multiplier_bps, 10_000);
    assert_eq!(
        split_rows(content_view),
        vec![("content_owner", 9_000), ("treasury", 1_000),]
    );

    let song_play = policy
        .actions
        .get("paid_song_play")
        .expect("paid_song_play must exist");

    assert!(song_play.enabled);
    assert_eq!(song_play.pricing_kind, PricingKind::PerSecondPlusMinimum);
    assert_eq!(song_play.price_per_second_minor, Some(1));
    assert_eq!(song_play.minimum_charge_minor, 30);
    assert_eq!(song_play.max_spend_minor, 100_000);
    assert_eq!(song_play.max_hold_multiplier_bps, 10_000);
    assert_eq!(
        split_rows(song_play),
        vec![("content_owner", 9_000), ("treasury", 1_000),]
    );

    assert_eq!(policy.price_for("paid_song_play", 1).unwrap(), 30);
    assert_eq!(policy.price_for("paid_song_play", 31).unwrap(), 31);

    let site_visit = policy
        .actions
        .get("site_visit")
        .expect("site_visit must exist");

    assert!(site_visit.enabled);
    assert_eq!(site_visit.pricing_kind, PricingKind::Flat);
    assert_eq!(site_visit.price_minor, Some(3));
    assert_eq!(site_visit.minimum_charge_minor, 3);
    assert_eq!(site_visit.max_spend_minor, 100_000);
    assert_eq!(site_visit.max_hold_multiplier_bps, 10_000);
    assert_eq!(
        split_rows(site_visit),
        vec![("content_owner", 9_000), ("treasury", 1_000),]
    );
}

#[test]
fn paid_action_split_order_is_normalized_for_hashing() {
    let original = canonical_config();
    let mut reordered = original.clone();

    for action in reordered.paid_actions.actions.values_mut() {
        action.splits.reverse();
    }

    assert_eq!(
        canonical_internal_roc_economics_bytes(&original).unwrap(),
        canonical_internal_roc_economics_bytes(&reordered).unwrap()
    );
}

#[test]
fn invalid_nested_paid_action_policy_rejects() {
    let mut config = canonical_config();

    let action = config
        .paid_actions
        .actions
        .get_mut("paid_storage_put")
        .expect("paid_storage_put must exist");

    action.splits[0].bps -= 1;

    let error = validate_internal_roc_economics_config(&config)
        .expect_err("invalid nested split total must reject");

    assert!(error.to_string().contains("split bps must sum to 10000"));
}

#[test]
fn paid_action_value_changes_config_hash() {
    let original = canonical_config();
    let mut changed = original.clone();

    let action = changed
        .paid_actions
        .actions
        .get_mut("paid_storage_pin")
        .expect("paid_storage_pin must exist");

    action.price_minor = Some(26);

    assert_ne!(
        internal_roc_economics_config_hash(&original).unwrap(),
        internal_roc_economics_config_hash(&changed).unwrap()
    );
}
