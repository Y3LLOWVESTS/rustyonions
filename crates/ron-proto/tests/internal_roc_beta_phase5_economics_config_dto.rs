//! RO:WHAT — Internal ROC Beta Phase 5 economics-config DTO validation tests.
//! RO:WHY — ECON/GOV: tokenomics config must be integer-safe, bounded, and non-authoritative.
//! RO:INTERACTS — ron_proto::econ internal ROC economics DTOs and configs/roc-economics.toml.
//! RO:INVARIANTS — no floats; bps totals exact; bridge/staking inert; config creates no receipt/balance truth.
//! RO:METRICS — none.
//! RO:CONFIG — mirrors configs/roc-economics.toml shape without parsing TOML in ron-proto.
//! RO:SECURITY — no wallet authority, ledger mutation, paid entitlement, bridge, or staking runtime.
//! RO:TEST — cargo test -p ron-proto --test internal_roc_beta_phase5_economics_config_dto.

use ron_proto::{
    InternalRocAntiFarmingConfigV1, InternalRocBpsSplitV1, InternalRocEconomicsConfigV1,
    InternalRocEconomicsConfigValidationError, InternalRocFutureFeaturePlaceholderV1,
    InternalRocPaidContentEconomicsV1, InternalRocRemainderSinkV1, InternalRocRewardCategoryCapV1,
    InternalRocRewardPoolEconomicsV1, InternalRocRoundingConfigV1, InternalRocRoundingModeV1,
    InternalRocUnitsV1, INTERNAL_ROC_BPS_DENOMINATOR, INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA,
    INTERNAL_ROC_ECONOMICS_CONFIG_VERSION,
};
use serde_json::{json, Value};

fn valid_config() -> InternalRocEconomicsConfigV1 {
    InternalRocEconomicsConfigV1 {
        schema: INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA.to_string(),
        version: INTERNAL_ROC_ECONOMICS_CONFIG_VERSION,
        units: InternalRocUnitsV1 {
            money: "integer_minor_units".to_string(),
            minor_unit_name: "roc_minor".to_string(),
            minor_units_per_roc: "1000000".to_string(),
            basis_point_denominator: INTERNAL_ROC_BPS_DENOMINATOR,
        },
        paid_content: InternalRocPaidContentEconomicsV1 {
            minimum_price_minor: "1".to_string(),
            default_splits: vec![
                InternalRocBpsSplitV1 {
                    label: "creator".to_string(),
                    account_role: "creator".to_string(),
                    bps: 8500,
                },
                InternalRocBpsSplitV1 {
                    label: "treasury".to_string(),
                    account_role: "treasury".to_string(),
                    bps: 1000,
                },
                InternalRocBpsSplitV1 {
                    label: "storage_provider".to_string(),
                    account_role: "storage_provider".to_string(),
                    bps: 500,
                },
            ],
        },
        reward_pools: InternalRocRewardPoolEconomicsV1 {
            epoch_pool_cap_minor: "1000000".to_string(),
            category_caps: vec![
                InternalRocRewardCategoryCapV1 {
                    category: "creator_rewards".to_string(),
                    pool_bps: 7000,
                    category_cap_minor: "700000".to_string(),
                },
                InternalRocRewardCategoryCapV1 {
                    category: "node_delivery".to_string(),
                    pool_bps: 2000,
                    category_cap_minor: "200000".to_string(),
                },
                InternalRocRewardCategoryCapV1 {
                    category: "moderation".to_string(),
                    pool_bps: 1000,
                    category_cap_minor: "100000".to_string(),
                },
            ],
        },
        anti_farming: InternalRocAntiFarmingConfigV1 {
            max_events_per_account_per_epoch: 1000,
            max_reward_minor_per_account_per_epoch: "10000".to_string(),
            max_reward_minor_per_content_per_epoch: "50000".to_string(),
        },
        rounding: InternalRocRoundingConfigV1 {
            mode: InternalRocRoundingModeV1::Floor,
            remainder_sink: InternalRocRemainderSinkV1::Treasury,
            remainder_sink_account: None,
        },
        future_bridge: InternalRocFutureFeaturePlaceholderV1 {
            enabled: false,
            state: "disabled_inert_placeholder".to_string(),
        },
        future_staking: InternalRocFutureFeaturePlaceholderV1 {
            enabled: false,
            state: "disabled_inert_placeholder".to_string(),
        },
    }
}

fn valid_json() -> Value {
    json!({
        "schema": INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA,
        "version": INTERNAL_ROC_ECONOMICS_CONFIG_VERSION,
        "units": {
            "money": "integer_minor_units",
            "minor_unit_name": "roc_minor",
            "minor_units_per_roc": "1000000",
            "basis_point_denominator": INTERNAL_ROC_BPS_DENOMINATOR
        },
        "paid_content": {
            "minimum_price_minor": "1",
            "default_splits": [
                { "label": "creator", "account_role": "creator", "bps": 8500 },
                { "label": "treasury", "account_role": "treasury", "bps": 1000 },
                { "label": "storage_provider", "account_role": "storage_provider", "bps": 500 }
            ]
        },
        "reward_pools": {
            "epoch_pool_cap_minor": "1000000",
            "category_caps": [
                { "category": "creator_rewards", "pool_bps": 7000, "category_cap_minor": "700000" },
                { "category": "node_delivery", "pool_bps": 2000, "category_cap_minor": "200000" },
                { "category": "moderation", "pool_bps": 1000, "category_cap_minor": "100000" }
            ]
        },
        "anti_farming": {
            "max_events_per_account_per_epoch": 1000,
            "max_reward_minor_per_account_per_epoch": "10000",
            "max_reward_minor_per_content_per_epoch": "50000"
        },
        "rounding": {
            "mode": "floor",
            "remainder_sink": "treasury"
        },
        "future_bridge": {
            "enabled": false,
            "state": "disabled_inert_placeholder"
        },
        "future_staking": {
            "enabled": false,
            "state": "disabled_inert_placeholder"
        }
    })
}

#[test]
fn valid_economics_config_shape_validates_without_authority() {
    let config = valid_config();
    config.validate().expect("valid economics config");

    assert_eq!(config.schema, INTERNAL_ROC_ECONOMICS_CONFIG_SCHEMA);
    assert_eq!(config.units.basis_point_denominator, 10_000);
    assert_eq!(
        config.rounding.remainder_sink,
        InternalRocRemainderSinkV1::Treasury
    );
    assert!(!config.future_bridge.enabled);
    assert!(!config.future_staking.enabled);
}

#[test]
fn serde_shape_rejects_unknown_fields() {
    let mut value = valid_json();
    value["surprise_wallet_authority"] = json!(true);

    let err = serde_json::from_value::<InternalRocEconomicsConfigV1>(value)
        .expect_err("unknown fields must reject");
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn serde_shape_rejects_float_or_numeric_money() {
    let mut numeric_money = valid_json();
    numeric_money["paid_content"]["minimum_price_minor"] = json!(1);

    let err = serde_json::from_value::<InternalRocEconomicsConfigV1>(numeric_money)
        .expect_err("numeric money must reject before validation");
    assert!(err.to_string().contains("invalid type"));

    let mut float_bps = valid_json();
    float_bps["paid_content"]["default_splits"][0]["bps"] = json!(8500.5);

    let err = serde_json::from_value::<InternalRocEconomicsConfigV1>(float_bps)
        .expect_err("float bps must reject before validation");
    assert!(err.to_string().contains("invalid type"));
}

#[test]
fn malformed_minor_unit_money_rejects() {
    let mut config = valid_config();
    config.anti_farming.max_reward_minor_per_account_per_epoch = "01".to_string();

    let err = config
        .validate()
        .expect_err("leading-zero money must reject");
    assert!(matches!(
        err,
        InternalRocEconomicsConfigValidationError::InvalidMoney { .. }
    ));
}

#[test]
fn invalid_bps_totals_reject() {
    let mut config = valid_config();
    config.paid_content.default_splits[0].bps = 8000;

    let err = config
        .validate()
        .expect_err("paid-content split total must reject");
    assert!(matches!(
        err,
        InternalRocEconomicsConfigValidationError::InvalidBpsTotal {
            field: "paid_content.default_splits",
            expected_bps: 10_000,
            actual_bps: 9500,
        }
    ));

    let mut config = valid_config();
    config.reward_pools.category_caps[0].pool_bps = 6000;

    let err = config
        .validate()
        .expect_err("reward category total must reject");
    assert!(matches!(
        err,
        InternalRocEconomicsConfigValidationError::InvalidBpsTotal {
            field: "reward_pools.category_caps",
            expected_bps: 10_000,
            actual_bps: 9000,
        }
    ));
}

#[test]
fn missing_remainder_sink_rejects_at_wire_shape() {
    let mut value = valid_json();
    value["rounding"]
        .as_object_mut()
        .expect("rounding object")
        .remove("remainder_sink");

    let err = serde_json::from_value::<InternalRocEconomicsConfigV1>(value)
        .expect_err("missing remainder sink must reject");
    assert!(err.to_string().contains("missing field"));
}

#[test]
fn bridge_and_staking_placeholders_must_remain_inert() {
    let mut config = valid_config();
    config.future_bridge.enabled = true;

    let err = config
        .validate()
        .expect_err("enabled bridge placeholder must reject");
    assert!(matches!(
        err,
        InternalRocEconomicsConfigValidationError::FutureFeatureEnabled {
            field: "future_bridge.enabled"
        }
    ));

    let mut config = valid_config();
    config.future_staking.enabled = true;

    let err = config
        .validate()
        .expect_err("enabled staking placeholder must reject");
    assert!(matches!(
        err,
        InternalRocEconomicsConfigValidationError::FutureFeatureEnabled {
            field: "future_staking.enabled"
        }
    ));
}

#[test]
fn configured_account_remainder_sink_requires_explicit_account() {
    let mut config = valid_config();
    config.rounding.remainder_sink = InternalRocRemainderSinkV1::ConfiguredAccount;

    let err = config
        .validate()
        .expect_err("configured account sink requires explicit account");
    assert!(matches!(
        err,
        InternalRocEconomicsConfigValidationError::MissingRemainderSinkAccount
    ));

    config.rounding.remainder_sink_account = Some("account:treasury_beta".to_string());
    config
        .validate()
        .expect("configured account sink with explicit account validates");
}
