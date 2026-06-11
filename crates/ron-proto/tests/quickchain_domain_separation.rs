use std::collections::BTreeSet;

use ron_proto::{
    validate_all_domain_separators_v1, validate_domain_separator_v1,
    QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN_V1, QUICKCHAIN_ACCOUNTING_WINDOW_DOMAIN_V1,
    QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1, QUICKCHAIN_ANCHOR_PAYLOAD_DOMAIN_V1,
    QUICKCHAIN_CHAIN_PARAMS_HASH_DOMAIN_V1, QUICKCHAIN_CHALLENGE_EVIDENCE_DOMAIN_V1,
    QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1, QUICKCHAIN_DATA_AVAILABILITY_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_DOMAIN_SEPARATORS_V1, QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
    QUICKCHAIN_HOLD_ROOT_HASH_DOMAIN_V1, QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    QUICKCHAIN_POLICY_HASH_DOMAIN_V1, QUICKCHAIN_RECEIPT_BATCH_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1, QUICKCHAIN_RECEIPT_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_REWARD_MANIFEST_DOMAIN_V1, QUICKCHAIN_REWARD_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1, QUICKCHAIN_VALIDATOR_SET_HASH_DOMAIN_V1,
};

#[test]
fn all_builtin_domain_separators_validate() {
    validate_all_domain_separators_v1().unwrap();

    for separator in QUICKCHAIN_DOMAIN_SEPARATORS_V1 {
        validate_domain_separator_v1(separator).unwrap();
    }
}

#[test]
fn domain_separators_are_unique() {
    let unique = QUICKCHAIN_DOMAIN_SEPARATORS_V1
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    assert_eq!(unique.len(), QUICKCHAIN_DOMAIN_SEPARATORS_V1.len());
}

#[test]
fn blueprint_required_domain_separators_have_exact_values() {
    assert_eq!(QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1, "quickchain.receipt.v1");
    assert_eq!(
        QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1,
        "quickchain.account-state.v1"
    );
    assert_eq!(
        QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
        "quickchain.hold-state.v1"
    );
    assert_eq!(
        QUICKCHAIN_HOLD_ROOT_HASH_DOMAIN_V1,
        "quickchain.hold-root.v1"
    );
    assert_eq!(
        QUICKCHAIN_RECEIPT_ROOT_HASH_DOMAIN_V1,
        "quickchain.receipt-root.v1"
    );
    assert_eq!(
        QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
        "quickchain.state-root.v1"
    );
    assert_eq!(
        QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN_V1,
        "quickchain.accounting-root.v1"
    );
    assert_eq!(
        QUICKCHAIN_REWARD_ROOT_HASH_DOMAIN_V1,
        "quickchain.reward-root.v1"
    );
    assert_eq!(
        QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
        "quickchain.checkpoint.v1"
    );
}

#[test]
fn reserved_preflight_domain_separators_have_exact_values() {
    assert_eq!(
        QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
        "quickchain.operation-intent.v1"
    );
    assert_eq!(
        QUICKCHAIN_CHAIN_PARAMS_HASH_DOMAIN_V1,
        "quickchain.chain-params.v1"
    );
    assert_eq!(
        QUICKCHAIN_VALIDATOR_SET_HASH_DOMAIN_V1,
        "quickchain.validator-set.v1"
    );
    assert_eq!(QUICKCHAIN_POLICY_HASH_DOMAIN_V1, "quickchain.policy.v1");
    assert_eq!(
        QUICKCHAIN_DATA_AVAILABILITY_ROOT_HASH_DOMAIN_V1,
        "quickchain.data-availability-root.v1"
    );
    assert_eq!(
        QUICKCHAIN_RECEIPT_BATCH_DOMAIN_V1,
        "quickchain.receipt-batch.v1"
    );
    assert_eq!(
        QUICKCHAIN_ACCOUNTING_WINDOW_DOMAIN_V1,
        "quickchain.accounting-window.v1"
    );
    assert_eq!(
        QUICKCHAIN_REWARD_MANIFEST_DOMAIN_V1,
        "quickchain.reward-manifest.v1"
    );
    assert_eq!(
        QUICKCHAIN_CHALLENGE_EVIDENCE_DOMAIN_V1,
        "quickchain.challenge-evidence.v1"
    );
    assert_eq!(
        QUICKCHAIN_ANCHOR_PAYLOAD_DOMAIN_V1,
        "quickchain.anchor-payload.v1"
    );
}

#[test]
fn domain_separator_rejects_wrong_prefix() {
    let err = validate_domain_separator_v1("ledger.account-state.v1").unwrap_err();

    assert!(err.to_string().contains("must start with quickchain."));
}

#[test]
fn domain_separator_rejects_wrong_version_suffix() {
    let err = validate_domain_separator_v1("quickchain.account-state.v2").unwrap_err();

    assert!(err.to_string().contains("must end with .v1"));
}

#[test]
fn domain_separator_rejects_uppercase_or_spaces() {
    assert!(validate_domain_separator_v1("quickchain.AccountState.v1").is_err());
    assert!(validate_domain_separator_v1("quickchain.account state.v1").is_err());
}

#[test]
fn domain_separator_rejects_empty_segments() {
    let err = validate_domain_separator_v1("quickchain.account-state..leaf.v1").unwrap_err();

    assert!(err.to_string().contains("empty segments"));
}

#[test]
fn domain_separator_allows_hyphenated_blueprint_names() {
    validate_domain_separator_v1("quickchain.account-state.v1").unwrap();
    validate_domain_separator_v1("quickchain.operation-intent.v1").unwrap();
    validate_domain_separator_v1("quickchain.hold-root.v1").unwrap();
    validate_domain_separator_v1("quickchain.receipt-root.v1").unwrap();
    validate_domain_separator_v1("quickchain.data-availability-root.v1").unwrap();
}
