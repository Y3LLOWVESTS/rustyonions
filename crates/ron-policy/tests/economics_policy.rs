#![allow(clippy::too_many_lines)]

use std::collections::BTreeMap;

use ron_policy::economics::{load_economics_toml_str, validate_economics_policy, EconomicsPolicy};

const CHECKED_IN_POLICY: &str = include_str!("../../../configs/roc-economics.toml");

fn load_checked_in() -> EconomicsPolicy {
    load_economics_toml_str(CHECKED_IN_POLICY).expect("checked-in economics config should load")
}

#[test]
fn valid_checked_in_roc_economics_config_loads() {
    let policy = load_checked_in();

    assert_eq!(policy.version, 1);
    assert_eq!(policy.unit, "roc_minor");
    assert_eq!(policy.default_asset, "roc");
    assert_eq!(policy.remainder_sink, "treasury");
    assert_eq!(policy.actions.len(), 5);
}

#[test]
fn deterministic_action_order_is_sorted() {
    let policy = load_checked_in();

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
}

#[test]
fn invalid_split_sum_rejects() {
    let bad = CHECKED_IN_POLICY.replacen("bps = 500", "bps = 499", 1);
    let err = load_economics_toml_str(&bad).expect_err("split mismatch must reject");

    assert!(err.to_string().contains("split bps must sum to 10000"));
}

#[test]
fn unknown_split_destination_rejects() {
    let bad = CHECKED_IN_POLICY.replacen("to = \"treasury\"", "to = \"ghost_sink\"", 1);
    let err = load_economics_toml_str(&bad).expect_err("unknown split destination must reject");

    assert!(err.to_string().contains("unknown split destination"));
}

#[test]
fn unknown_action_rejects() {
    let bad = format!(
        "{CHECKED_IN_POLICY}\n\
         [actions.mystery_action]\n\
         enabled = true\n\
         pricing_kind = \"flat\"\n\
         price_minor = 1\n\
         minimum_charge_minor = 1\n\
         max_spend_minor = 10\n\
         max_hold_multiplier_bps = 10000\n\
         [[actions.mystery_action.splits]]\n\
         to = \"treasury\"\n\
         bps = 10000\n"
    );

    let err = load_economics_toml_str(&bad).expect_err("unknown action must reject");
    assert!(err.to_string().contains("unknown economics action"));
}

#[test]
fn disabled_action_rejects_lookup_but_config_can_load() {
    let raw = CHECKED_IN_POLICY.replacen(
        "[actions.paid_song_play]\nenabled = true",
        "[actions.paid_song_play]\nenabled = false",
        1,
    );

    let policy = load_economics_toml_str(&raw).expect("disabled action config can load");
    let err = policy
        .price_for("paid_song_play", 1)
        .expect_err("disabled action lookup must reject");

    assert!(err.to_string().contains("economics action disabled"));
}

#[test]
fn float_value_rejects_during_parse() {
    let bad =
        CHECKED_IN_POLICY.replacen("price_per_byte_minor = 1", "price_per_byte_minor = 1.5", 1);

    let err = load_economics_toml_str(&bad).expect_err("float money value must reject");
    assert!(err.to_string().contains("parse error"));
}

#[test]
fn negative_value_rejects_during_parse() {
    let bad =
        CHECKED_IN_POLICY.replacen("minimum_charge_minor = 70", "minimum_charge_minor = -70", 1);

    let err = load_economics_toml_str(&bad).expect_err("negative money value must reject");
    assert!(err.to_string().contains("parse error"));
}

#[test]
fn overflow_value_rejects_during_parse() {
    let bad = CHECKED_IN_POLICY.replacen(
        "minimum_charge_minor = 70",
        "minimum_charge_minor = 9223372036854775808",
        1,
    );

    let err = load_economics_toml_str(&bad).expect_err("overflow money value must reject");
    assert!(err.to_string().contains("parse error"));
}

#[test]
fn missing_required_action_rejects() {
    let mut policy = load_checked_in();
    policy.actions.remove("site_visit");

    let err = validate_economics_policy(&policy).expect_err("missing required action must reject");

    assert!(err
        .to_string()
        .contains("missing required economics action"));
}

#[test]
fn paid_storage_put_price_uses_minimum_and_hold_multiplier() {
    let policy = load_checked_in();

    assert_eq!(policy.price_for("paid_storage_put", 1).unwrap(), 84);
    assert_eq!(policy.price_for("paid_storage_put", 48).unwrap(), 84);
    assert_eq!(policy.price_for("paid_storage_put", 100).unwrap(), 120);
}

#[test]
fn unknown_paid_action_lookup_rejects() {
    let policy = load_checked_in();
    let err = policy
        .price_for("unknown_action", 1)
        .expect_err("unknown action lookup must reject");

    assert!(err.to_string().contains("unknown economics action"));
}

#[test]
fn missing_dynamic_recipient_rejects_capture_plan() {
    let policy = load_checked_in();
    let recipients = BTreeMap::<String, String>::new();

    let err = policy
        .validate_capture_plan("paid_storage_put", &recipients, 70)
        .expect_err("storage_provider recipient is required");

    assert!(err.to_string().contains("missing recipient"));
}

#[test]
fn capture_plan_accepts_required_dynamic_recipient() {
    let policy = load_checked_in();
    let mut recipients = BTreeMap::<String, String>::new();
    recipients.insert(
        "storage_provider".to_string(),
        "t:default/provider/storage/w".to_string(),
    );

    policy
        .validate_capture_plan("paid_storage_put", &recipients, 70)
        .expect("valid paid_storage_put capture plan should pass");
}

#[test]
fn capture_over_action_cap_rejects() {
    let policy = load_checked_in();
    let mut recipients = BTreeMap::<String, String>::new();
    recipients.insert(
        "storage_provider".to_string(),
        "t:default/provider/storage/w".to_string(),
    );

    let err = policy
        .validate_capture_plan("paid_storage_put", &recipients, 100_001)
        .expect_err("capture over action cap must reject");

    assert!(err.to_string().contains("exceeds max_spend_minor"));
}
