#![cfg(feature = "quickchain-preflight")]
//! RO:WHAT — Phase 3 Round 1 tests for read-only passport-gated validator attestation authorization.
//! RO:WHY — ECON/GOV: ron-ledger can evaluate validator eligibility without giving validators balance, staking, or settlement authority.
//! RO:INTERACTS — ron-ledger passport_gate and ron-proto Phase 3 validator DTOs.
//! RO:INVARIANTS — deterministic input time only; unauthorized/revoked/expired reject; no wallet mutation; no staking/slashing.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — fixture signatures are inert and grant no spend, bridge, paid unlock, or finality authority.
//! RO:TEST — this file.

use ron_ledger::quickchain::evaluate_passport_gated_attestation_authorization;
use ron_proto::{
    quantum::SignatureAlg,
    quickchain::{
        QuickChainValidatorAuthorizationRejectionCodeV1, QuickChainValidatorAuthorizationRequestV1,
        QuickChainValidatorAuthorizationStatusV1, QuickChainValidatorCapabilityV1,
        QuickChainValidatorIdentityV1, QuickChainValidatorLifecycleStatusV1,
        QuickChainValidatorSetV1, QuickChainVerifierAttestationV1,
        QuickChainVerifierReplayStatusV1, QUICKCHAIN_COMMITTEE_ATTESTATION_SIGNED_PAYLOAD_SCHEMA,
        QUICKCHAIN_DTO_VERSION, QUICKCHAIN_VALIDATOR_AUTHORIZATION_REQUEST_SCHEMA,
        QUICKCHAIN_VALIDATOR_CAPABILITY_SCHEMA, QUICKCHAIN_VALIDATOR_CAPABILITY_SCOPE_VERIFY_V1,
        QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA,
        QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1, QUICKCHAIN_VALIDATOR_SET_SCHEMA,
        QUICKCHAIN_VERIFIER_ATTESTATION_SCHEMA,
    },
    ContentId,
};

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

fn validator_set(status: QuickChainValidatorLifecycleStatusV1) -> QuickChainValidatorSetV1 {
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
        members: vec![identity(status)],
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

fn request(
    status: QuickChainValidatorLifecycleStatusV1,
) -> QuickChainValidatorAuthorizationRequestV1 {
    let set = validator_set(status);

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
fn active_passport_registry_validator_authorizes_attestation_read_only() {
    let request = request(QuickChainValidatorLifecycleStatusV1::Active);

    let result = evaluate_passport_gated_attestation_authorization(&request).unwrap();

    assert_eq!(
        result.status,
        QuickChainValidatorAuthorizationStatusV1::Authorized
    );
    assert_eq!(result.rejection_code, None);
    assert_eq!(result.validator_id, VALIDATOR_ID);
    assert_eq!(result.passport_subject, PASSPORT_SUBJECT);
    result.validate().unwrap();
}

#[test]
fn unauthorized_identity_rejects() {
    let mut request = request(QuickChainValidatorLifecycleStatusV1::Active);
    request.capability.validator_id = "validator-outsider".to_string();
    request.attestation.committee_member_id = "validator-outsider".to_string();

    let result = evaluate_passport_gated_attestation_authorization(&request).unwrap();

    assert_eq!(
        result.status,
        QuickChainValidatorAuthorizationStatusV1::Rejected
    );
    assert_eq!(
        result.rejection_code,
        Some(QuickChainValidatorAuthorizationRejectionCodeV1::UnauthorizedIdentity)
    );
}

#[test]
fn revoked_identity_rejects() {
    let request = request(QuickChainValidatorLifecycleStatusV1::Revoked);

    let result = evaluate_passport_gated_attestation_authorization(&request).unwrap();

    assert_eq!(
        result.status,
        QuickChainValidatorAuthorizationStatusV1::Rejected
    );
    assert_eq!(
        result.rejection_code,
        Some(QuickChainValidatorAuthorizationRejectionCodeV1::RevokedValidator)
    );
}

#[test]
fn revoked_capability_rejects() {
    let mut request = request(QuickChainValidatorLifecycleStatusV1::Active);
    request.capability.revoked_at_ms = Some(EVALUATION_TIME_MS - 1);

    let result = evaluate_passport_gated_attestation_authorization(&request).unwrap();

    assert_eq!(
        result.status,
        QuickChainValidatorAuthorizationStatusV1::Rejected
    );
    assert_eq!(
        result.rejection_code,
        Some(QuickChainValidatorAuthorizationRejectionCodeV1::RevokedValidator)
    );
}

#[test]
fn expired_capability_rejects() {
    let mut request = request(QuickChainValidatorLifecycleStatusV1::Active);
    request.evaluation_time_ms = EXPIRES_AT_MS;

    let result = evaluate_passport_gated_attestation_authorization(&request).unwrap();

    assert_eq!(
        result.status,
        QuickChainValidatorAuthorizationStatusV1::Rejected
    );
    assert_eq!(
        result.rejection_code,
        Some(QuickChainValidatorAuthorizationRejectionCodeV1::ExpiredCapability)
    );
}

#[test]
fn bonded_economics_are_rejected_before_authorization() {
    let mut request = request(QuickChainValidatorLifecycleStatusV1::Active);
    request.validator_set.bond_required = true;

    let error = evaluate_passport_gated_attestation_authorization(&request).unwrap_err();

    match error {
        ron_proto::QuickChainValidationError::InvalidField { field, .. } => {
            assert_eq!(field, "bond_required");
        }
        other => panic!("expected bond_required InvalidField, got {other:?}"),
    }
}

#[test]
fn key_mismatch_rejects_without_mutating_ledger_truth() {
    let mut request = request(QuickChainValidatorLifecycleStatusV1::Active);
    request.attestation.key_id = "key:validator-alpha:rotated".to_string();

    let result = evaluate_passport_gated_attestation_authorization(&request).unwrap();

    assert_eq!(
        result.status,
        QuickChainValidatorAuthorizationStatusV1::Rejected
    );
    assert_eq!(
        result.rejection_code,
        Some(QuickChainValidatorAuthorizationRejectionCodeV1::KeyMismatch)
    );
}
