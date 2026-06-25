//! RO:WHAT — Phase 3 Round 1 DTO tests for passport-gated validator set and capability authorization shapes.
//! RO:WHY — ECON/GOV: validators become passport/registry-gated without staking, slashing, public bridge, or settlement authority.
//! RO:INTERACTS — ron-proto quickchain validator_set DTOs and Phase 2 verifier attestations.
//! RO:INVARIANTS — unknown fields reject; passports required; bonds forbidden; revoked/expired/unauthorized identities are expressible.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — fixture hashes/signatures are inert test artifacts and grant no spend or settlement authority.
//! RO:TEST — this file.

use ron_proto::{
    quantum::SignatureAlg,
    quickchain::{
        QuickChainValidatorAuthorizationRejectionCodeV1, QuickChainValidatorAuthorizationRequestV1,
        QuickChainValidatorAuthorizationResultV1, QuickChainValidatorAuthorizationStatusV1,
        QuickChainValidatorCapabilityV1, QuickChainValidatorIdentityV1,
        QuickChainValidatorLifecycleStatusV1, QuickChainValidatorSetV1,
        QuickChainVerifierAttestationV1, QuickChainVerifierReplayStatusV1,
        QUICKCHAIN_COMMITTEE_ATTESTATION_SIGNED_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
        QUICKCHAIN_VALIDATOR_AUTHORIZATION_REQUEST_SCHEMA,
        QUICKCHAIN_VALIDATOR_AUTHORIZATION_RESULT_SCHEMA, QUICKCHAIN_VALIDATOR_CAPABILITY_SCHEMA,
        QUICKCHAIN_VALIDATOR_CAPABILITY_SCOPE_VERIFY_V1, QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA,
        QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1, QUICKCHAIN_VALIDATOR_SET_SCHEMA,
        QUICKCHAIN_VERIFIER_ATTESTATION_SCHEMA,
    },
    ContentId,
};
use serde_json::json;

const CHAIN_ID: &str = "ron-devnet";
const EPOCH_ID: &str = "epoch_0004";
const VALIDATOR_ID: &str = "validator-alpha";
const PASSPORT_SUBJECT: &str = "@validator-alpha";
const REGISTRY_ENTRY_ID: &str = "registry:validator-alpha";
const KEY_ID: &str = "key:validator-alpha:001";
const CAPABILITY_ID: &str = "cap:validator-alpha:verify:001";
const NOT_BEFORE_MS: u64 = 1_800_000_000_000;
const EXPIRES_AT_MS: u64 = 1_800_086_400_000;
const EVALUATION_TIME_MS: u64 = 1_800_000_060_000;

fn cid(label: &str) -> ContentId {
    let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

    format!("b3:{digest}")
        .parse()
        .expect("fixture content ID should parse")
}

fn identity(status: QuickChainValidatorLifecycleStatusV1) -> QuickChainValidatorIdentityV1 {
    QuickChainValidatorIdentityV1 {
        schema: QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_id: VALIDATOR_ID.to_string(),
        passport_subject: PASSPORT_SUBJECT.to_string(),
        registry_entry_id: REGISTRY_ENTRY_ID.to_string(),
        key_id: KEY_ID.to_string(),
        capability_id: CAPABILITY_ID.to_string(),
        signature_algorithm: SignatureAlg::Ed25519,
        lifecycle_status: status,
        not_before_ms: NOT_BEFORE_MS,
        expires_at_ms: EXPIRES_AT_MS,
    }
}

fn validator_set(members: Vec<QuickChainValidatorIdentityV1>) -> QuickChainValidatorSetV1 {
    QuickChainValidatorSetV1 {
        schema: QUICKCHAIN_VALIDATOR_SET_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: cid("validator-set"),
        policy_hash: cid("policy"),
        registry_snapshot_hash: cid("registry"),
        passport_required: true,
        bond_required: false,
        validator_set_algorithm: QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1
            .to_string(),
        members,
    }
}

fn capability() -> QuickChainValidatorCapabilityV1 {
    QuickChainValidatorCapabilityV1 {
        schema: QUICKCHAIN_VALIDATOR_CAPABILITY_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_id: VALIDATOR_ID.to_string(),
        passport_subject: PASSPORT_SUBJECT.to_string(),
        registry_entry_id: REGISTRY_ENTRY_ID.to_string(),
        key_id: KEY_ID.to_string(),
        capability_id: CAPABILITY_ID.to_string(),
        capability_scope: QUICKCHAIN_VALIDATOR_CAPABILITY_SCOPE_VERIFY_V1.to_string(),
        signature_algorithm: SignatureAlg::Ed25519,
        issued_at_ms: NOT_BEFORE_MS,
        not_before_ms: NOT_BEFORE_MS,
        expires_at_ms: EXPIRES_AT_MS,
        revoked_at_ms: None,
    }
}

fn attestation() -> QuickChainVerifierAttestationV1 {
    QuickChainVerifierAttestationV1 {
        schema: QUICKCHAIN_VERIFIER_ATTESTATION_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        committee_member_id: VALIDATOR_ID.to_string(),
        key_id: KEY_ID.to_string(),
        signature_algorithm: SignatureAlg::Ed25519,
        signed_payload_schema: QUICKCHAIN_COMMITTEE_ATTESTATION_SIGNED_PAYLOAD_SCHEMA.to_string(),
        replay_result_hash: cid("replay-result"),
        replay_status: QuickChainVerifierReplayStatusV1::Verified,
        signature_wire: "shape-only-signature-wire".to_string(),
    }
}

fn request() -> QuickChainValidatorAuthorizationRequestV1 {
    let set = validator_set(vec![identity(QuickChainValidatorLifecycleStatusV1::Active)]);

    QuickChainValidatorAuthorizationRequestV1 {
        schema: QUICKCHAIN_VALIDATOR_AUTHORIZATION_REQUEST_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: set.validator_set_hash.clone(),
        replay_result_hash: cid("replay-result"),
        replay_status: QuickChainVerifierReplayStatusV1::Verified,
        evaluation_time_ms: EVALUATION_TIME_MS,
        validator_set: set,
        attestation: attestation(),
        capability: capability(),
    }
}

#[test]
fn phase3_validator_set_validates_passport_required_bond_forbidden_shape() {
    let set = validator_set(vec![identity(QuickChainValidatorLifecycleStatusV1::Active)]);

    set.validate().unwrap();
}

#[test]
fn phase3_validator_set_rejects_missing_passport_requirement() {
    let mut set = validator_set(vec![identity(QuickChainValidatorLifecycleStatusV1::Active)]);
    set.passport_required = false;

    let error = set.validate().unwrap_err();

    match error {
        ron_proto::QuickChainValidationError::InvalidField { field, .. } => {
            assert_eq!(field, "passport_required");
        }
        other => panic!("expected passport_required InvalidField, got {other:?}"),
    }
}

#[test]
fn phase3_validator_set_rejects_bonded_economics_in_round1() {
    let mut set = validator_set(vec![identity(QuickChainValidatorLifecycleStatusV1::Active)]);
    set.bond_required = true;

    let error = set.validate().unwrap_err();

    match error {
        ron_proto::QuickChainValidationError::InvalidField { field, .. } => {
            assert_eq!(field, "bond_required");
        }
        other => panic!("expected bond_required InvalidField, got {other:?}"),
    }
}

#[test]
fn phase3_validator_set_rejects_duplicate_passport_subjects() {
    let mut second = identity(QuickChainValidatorLifecycleStatusV1::Active);
    second.validator_id = "validator-beta".to_string();
    second.key_id = "key:validator-beta:001".to_string();
    second.capability_id = "cap:validator-beta:verify:001".to_string();

    let set = validator_set(vec![
        identity(QuickChainValidatorLifecycleStatusV1::Active),
        second,
    ]);

    let error = set.validate().unwrap_err();

    match error {
        ron_proto::QuickChainValidationError::InvalidField { field, .. } => {
            assert_eq!(field, "members.passport_subject");
        }
        other => panic!("expected duplicate passport InvalidField, got {other:?}"),
    }
}

#[test]
fn phase3_authorization_request_binds_set_attestation_and_capability() {
    let auth = request();

    auth.validate().unwrap();

    let json = serde_json::to_string(&auth).unwrap();
    let decoded: QuickChainValidatorAuthorizationRequestV1 = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, auth);
    decoded.validate().unwrap();
}

#[test]
fn phase3_authorization_request_rejects_wrong_validator_set_hash() {
    let mut auth = request();
    auth.validator_set_hash = cid("different-validator-set");

    let error = auth.validate().unwrap_err();

    match error {
        ron_proto::QuickChainValidationError::InvalidField { field, .. } => {
            assert_eq!(field, "validator_set_hash");
        }
        other => panic!("expected validator_set_hash InvalidField, got {other:?}"),
    }
}

#[test]
fn phase3_authorization_result_status_consistency_is_strict() {
    let authorized = QuickChainValidatorAuthorizationResultV1 {
        schema: QUICKCHAIN_VALIDATOR_AUTHORIZATION_RESULT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        epoch_id: EPOCH_ID.to_string(),
        validator_set_hash: cid("validator-set"),
        validator_id: VALIDATOR_ID.to_string(),
        passport_subject: PASSPORT_SUBJECT.to_string(),
        key_id: KEY_ID.to_string(),
        replay_result_hash: cid("replay-result"),
        replay_status: QuickChainVerifierReplayStatusV1::Verified,
        status: QuickChainValidatorAuthorizationStatusV1::Authorized,
        rejection_code: None,
    };

    authorized.validate().unwrap();

    let mut rejected = authorized.clone();
    rejected.status = QuickChainValidatorAuthorizationStatusV1::Rejected;
    rejected.rejection_code =
        Some(QuickChainValidatorAuthorizationRejectionCodeV1::UnauthorizedIdentity);

    rejected.validate().unwrap();

    let mut invalid = authorized;
    invalid.rejection_code =
        Some(QuickChainValidatorAuthorizationRejectionCodeV1::UnauthorizedIdentity);

    assert!(invalid.validate().is_err());
}

#[test]
fn phase3_validator_set_rejects_unknown_fields() {
    let set = validator_set(vec![identity(QuickChainValidatorLifecycleStatusV1::Active)]);
    let mut value = serde_json::to_value(set).unwrap();

    value
        .as_object_mut()
        .unwrap()
        .insert("staking_power".to_string(), json!("1000000"));

    let decoded = serde_json::from_value::<QuickChainValidatorSetV1>(value);

    assert!(
        decoded.is_err(),
        "validator set must reject unknown staking/economic fields"
    );
}
