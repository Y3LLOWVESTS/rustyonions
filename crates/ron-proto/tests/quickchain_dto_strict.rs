use ron_proto::{
    ContentId, QuickChainAccountStateV1, QuickChainCanonicalEncodingV1, QuickChainChainParamsV1,
    QuickChainCheckpointHeaderV1, QuickChainReceiptRootSchemeV1, QuickChainStateRootSchemeV1,
    QuickChainValidationError, QuickChainValidatorSignatureV1, SignatureAlg,
    QUICKCHAIN_ACCOUNT_STATE_SCHEMA, QUICKCHAIN_CHAIN_PARAMS_SCHEMA,
    QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_VALIDATOR_SIGNATURE_SCHEMA,
};
use serde_json::json;

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn valid_signature() -> QuickChainValidatorSignatureV1 {
    QuickChainValidatorSignatureV1 {
        schema: QUICKCHAIN_VALIDATOR_SIGNATURE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        validator_id: "validator-01".to_string(),
        key_id: "key-01".to_string(),
        algorithm: SignatureAlg::Ed25519,
        checkpoint_hash: cid('f'),
        signature_wire: "dev-signature-wire-placeholder".to_string(),
    }
}

fn valid_checkpoint() -> QuickChainCheckpointHeaderV1 {
    QuickChainCheckpointHeaderV1 {
        schema: QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        height: 1,
        epoch_id: "roc-dev:height:000000000001".to_string(),
        previous_checkpoint_hash: cid('0'),
        previous_state_root: cid('1'),
        new_state_root: cid('2'),
        receipt_root: cid('3'),
        accounting_snapshot_root: cid('4'),
        reward_manifest_root: cid('5'),
        data_availability_root: cid('6'),
        policy_hash: cid('7'),
        validator_set_hash: cid('8'),
        chain_params_hash: cid('9'),
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        state_root_scheme: QuickChainStateRootSchemeV1::SortedMerkleMapV1,
        receipt_root_scheme: QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,
        supply_delta_minor_units: "0".to_string(),
        started_at_ms: 1_800_000_000_000,
        ended_at_ms: 1_800_000_060_000,
        produced_at_ms: 1_800_000_061_000,
        signatures: vec![valid_signature()],
    }
}

fn valid_account_state() -> QuickChainAccountStateV1 {
    QuickChainAccountStateV1 {
        schema: QUICKCHAIN_ACCOUNT_STATE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        account_id: "account:creator-a".to_string(),
        available_minor_units: "1000".to_string(),
        held_minor_units: "0".to_string(),
        nonce: 7,
        last_ledger_seq: 42,
    }
}

fn valid_phase0_params() -> QuickChainChainParamsV1 {
    QuickChainChainParamsV1 {
        schema: QUICKCHAIN_CHAIN_PARAMS_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        enabled: false,
        epoch_duration_ms: 300_000,
        checkpoint_cadence: 1,
        challenge_window_ms: 86_400_000,
        max_receipts_per_batch: 100_000,
        max_accounting_snapshot_bytes: 16 * 1024 * 1024,
        max_reward_manifest_bytes: 16 * 1024 * 1024,
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        state_root_scheme: QuickChainStateRootSchemeV1::SortedMerkleMapV1,
        receipt_root_scheme: QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,
        quorum_bps: 6700,
        min_validators: 4,
        max_validators: 21,
        passport_required: true,
        bond_required: false,
        rox_anchor_enabled: false,
    }
}

#[test]
fn checkpoint_header_validates_and_roundtrips_json() {
    let checkpoint = valid_checkpoint();

    checkpoint.validate().unwrap();

    let json = serde_json::to_string(&checkpoint).unwrap();
    let decoded: QuickChainCheckpointHeaderV1 = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, checkpoint);
    decoded.validate().unwrap();
}

#[test]
fn account_state_rejects_unknown_fields() {
    let value = json!({
        "schema": QUICKCHAIN_ACCOUNT_STATE_SCHEMA,
        "version": QUICKCHAIN_DTO_VERSION,
        "chain_id": "roc-dev",
        "account_id": "account:creator-a",
        "available_minor_units": "1000",
        "held_minor_units": "0",
        "nonce": 7,
        "last_ledger_seq": 42,
        "unexpected": true
    });

    let error = serde_json::from_value::<QuickChainAccountStateV1>(value).unwrap_err();

    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn checkpoint_rejects_uppercase_b3_hash() {
    let mut value = serde_json::to_value(valid_checkpoint()).unwrap();
    value["previous_checkpoint_hash"] =
        json!("b3:ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD");

    let error = serde_json::from_value::<QuickChainCheckpointHeaderV1>(value).unwrap_err();

    assert!(error.to_string().contains("hex must be lowercase"));
}

#[test]
fn account_state_rejects_float_money_wire_value() {
    let value = json!({
        "schema": QUICKCHAIN_ACCOUNT_STATE_SCHEMA,
        "version": QUICKCHAIN_DTO_VERSION,
        "chain_id": "roc-dev",
        "account_id": "account:creator-a",
        "available_minor_units": 1.5,
        "held_minor_units": "0",
        "nonce": 7,
        "last_ledger_seq": 42
    });

    assert!(serde_json::from_value::<QuickChainAccountStateV1>(value).is_err());
}

#[test]
fn account_state_rejects_noncanonical_money_string() {
    let mut account = valid_account_state();
    account.available_minor_units = "001".to_string();

    let error = account.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidMoney {
            field: "available_minor_units",
            ..
        }
    ));
}

#[test]
fn legacy_quickchain_dtos_use_shared_minor_unit_width_limit() {
    let mut account = valid_account_state();
    account.available_minor_units = "1".repeat(40);

    let error = account.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidMoney {
            field: "available_minor_units",
            reason: "must not exceed u128 decimal width",
        }
    ));

    let mut checkpoint = valid_checkpoint();
    checkpoint.supply_delta_minor_units = "1".repeat(40);

    let error = checkpoint.validate().unwrap_err();

    assert!(matches!(
        error,
        QuickChainValidationError::InvalidMoney {
            field: "supply_delta_minor_units",
            reason: "must not exceed u128 decimal width",
        }
    ));
}

#[test]
fn phase0_params_require_quickchain_and_rox_anchor_disabled() {
    let params = valid_phase0_params();

    params.validate_phase0_disabled().unwrap();

    let mut enabled = params.clone();
    enabled.enabled = true;
    assert!(enabled.validate_phase0_disabled().is_err());

    let mut anchored = params;
    anchored.rox_anchor_enabled = true;
    assert!(anchored.validate_phase0_disabled().is_err());
}

#[test]
fn validator_signature_validates_but_does_not_claim_crypto_verification() {
    let signature = valid_signature();

    signature.validate().unwrap();

    let json = serde_json::to_string(&signature).unwrap();
    let decoded: QuickChainValidatorSignatureV1 = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, signature);
}
