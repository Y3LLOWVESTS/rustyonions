//! RO:WHAT — Economics-config non-authority tests for ron-policy QuickChain preflight.
//! RO:WHY — Ensures ROC economics config stays strict configuration, not ledger mutation or settlement truth.
//! RO:INTERACTS — `configs/roc-economics.toml`, `ron_policy::economics`.
//! RO:INVARIANTS — integer minor-unit config only; no receipts, balances, finality, roots, or settlement fields.

use ron_policy::economics::load_economics_toml_str;
use serde_json::Value;

const CHECKED_IN_POLICY: &str = include_str!("../../../configs/roc-economics.toml");

#[test]
fn economics_config_rejects_top_level_authority_field() {
    let bad = CHECKED_IN_POLICY.replacen(
        "version = 1",
        "version = 1\nreceipt_id = \"fake-policy-receipt\"",
        1,
    );

    let err = match load_economics_toml_str(&bad) {
        Ok(_) => panic!("unknown top-level receipt authority field must reject"),
        Err(err) => err,
    };

    assert!(
        err.to_string().contains("parse error"),
        "unexpected error: {err}"
    );
}

#[test]
fn economics_action_config_rejects_authority_field() {
    let bad =
        CHECKED_IN_POLICY.replacen("enabled = true", "enabled = true\nunlock_granted = true", 1);

    let err = match load_economics_toml_str(&bad) {
        Ok(_) => panic!("unknown action-level unlock authority field must reject"),
        Err(err) => err,
    };

    assert!(
        err.to_string().contains("parse error"),
        "unexpected error: {err}"
    );
}

#[test]
fn economics_config_serialized_shape_has_no_receipt_balance_finality_or_root_fields() {
    let policy = load_economics_toml_str(CHECKED_IN_POLICY)
        .expect("checked-in economics config should load");
    let value = serde_json::to_value(&policy).expect("economics config should serialize");

    assert_json_keys_are_not_authority_fields(&value);
}

#[test]
fn economics_helpers_return_amounts_and_validation_only_not_truth_artifacts() {
    let policy = load_economics_toml_str(CHECKED_IN_POLICY)
        .expect("checked-in economics config should load");

    let price = policy
        .price_for("paid_storage_put", 48)
        .expect("price estimate should calculate");

    assert_eq!(price, 84);

    let debug = format!("{price:?}");
    for token in [
        "receipt",
        "balance",
        "finality",
        "unlock",
        "settlement",
        "state_root",
        "checkpoint",
    ] {
        assert!(
            !debug.to_ascii_lowercase().contains(token),
            "price helper must return amount only, not authority artifact token: {token}"
        );
    }
}

fn assert_json_keys_are_not_authority_fields(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                assert!(
                    !is_forbidden_authority_key(key),
                    "economics config serialized authority-shaped key: {key}"
                );
                assert_json_keys_are_not_authority_fields(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_json_keys_are_not_authority_fields(item);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn is_forbidden_authority_key(key: &str) -> bool {
    matches!(
        normalize_key(key).as_str(),
        "receiptid"
            | "receipthash"
            | "receiptroot"
            | "balance"
            | "balanceminor"
            | "walletbalance"
            | "ledgerbalance"
            | "finality"
            | "finalized"
            | "unlockgranted"
            | "paidproof"
            | "settlementstatus"
            | "stateroot"
            | "checkpointroot"
            | "checkpointhash"
            | "validatorsignature"
            | "mintauthority"
    )
}

fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}
