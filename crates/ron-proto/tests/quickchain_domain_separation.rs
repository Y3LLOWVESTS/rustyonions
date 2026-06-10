use std::collections::BTreeSet;

use ron_proto::{
    validate_all_domain_separators_v1, validate_domain_separator_v1,
    QUICKCHAIN_ACCOUNTING_SNAPSHOT_DOMAIN_V1, QUICKCHAIN_ACCOUNT_STATE_EMPTY_DOMAIN_V1,
    QUICKCHAIN_ACCOUNT_STATE_LEAF_DOMAIN_V1, QUICKCHAIN_ACCOUNT_STATE_NODE_DOMAIN_V1,
    QUICKCHAIN_ANCHOR_PAYLOAD_DOMAIN_V1, QUICKCHAIN_CHAIN_PARAMS_DOMAIN_V1,
    QUICKCHAIN_CHALLENGE_EVIDENCE_DOMAIN_V1, QUICKCHAIN_CHECKPOINT_HEADER_DOMAIN_V1,
    QUICKCHAIN_CHECKPOINT_SIGNATURE_DOMAIN_V1, QUICKCHAIN_DATA_AVAILABILITY_EMPTY_DOMAIN_V1,
    QUICKCHAIN_DATA_AVAILABILITY_LEAF_DOMAIN_V1, QUICKCHAIN_DATA_AVAILABILITY_NODE_DOMAIN_V1,
    QUICKCHAIN_DOMAIN_SEPARATORS_V1, QUICKCHAIN_RECEIPT_BATCH_HEADER_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_EMPTY_DOMAIN_V1, QUICKCHAIN_RECEIPT_LEAF_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_NODE_DOMAIN_V1, QUICKCHAIN_REWARD_MANIFEST_DOMAIN_V1,
    QUICKCHAIN_VALIDATOR_SET_DOMAIN_V1,
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
fn domain_separators_have_expected_exact_values() {
    assert_eq!(
        QUICKCHAIN_ACCOUNT_STATE_LEAF_DOMAIN_V1,
        "quickchain.account_state.leaf.v1"
    );
    assert_eq!(
        QUICKCHAIN_ACCOUNT_STATE_NODE_DOMAIN_V1,
        "quickchain.account_state.node.v1"
    );
    assert_eq!(
        QUICKCHAIN_ACCOUNT_STATE_EMPTY_DOMAIN_V1,
        "quickchain.account_state.empty.v1"
    );
    assert_eq!(
        QUICKCHAIN_RECEIPT_LEAF_DOMAIN_V1,
        "quickchain.receipt.leaf.v1"
    );
    assert_eq!(
        QUICKCHAIN_RECEIPT_NODE_DOMAIN_V1,
        "quickchain.receipt.node.v1"
    );
    assert_eq!(
        QUICKCHAIN_RECEIPT_EMPTY_DOMAIN_V1,
        "quickchain.receipt.empty.v1"
    );
    assert_eq!(
        QUICKCHAIN_RECEIPT_BATCH_HEADER_DOMAIN_V1,
        "quickchain.receipt_batch.header.v1"
    );
    assert_eq!(
        QUICKCHAIN_CHECKPOINT_HEADER_DOMAIN_V1,
        "quickchain.checkpoint.header.v1"
    );
    assert_eq!(
        QUICKCHAIN_CHECKPOINT_SIGNATURE_DOMAIN_V1,
        "quickchain.checkpoint.signature.v1"
    );
    assert_eq!(
        QUICKCHAIN_VALIDATOR_SET_DOMAIN_V1,
        "quickchain.validator_set.v1"
    );
    assert_eq!(
        QUICKCHAIN_CHAIN_PARAMS_DOMAIN_V1,
        "quickchain.chain_params.v1"
    );
    assert_eq!(
        QUICKCHAIN_CHALLENGE_EVIDENCE_DOMAIN_V1,
        "quickchain.challenge.evidence.v1"
    );
    assert_eq!(
        QUICKCHAIN_DATA_AVAILABILITY_LEAF_DOMAIN_V1,
        "quickchain.data_availability.leaf.v1"
    );
    assert_eq!(
        QUICKCHAIN_DATA_AVAILABILITY_NODE_DOMAIN_V1,
        "quickchain.data_availability.node.v1"
    );
    assert_eq!(
        QUICKCHAIN_DATA_AVAILABILITY_EMPTY_DOMAIN_V1,
        "quickchain.data_availability.empty.v1"
    );
    assert_eq!(
        QUICKCHAIN_ACCOUNTING_SNAPSHOT_DOMAIN_V1,
        "quickchain.accounting_snapshot.v1"
    );
    assert_eq!(
        QUICKCHAIN_REWARD_MANIFEST_DOMAIN_V1,
        "quickchain.reward_manifest.v1"
    );
    assert_eq!(
        QUICKCHAIN_ANCHOR_PAYLOAD_DOMAIN_V1,
        "quickchain.anchor_payload.v1"
    );
}

#[test]
fn domain_separator_rejects_wrong_prefix() {
    let err = validate_domain_separator_v1("ledger.account_state.leaf.v1").unwrap_err();

    assert!(err.to_string().contains("must start with quickchain."));
}

#[test]
fn domain_separator_rejects_wrong_version_suffix() {
    let err = validate_domain_separator_v1("quickchain.account_state.leaf.v2").unwrap_err();

    assert!(err.to_string().contains("must end with .v1"));
}

#[test]
fn domain_separator_rejects_uppercase_or_spaces() {
    assert!(validate_domain_separator_v1("quickchain.AccountState.leaf.v1").is_err());
    assert!(validate_domain_separator_v1("quickchain.account state.leaf.v1").is_err());
}

#[test]
fn domain_separator_rejects_empty_segments() {
    let err = validate_domain_separator_v1("quickchain.account_state..leaf.v1").unwrap_err();

    assert!(err.to_string().contains("empty segments"));
}
