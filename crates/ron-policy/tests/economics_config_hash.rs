//! RO:WHAT — Deterministic identity tests for canonical Internal ROC economics.
//! RO:WHY — Phase 14D requires config-bound accounting and reward planning.
//! RO:INTERACTS — ron_policy::economics and configs/roc-economics.toml.
//! RO:INVARIANTS — validated model only; deterministic ordering; b3 identity; no economic authority.
//! RO:METRICS — none.
//! RO:CONFIG — canonical checked-in economics TOML.
//! RO:SECURITY — hash identity only; no wallet, ledger, payout, receipt, or finality mutation.
//! RO:TEST — this integration test target.

#![allow(clippy::missing_panics_doc)]

use ron_policy::economics::{
    canonical_internal_roc_economics_bytes, internal_roc_economics_config_hash,
    load_internal_roc_economics_toml,
};

const CANONICAL: &str = include_str!("../../../configs/roc-economics.toml");

fn canonical_config() -> ron_policy::economics::InternalRocEconomicsConfig {
    load_internal_roc_economics_toml(CANONICAL.as_bytes())
        .expect("canonical economics config must validate")
}

#[test]
fn canonical_hash_is_replay_stable_and_b3_shaped() {
    let config = canonical_config();

    let first = internal_roc_economics_config_hash(&config).unwrap();

    let second = internal_roc_economics_config_hash(&config).unwrap();

    assert_eq!(first, second);

    let hex = first
        .strip_prefix("b3:")
        .expect("economics hash must use b3 prefix");

    assert_eq!(hex.len(), 64);

    assert!(hex
        .bytes()
        .all(|byte| { byte.is_ascii_digit() || matches!(byte, b'a'..=b'f') }));
}

#[test]
fn comments_and_whitespace_do_not_change_hash() {
    let original = canonical_config();

    let decorated = format!("# non-semantic review comment\n\n{CANONICAL}\n\n");

    let reparsed = load_internal_roc_economics_toml(decorated.as_bytes())
        .expect("decorated economics config must validate");

    assert_eq!(
        internal_roc_economics_config_hash(&original).unwrap(),
        internal_roc_economics_config_hash(&reparsed).unwrap()
    );
}

#[test]
fn split_and_category_row_order_is_normalized() {
    let original = canonical_config();
    let mut reordered = original.clone();

    reordered.paid_content.default_splits.reverse();

    reordered.reward_pools.category_caps.reverse();

    assert_eq!(
        canonical_internal_roc_economics_bytes(&original).unwrap(),
        canonical_internal_roc_economics_bytes(&reordered).unwrap()
    );

    assert_eq!(
        internal_roc_economics_config_hash(&original).unwrap(),
        internal_roc_economics_config_hash(&reordered).unwrap()
    );
}

#[test]
fn semantic_economics_change_changes_hash() {
    let original = canonical_config();
    let mut changed = original.clone();

    changed.paid_content.minimum_price_minor = "2".to_string();

    assert_ne!(
        internal_roc_economics_config_hash(&original).unwrap(),
        internal_roc_economics_config_hash(&changed).unwrap()
    );
}
