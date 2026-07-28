//! RO:WHAT — Native Passport device-bound capability contract.
//! RO:WHY — P3 Identity & Keys. Adds a contract-only review for future device-bound capability decisions after replay/challenge state evidence exists.
//! RO:INTERACTS — Phase 11B replay/challenge adapter envelopes and future capability state adapters.
//! RO:INVARIANTS — requires replay recorded, challenge consumed, device authority, request-proof binding, device binding, scope hash, capability binding hash, TTL, signer/verifier labels, replay key, idempotency key, transcript hash, and signed payload. This phase does not issue, consume, refresh, revoke, store, route, perform runtime I/O, touch wallet/ledger state, or access secrets.
//! RO:TEST — tests/native_passport_phase11c_device_bound_capability_contract.rs.

use super::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, NativePassportProofAuthority, NativePassportProofKind,
    NativePassportScope, NativeProofSignatureAlgorithm, NativeProofSignedPayloadHex,
    NativeProofSigningOperationKind, NativeReplayChallengeConsumptionAdapterEnvelopeV1,
    PassportIdV1, PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN,
    PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_VERSION,
    PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_PROOF_CONTRACT_DOMAIN,
    PHASE8B_PROOF_CONTRACT_VERSION, PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN,
    PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
};

pub const NATIVE_PASSPORT_PHASE11C_LABEL: &str =
    "NATIVE_PASSPORT_PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT";

pub const PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN: &str =
    "native-passport/device-bound-capability-contract/v1";

pub const PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_VERSION: u16 = 1;

pub const PHASE11C_MAX_CAPABILITY_TTL_MS: u64 = 3_600_000;

pub const PHASE11C_MAX_CLOCK_SKEW_MS: u64 = 30_000;

pub const PHASE11C_FORBIDDEN_CAPABILITY_CONTRACT_AUTHORITY_FLAGS: &[&str] = &[
    "capability_issuance_execution",
    "capability_consumption_execution",
    "capability_refresh_execution",
    "capability_revocation_execution",
    "capability_storage_mutation",
    "secret_key_access",
    "key_loading",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_unseal",
    "runtime_io",
    "route_added",
    "storage_mutation_inside_svc_passport_native",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceBoundCapabilityContractDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub operation_kind: NativeProofSigningOperationKind,
    pub expected_authority: NativePassportProofAuthority,
    pub challenge_id: ChallengeIdV1,
    pub passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
    pub proof_contract_domain: &'static str,
    pub proof_contract_version: u16,
    pub request_contract_domain: Option<&'static str>,
    pub request_contract_version: Option<u16>,
    pub proof_kind: NativePassportProofKind,
    pub transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub expected_signature_algorithm: NativeProofSignatureAlgorithm,
    pub signed_payload_hex: NativeProofSignedPayloadHex,
    pub verification_evidence_label: &'static str,
    pub verifier_public_key_label: &'static str,
    pub state_adapter_label: &'static str,
    pub replay_key_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub capability_id_hash: B3DigestHex,
    pub capability_binding_hash: B3DigestHex,
    pub capability_scope_hash: B3DigestHex,
    pub capability_scopes: Vec<NativePassportScope>,
    pub capability_issued_at_ms: u64,
    pub capability_expires_at_ms: u64,
    pub max_clock_skew_ms: u64,
    pub device_bound: bool,
    pub expects_replay_recorded_by_adapter: bool,
    pub expects_challenge_consumed_by_adapter: bool,
    pub contract_only: bool,
    pub requests_capability_issuance_execution: bool,
    pub requests_capability_consumption_execution: bool,
    pub requests_capability_lifecycle_runtime: bool,
    pub requests_capability_storage_mutation: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_loading: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_runtime_io: bool,
    pub adds_routes: bool,
    pub requests_storage_mutation_inside_native: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceBoundCapabilityContractDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub operation_kind: NativeProofSigningOperationKind,
    pub reviewed_authority: NativePassportProofAuthority,
    pub challenge_id: ChallengeIdV1,
    pub passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
    pub proof_contract_domain: &'static str,
    pub proof_contract_version: u16,
    pub request_contract_domain: Option<&'static str>,
    pub request_contract_version: Option<u16>,
    pub proof_kind: NativePassportProofKind,
    pub transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub expected_signature_algorithm: NativeProofSignatureAlgorithm,
    pub signed_payload_hex: NativeProofSignedPayloadHex,
    pub verification_evidence_label: &'static str,
    pub verifier_public_key_label: &'static str,
    pub state_adapter_label: &'static str,
    pub replay_key_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub capability_id_hash: B3DigestHex,
    pub capability_binding_hash: B3DigestHex,
    pub capability_scope_hash: B3DigestHex,
    pub capability_scopes: Vec<NativePassportScope>,
    pub capability_issued_at_ms: u64,
    pub capability_expires_at_ms: u64,
    pub device_bound: bool,
    pub replay_recorded_reviewed: bool,
    pub challenge_consumed_reviewed: bool,
    pub capability_issued: bool,
    pub capability_state_changed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDeviceBoundCapabilityContractReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    ConsumptionEnvelopeDomainMismatch,
    ConsumptionEnvelopeVersionMismatch,
    ConsumptionEnvelopeStateNotApplied,
    ConsumptionEnvelopeUnsafe,
    UnsupportedCapabilityOperation,
    AuthorityMismatch,
    ChallengeIdMismatch,
    PassportBindingMismatch,
    DeviceBindingMissing,
    DeviceBindingMismatch,
    ProofContractDomainMismatch,
    ProofContractVersionMismatch,
    RequestProofContractDomainMissing,
    RequestProofContractDomainMismatch,
    RequestProofContractVersionMismatch,
    ProofKindMismatch,
    TranscriptHashMismatch,
    MissingSignerPublicKeyLabel,
    SignerPublicKeyLabelMismatch,
    SignatureAlgorithmMismatch,
    SignedPayloadMismatch,
    MissingVerificationEvidenceLabel,
    VerificationEvidenceLabelMismatch,
    MissingVerifierPublicKeyLabel,
    VerifierPublicKeyLabelMismatch,
    MissingStateAdapterLabel,
    StateAdapterLabelMismatch,
    ReplayKeyMismatch,
    IdempotencyKeyMismatch,
    EmptyCapabilityScopes,
    DuplicateCapabilityScopes,
    CapabilityNotDeviceBound,
    InvalidCapabilityTimeWindow,
    CapabilityTtlTooLong,
    ClockSkewTooLarge,
    CapabilityContractNotContractOnly,
    UnsafeCapabilityAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceBoundCapabilityContractPosture {
    pub phase_label: &'static str,
    pub device_bound_capability_contract_added: bool,
    pub capability_scope_binding_added: bool,
    pub capability_id_binding_added: bool,
    pub capability_issuance_execution_added: bool,
    pub capability_consumption_execution_added: bool,
    pub capability_lifecycle_runtime_added: bool,
    pub capability_storage_mutation_added: bool,
    pub secret_key_access_added: bool,
    pub key_loading_added: bool,
    pub key_derivation_runtime_added: bool,
    pub vault_unlock_added: bool,
    pub vault_runtime_added: bool,
    pub platform_sealer_unseal_added: bool,
    pub runtime_io_added: bool,
    pub routes_added: bool,
    pub storage_mutation_inside_native_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_device_bound_capability_contract_posture(
) -> NativeDeviceBoundCapabilityContractPosture {
    NativeDeviceBoundCapabilityContractPosture {
        phase_label: NATIVE_PASSPORT_PHASE11C_LABEL,
        device_bound_capability_contract_added: true,
        capability_scope_binding_added: true,
        capability_id_binding_added: true,
        capability_issuance_execution_added: false,
        capability_consumption_execution_added: false,
        capability_lifecycle_runtime_added: false,
        capability_storage_mutation_added: false,
        secret_key_access_added: false,
        key_loading_added: false,
        key_derivation_runtime_added: false,
        vault_unlock_added: false,
        vault_runtime_added: false,
        platform_sealer_unseal_added: false,
        runtime_io_added: false,
        routes_added: false,
        storage_mutation_inside_native_added: false,
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE11C_FORBIDDEN_CAPABILITY_CONTRACT_AUTHORITY_FLAGS,
    }
}

pub fn review_native_device_bound_capability_contract(
    envelope: &NativeReplayChallengeConsumptionAdapterEnvelopeV1,
    draft: NativeDeviceBoundCapabilityContractDraftV1,
) -> Result<
    NativeDeviceBoundCapabilityContractDecisionV1,
    NativeDeviceBoundCapabilityContractReviewError,
> {
    validate_consumption_envelope(envelope)?;
    validate_draft_against_envelope(envelope, &draft)?;
    validate_no_unsafe_flags(&draft)?;

    Ok(NativeDeviceBoundCapabilityContractDecisionV1 {
        contract_domain: PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN,
        contract_version: PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_VERSION,
        operation_kind: draft.operation_kind,
        reviewed_authority: draft.expected_authority,
        challenge_id: draft.challenge_id,
        passport_id: draft.passport_id,
        device_id: draft.device_id,
        proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
        proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
        request_contract_domain: draft.request_contract_domain,
        request_contract_version: draft.request_contract_version,
        proof_kind: draft.proof_kind,
        transcript_hash: draft.transcript_hash,
        signer_public_key_label: draft.signer_public_key_label,
        expected_signature_algorithm: draft.expected_signature_algorithm,
        signed_payload_hex: draft.signed_payload_hex,
        verification_evidence_label: draft.verification_evidence_label,
        verifier_public_key_label: draft.verifier_public_key_label,
        state_adapter_label: draft.state_adapter_label,
        replay_key_hash: draft.replay_key_hash,
        idempotency_key_hash: draft.idempotency_key_hash,
        capability_id_hash: draft.capability_id_hash,
        capability_binding_hash: draft.capability_binding_hash,
        capability_scope_hash: draft.capability_scope_hash,
        capability_scopes: draft.capability_scopes,
        capability_issued_at_ms: draft.capability_issued_at_ms,
        capability_expires_at_ms: draft.capability_expires_at_ms,
        device_bound: true,
        replay_recorded_reviewed: true,
        challenge_consumed_reviewed: true,
        capability_issued: false,
        capability_state_changed: false,
        runtime_io_performed: false,
        wallet_or_ledger_mutated: false,
        contract_only: true,
    })
}

fn validate_consumption_envelope(
    envelope: &NativeReplayChallengeConsumptionAdapterEnvelopeV1,
) -> Result<(), NativeDeviceBoundCapabilityContractReviewError> {
    if envelope.contract_domain != PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN {
        return Err(
            NativeDeviceBoundCapabilityContractReviewError::ConsumptionEnvelopeDomainMismatch,
        );
    }

    if envelope.contract_version != PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_VERSION {
        return Err(
            NativeDeviceBoundCapabilityContractReviewError::ConsumptionEnvelopeVersionMismatch,
        );
    }

    if !envelope.replay_recorded_by_adapter || !envelope.challenge_consumed_by_adapter {
        return Err(
            NativeDeviceBoundCapabilityContractReviewError::ConsumptionEnvelopeStateNotApplied,
        );
    }

    if envelope.capability_state_changed
        || envelope.runtime_io_performed
        || envelope.wallet_or_ledger_mutated
    {
        return Err(NativeDeviceBoundCapabilityContractReviewError::ConsumptionEnvelopeUnsafe);
    }

    Ok(())
}

fn validate_draft_against_envelope(
    envelope: &NativeReplayChallengeConsumptionAdapterEnvelopeV1,
    draft: &NativeDeviceBoundCapabilityContractDraftV1,
) -> Result<(), NativeDeviceBoundCapabilityContractReviewError> {
    if draft.contract_domain != PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN {
        return Err(NativeDeviceBoundCapabilityContractReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_VERSION {
        return Err(NativeDeviceBoundCapabilityContractReviewError::ContractVersionMismatch);
    }

    if !draft.contract_only {
        return Err(
            NativeDeviceBoundCapabilityContractReviewError::CapabilityContractNotContractOnly,
        );
    }

    if draft.operation_kind != NativeProofSigningOperationKind::RequestProof {
        return Err(NativeDeviceBoundCapabilityContractReviewError::UnsupportedCapabilityOperation);
    }

    if draft.operation_kind != envelope.operation_kind {
        return Err(NativeDeviceBoundCapabilityContractReviewError::UnsupportedCapabilityOperation);
    }

    if draft.expected_authority != NativePassportProofAuthority::DeviceAuthority
        || draft.expected_authority != envelope.reviewed_authority
    {
        return Err(NativeDeviceBoundCapabilityContractReviewError::AuthorityMismatch);
    }

    if draft.challenge_id != envelope.challenge_id {
        return Err(NativeDeviceBoundCapabilityContractReviewError::ChallengeIdMismatch);
    }

    if draft.passport_id != envelope.passport_id {
        return Err(NativeDeviceBoundCapabilityContractReviewError::PassportBindingMismatch);
    }

    let draft_device_id = draft
        .device_id
        .as_ref()
        .ok_or(NativeDeviceBoundCapabilityContractReviewError::DeviceBindingMissing)?;
    let envelope_device_id = envelope
        .device_id
        .as_ref()
        .ok_or(NativeDeviceBoundCapabilityContractReviewError::DeviceBindingMissing)?;

    if draft_device_id != envelope_device_id {
        return Err(NativeDeviceBoundCapabilityContractReviewError::DeviceBindingMismatch);
    }

    if draft.proof_contract_domain != envelope.proof_contract_domain
        || draft.proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
    {
        return Err(NativeDeviceBoundCapabilityContractReviewError::ProofContractDomainMismatch);
    }

    if draft.proof_contract_version != envelope.proof_contract_version
        || draft.proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
    {
        return Err(NativeDeviceBoundCapabilityContractReviewError::ProofContractVersionMismatch);
    }

    let request_domain = draft
        .request_contract_domain
        .ok_or(NativeDeviceBoundCapabilityContractReviewError::RequestProofContractDomainMissing)?;

    if Some(request_domain) != envelope.request_contract_domain
        || request_domain != PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN
    {
        return Err(
            NativeDeviceBoundCapabilityContractReviewError::RequestProofContractDomainMismatch,
        );
    }

    if draft.request_contract_version != envelope.request_contract_version
        || draft.request_contract_version != Some(PHASE8C_PROOF_REQUEST_CONTRACT_VERSION)
    {
        return Err(
            NativeDeviceBoundCapabilityContractReviewError::RequestProofContractVersionMismatch,
        );
    }

    if draft.proof_kind != envelope.proof_kind {
        return Err(NativeDeviceBoundCapabilityContractReviewError::ProofKindMismatch);
    }

    if draft.transcript_hash != envelope.transcript_hash {
        return Err(NativeDeviceBoundCapabilityContractReviewError::TranscriptHashMismatch);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(NativeDeviceBoundCapabilityContractReviewError::MissingSignerPublicKeyLabel);
    }

    if draft.signer_public_key_label != envelope.signer_public_key_label {
        return Err(NativeDeviceBoundCapabilityContractReviewError::SignerPublicKeyLabelMismatch);
    }

    validate_signature_algorithm(draft.expected_authority, draft.expected_signature_algorithm)?;

    if draft.expected_signature_algorithm != envelope.expected_signature_algorithm {
        return Err(NativeDeviceBoundCapabilityContractReviewError::SignatureAlgorithmMismatch);
    }

    if draft.signed_payload_hex != envelope.signed_payload_hex {
        return Err(NativeDeviceBoundCapabilityContractReviewError::SignedPayloadMismatch);
    }

    if draft.verification_evidence_label.is_empty() {
        return Err(
            NativeDeviceBoundCapabilityContractReviewError::MissingVerificationEvidenceLabel,
        );
    }

    if draft.verification_evidence_label != envelope.verification_evidence_label {
        return Err(
            NativeDeviceBoundCapabilityContractReviewError::VerificationEvidenceLabelMismatch,
        );
    }

    if draft.verifier_public_key_label.is_empty() {
        return Err(NativeDeviceBoundCapabilityContractReviewError::MissingVerifierPublicKeyLabel);
    }

    if draft.verifier_public_key_label != envelope.verifier_public_key_label {
        return Err(NativeDeviceBoundCapabilityContractReviewError::VerifierPublicKeyLabelMismatch);
    }

    if draft.state_adapter_label.is_empty() {
        return Err(NativeDeviceBoundCapabilityContractReviewError::MissingStateAdapterLabel);
    }

    if draft.state_adapter_label != envelope.state_adapter_label {
        return Err(NativeDeviceBoundCapabilityContractReviewError::StateAdapterLabelMismatch);
    }

    if draft.replay_key_hash != envelope.replay_key_hash {
        return Err(NativeDeviceBoundCapabilityContractReviewError::ReplayKeyMismatch);
    }

    if draft.idempotency_key_hash != envelope.idempotency_key_hash {
        return Err(NativeDeviceBoundCapabilityContractReviewError::IdempotencyKeyMismatch);
    }

    if !draft.expects_replay_recorded_by_adapter || !draft.expects_challenge_consumed_by_adapter {
        return Err(
            NativeDeviceBoundCapabilityContractReviewError::ConsumptionEnvelopeStateNotApplied,
        );
    }

    if draft.capability_scopes.is_empty() {
        return Err(NativeDeviceBoundCapabilityContractReviewError::EmptyCapabilityScopes);
    }

    if has_duplicate_scopes(&draft.capability_scopes) {
        return Err(NativeDeviceBoundCapabilityContractReviewError::DuplicateCapabilityScopes);
    }

    if !draft.device_bound {
        return Err(NativeDeviceBoundCapabilityContractReviewError::CapabilityNotDeviceBound);
    }

    if draft.capability_issued_at_ms < envelope.consumption_requested_at_ms
        || draft.capability_expires_at_ms <= draft.capability_issued_at_ms
    {
        return Err(NativeDeviceBoundCapabilityContractReviewError::InvalidCapabilityTimeWindow);
    }

    if draft.capability_expires_at_ms - draft.capability_issued_at_ms
        > PHASE11C_MAX_CAPABILITY_TTL_MS
    {
        return Err(NativeDeviceBoundCapabilityContractReviewError::CapabilityTtlTooLong);
    }

    if draft.max_clock_skew_ms > PHASE11C_MAX_CLOCK_SKEW_MS {
        return Err(NativeDeviceBoundCapabilityContractReviewError::ClockSkewTooLarge);
    }

    Ok(())
}

fn validate_signature_algorithm(
    authority: NativePassportProofAuthority,
    algorithm: NativeProofSignatureAlgorithm,
) -> Result<(), NativeDeviceBoundCapabilityContractReviewError> {
    let expected = match authority {
        NativePassportProofAuthority::RootAuthority => PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
        NativePassportProofAuthority::DeviceAuthority => PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
    };

    if algorithm != expected {
        return Err(NativeDeviceBoundCapabilityContractReviewError::SignatureAlgorithmMismatch);
    }

    Ok(())
}

fn has_duplicate_scopes(scopes: &[NativePassportScope]) -> bool {
    for (index, scope) in scopes.iter().enumerate() {
        if scopes[index + 1..]
            .iter()
            .any(|candidate| candidate == scope)
        {
            return true;
        }
    }

    false
}

fn validate_no_unsafe_flags(
    draft: &NativeDeviceBoundCapabilityContractDraftV1,
) -> Result<(), NativeDeviceBoundCapabilityContractReviewError> {
    if draft.requests_capability_issuance_execution
        || draft.requests_capability_consumption_execution
        || draft.requests_capability_lifecycle_runtime
        || draft.requests_capability_storage_mutation
        || draft.requests_secret_key_access
        || draft.requests_key_loading
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation_inside_native
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeDeviceBoundCapabilityContractReviewError::UnsafeCapabilityAuthorityFlag);
    }

    Ok(())
}
