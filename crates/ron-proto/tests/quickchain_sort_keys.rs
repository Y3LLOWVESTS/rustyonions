//! RO:WHAT — Proves exact QuickChain account/hold sort-key bytes and deterministic ordering.
//! RO:WHY — ECON/RES: future roots must not inherit database, map, locale, or arrival ordering.
//! RO:INVARIANTS — bytewise ascending; duplicate keys reject; no hashing and no roots.
//! RO:TEST — file-backed Phase 0 sort-key vectors plus negative input tests.

use ron_proto::{
    quickchain_account_leaf_sort_key_v1, quickchain_hold_leaf_sort_key_v1, sort_quickchain_keys_v1,
    validate_quickchain_sorted_unique_keys_v1, QuickChainValidationError,
    QUICKCHAIN_ACCOUNT_LEAF_ASSET_ROC_V1, QUICKCHAIN_ACCOUNT_LEAF_SORT_KEY_DELIMITER_V1,
};
use serde::Deserialize;
use serde_json::{json, Value};

const SORT_KEY_VECTOR: &str =
    include_str!("vectors/quickchain/sort_keys/sort_keys_locked_bytes_v1.json");

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountInput {
    account_id: String,
    asset: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldInput {
    hold_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountLeafVector {
    account_id: String,
    asset: String,
    expected_sort_key_utf8: String,
    expected_sort_key_hex: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldLeafVector {
    hold_id: String,
    expected_sort_key_utf8: String,
    expected_sort_key_hex: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SortKeyVectorSet {
    schema: String,
    version: u16,
    status: String,
    account_leaf: AccountLeafVector,
    hold_leaf: HoldLeafVector,
    unordered_account_inputs: Vec<AccountInput>,
    expected_account_order_hex: Vec<String>,
    unordered_hold_inputs: Vec<HoldInput>,
    expected_hold_order_hex: Vec<String>,
    duplicate_account_inputs: Vec<AccountInput>,
    expected_duplicate_error: String,
    notes: Vec<String>,
}

fn load_vector() -> SortKeyVectorSet {
    let vector: SortKeyVectorSet = serde_json::from_str(SORT_KEY_VECTOR).unwrap();

    assert_eq!(vector.schema, "quickchain.sort-key-vector-set.v1");
    assert_eq!(vector.version, 1);
    assert_eq!(vector.status, "locked_bytes");
    assert!(!vector.notes.is_empty());
    assert!(vector.notes.iter().all(|note| !note.is_empty()));

    vector
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }

    output
}

#[test]
fn account_and_hold_sort_keys_match_exact_locked_bytes() {
    let vector = load_vector();

    assert_eq!(QUICKCHAIN_ACCOUNT_LEAF_ASSET_ROC_V1, "roc");
    assert_eq!(QUICKCHAIN_ACCOUNT_LEAF_SORT_KEY_DELIMITER_V1, 0x00);

    let account_key = quickchain_account_leaf_sort_key_v1(
        &vector.account_leaf.account_id,
        &vector.account_leaf.asset,
    )
    .unwrap();

    assert_eq!(
        account_key.as_slice(),
        vector.account_leaf.expected_sort_key_utf8.as_bytes()
    );
    assert_eq!(
        lower_hex(&account_key),
        vector.account_leaf.expected_sort_key_hex.as_str()
    );

    let hold_key = quickchain_hold_leaf_sort_key_v1(&vector.hold_leaf.hold_id).unwrap();

    assert_eq!(
        hold_key.as_slice(),
        vector.hold_leaf.expected_sort_key_utf8.as_bytes()
    );
    assert_eq!(
        lower_hex(&hold_key),
        vector.hold_leaf.expected_sort_key_hex.as_str()
    );
}

#[test]
fn unordered_inputs_produce_the_same_locked_bytewise_order() {
    let vector = load_vector();

    let mut account_keys: Vec<Vec<u8>> = vector
        .unordered_account_inputs
        .iter()
        .map(|input| quickchain_account_leaf_sort_key_v1(&input.account_id, &input.asset).unwrap())
        .collect();

    let mut reversed_account_keys = account_keys.clone();
    reversed_account_keys.reverse();

    sort_quickchain_keys_v1(&mut account_keys).unwrap();
    sort_quickchain_keys_v1(&mut reversed_account_keys).unwrap();

    assert_eq!(account_keys, reversed_account_keys);

    let account_hex: Vec<String> = account_keys.iter().map(|key| lower_hex(key)).collect();

    assert_eq!(
        account_hex.as_slice(),
        vector.expected_account_order_hex.as_slice()
    );

    let mut hold_keys: Vec<Vec<u8>> = vector
        .unordered_hold_inputs
        .iter()
        .map(|input| quickchain_hold_leaf_sort_key_v1(&input.hold_id).unwrap())
        .collect();

    let mut reversed_hold_keys = hold_keys.clone();
    reversed_hold_keys.reverse();

    sort_quickchain_keys_v1(&mut hold_keys).unwrap();
    sort_quickchain_keys_v1(&mut reversed_hold_keys).unwrap();

    assert_eq!(hold_keys, reversed_hold_keys);

    let hold_hex: Vec<String> = hold_keys.iter().map(|key| lower_hex(key)).collect();

    assert_eq!(
        hold_hex.as_slice(),
        vector.expected_hold_order_hex.as_slice()
    );
}

#[test]
fn duplicate_sort_keys_reject_after_deterministic_sorting() {
    let vector = load_vector();

    let mut keys: Vec<Vec<u8>> = vector
        .duplicate_account_inputs
        .iter()
        .map(|input| quickchain_account_leaf_sort_key_v1(&input.account_id, &input.asset).unwrap())
        .collect();

    let error = sort_quickchain_keys_v1(&mut keys).unwrap_err();

    match error {
        QuickChainValidationError::InvalidField { field, reason } => {
            assert_eq!(field, "sort_keys");
            assert_eq!(reason, vector.expected_duplicate_error.as_str());
        }
        other => panic!("expected duplicate sort-key error, got {other:?}"),
    }
}

#[test]
fn sorted_unique_validator_rejects_descending_duplicate_and_empty_keys() {
    let alpha = quickchain_account_leaf_sort_key_v1("account:alpha", "roc").unwrap();
    let beta = quickchain_account_leaf_sort_key_v1("account:beta", "roc").unwrap();

    let descending = vec![beta.clone(), alpha.clone()];
    let error = validate_quickchain_sorted_unique_keys_v1(&descending).unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "sort_keys",
            reason: "sort keys must be strictly bytewise ascending"
        }
    ));

    let duplicate = vec![alpha.clone(), alpha];
    let error = validate_quickchain_sorted_unique_keys_v1(&duplicate).unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "sort_keys",
            reason: "duplicate sort keys are forbidden"
        }
    ));

    let empty_key = vec![Vec::new()];
    let error = validate_quickchain_sorted_unique_keys_v1(&empty_key).unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "sort_keys",
            reason: "sort keys must not be empty"
        }
    ));

    validate_quickchain_sorted_unique_keys_v1(&[]).unwrap();
}

#[test]
fn sort_key_inputs_reject_non_roc_asset_and_noncanonical_ids() {
    let error = quickchain_account_leaf_sort_key_v1("account:alpha", "rox").unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidField {
            field: "asset",
            reason: "must be roc for the Phase 0 account-leaf sort key"
        }
    ));

    quickchain_account_leaf_sort_key_v1("account:Alpha", "roc")
        .expect_err("uppercase account token must reject");

    quickchain_account_leaf_sort_key_v1("account:alpha\0evil", "roc")
        .expect_err("embedded NUL must reject");

    quickchain_hold_leaf_sort_key_v1("hold_ABCDEF0123456789ABCDEF0123456789")
        .expect_err("uppercase hold id must reject");

    quickchain_hold_leaf_sort_key_v1("hold_1234").expect_err("short hold id must reject");
}

#[test]
fn sort_key_vector_file_rejects_unknown_fields() {
    let mut value: Value = serde_json::from_str(SORT_KEY_VECTOR).unwrap();

    value
        .as_object_mut()
        .unwrap()
        .insert("database_order".to_string(), json!(true));

    serde_json::from_value::<SortKeyVectorSet>(value)
        .expect_err("unknown sort-key vector fields must reject");
}
