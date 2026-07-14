//! RO:WHAT — Phase 18 economics-schema tests for probation Service Node reward caps.
//!
//! RO:WHY — The probation ceiling must be explicit, reviewed, and centrally
//! configured rather than hardcoded in rewarder or registry code.
//!
//! RO:INVARIANTS — both profiles carry a positive cap; missing or oversized
//! values fail closed; config identity changes when the cap changes.

use ron_policy::economics::{internal_roc_economics_config_hash, load_internal_roc_economics_toml};

const CANONICAL: &[u8] = include_bytes!("../../../configs/roc-economics.toml");

const DEVELOPMENT: &[u8] = include_bytes!("../../../configs/roc-economics.dev.toml");

fn assert_valid_probation_cap(bytes: &[u8]) {
    let config =
        load_internal_roc_economics_toml(bytes).expect("economics profile should validate");

    let probation = config
        .anti_farming
        .probation_reward_cap_minor_per_node_per_epoch
        .parse::<u128>()
        .expect("probation cap should parse");

    let normal = config
        .anti_farming
        .max_reward_minor_per_account_per_epoch
        .parse::<u128>()
        .expect("normal account cap should parse");

    assert!(probation > 0);
    assert!(probation <= normal);
}

#[test]
fn canonical_and_development_profiles_define_probation_cap() {
    assert_valid_probation_cap(CANONICAL);
    assert_valid_probation_cap(DEVELOPMENT);
}

#[test]
fn missing_probation_cap_is_rejected() {
    let config =
        load_internal_roc_economics_toml(CANONICAL).expect("canonical economics should load");

    let line = format!(
        "probation_reward_cap_minor_per_node_per_epoch = \"{}\"",
        config
            .anti_farming
            .probation_reward_cap_minor_per_node_per_epoch
    );

    let raw = std::str::from_utf8(CANONICAL).expect("canonical economics should be UTF-8");

    assert_eq!(raw.matches(&line).count(), 1);

    let missing = raw.replacen(&line, "", 1);

    assert!(
        load_internal_roc_economics_toml(missing.as_bytes()).is_err(),
        "missing required probation cap must reject"
    );
}

#[test]
fn probation_cap_above_normal_account_cap_is_rejected() {
    let config =
        load_internal_roc_economics_toml(CANONICAL).expect("canonical economics should load");

    let normal = config
        .anti_farming
        .max_reward_minor_per_account_per_epoch
        .parse::<u128>()
        .expect("normal account cap should parse");

    let old = format!(
        "probation_reward_cap_minor_per_node_per_epoch = \"{}\"",
        config
            .anti_farming
            .probation_reward_cap_minor_per_node_per_epoch
    );

    let new = format!(
        "probation_reward_cap_minor_per_node_per_epoch = \"{}\"",
        normal + 1
    );

    let raw = std::str::from_utf8(CANONICAL).expect("canonical economics should be UTF-8");

    let oversized = raw.replacen(&old, &new, 1);

    assert!(
        load_internal_roc_economics_toml(oversized.as_bytes()).is_err(),
        "probation cap above normal account cap must reject"
    );
}

#[test]
fn probation_cap_changes_canonical_economics_identity() {
    let original =
        load_internal_roc_economics_toml(CANONICAL).expect("canonical economics should load");

    let current = original
        .anti_farming
        .probation_reward_cap_minor_per_node_per_epoch
        .parse::<u128>()
        .expect("probation cap should parse");

    assert!(current > 1);

    let old = format!("probation_reward_cap_minor_per_node_per_epoch = \"{current}\"");

    let new = format!(
        "probation_reward_cap_minor_per_node_per_epoch = \"{}\"",
        current - 1
    );

    let raw = std::str::from_utf8(CANONICAL).expect("canonical economics should be UTF-8");

    let changed = load_internal_roc_economics_toml(raw.replacen(&old, &new, 1).as_bytes())
        .expect("changed complete economics should validate");

    assert_ne!(
        internal_roc_economics_config_hash(&original).expect("original hash"),
        internal_roc_economics_config_hash(&changed).expect("changed hash")
    );
}
