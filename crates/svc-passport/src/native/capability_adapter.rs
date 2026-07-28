//! RO:WHAT — Native Passport injected device-bound capability adapter seam.
//! RO:WHY — P3 Identity & Keys. Allows a future bounded state adapter to acknowledge device-bound capability evidence only after Phase 11C accepted the exact contract decision.
//! RO:INTERACTS — Phase 11C device-bound capability contract decisions and future durable capability state adapters.
//! RO:INVARIANTS — validates the Phase 11C decision, device binding, request-proof binding, capability id/binding/scope hashes, scope list, signer/verifier/state labels, replay/idempotency keys, signed payload, and timing before calling an injected adapter. svc-passport native still does not own durable storage, routes, runtime I/O, wallet/ledger mutation, or secret/key material.
//! RO:TEST — tests/native_passport_phase11d_device_bound_capability_adapter.rs.

use super::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, NativeDeviceBoundCapabilityContractDecisionV1,
    NativePassportProofAuthority, NativePassportProofKind, NativePassportScope,
    NativeProofSignatureAlgorithm, NativeProofSignedPayloadHex, NativeProofSigningOperationKind,
    PassportIdV1, PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN,
    PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_VERSION, PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
    PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
    PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
};

pub const NATIVE_PASSPORT_PHASE11D_LABEL: &str =
    "NATIVE_PASSPORT_PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER";

pub const PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_DOMAIN: &str =
    "native-passport/device-bound-capability-adapter/v1";

pub const PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_VERSION: u16 = 1;

pub const PHASE11D_FORBIDDEN_CAPABILITY_ADAPTER_AUTHORITY_FLAGS: &[&str] = &[
    "secret_key_access",
    "key_loading",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_unseal",
    "capability_consumption_execution",
    "capability_refresh_execution",
    "capability_revocation_execution",
    "runtime_io",
    "route_added",
    "storage_mutation_inside_svc_passport_native",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceBoundCapabilityAdapterDraftV1 {
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
    pub device_bound: bool,
    pub allows_injected_capability_state_adapter: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_loading: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_capability_consumption_execution: bool,
    pub requests_capability_lifecycle_runtime: bool,
    pub requests_runtime_io: bool,
    pub adds_routes: bool,
    pub requests_storage_mutation_inside_native: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceBoundCapabilityAdapterRequestV1 {
    pub operation_kind: NativeProofSigningOperationKind,
    pub expected_authority: NativePassportProofAuthority,
    pub challenge_id: ChallengeIdV1,
    pub passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
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
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceBoundCapabilityAdapterEvidenceV1 {
    pub accepted: bool,
    pub capability_state_adapter_label: &'static str,
    pub challenge_id: ChallengeIdV1,
    pub device_id: Option<DeviceIdV1>,
    pub capability_id_hash: B3DigestHex,
    pub capability_binding_hash: B3DigestHex,
    pub capability_scope_hash: B3DigestHex,
    pub replay_key_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub capability_recorded: bool,
    pub device_bound: bool,
    pub capability_consumed: bool,
    pub secret_material_exposed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceBoundCapabilityAdapterEnvelopeV1 {
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
    pub capability_state_adapter_label: &'static str,
    pub replay_key_hash: B3DigestHex,
    pub idempotency_key_hash: B3DigestHex,
    pub capability_id_hash: B3DigestHex,
    pub capability_binding_hash: B3DigestHex,
    pub capability_scope_hash: B3DigestHex,
    pub capability_scopes: Vec<NativePassportScope>,
    pub capability_issued_at_ms: u64,
    pub capability_expires_at_ms: u64,
    pub device_bound: bool,
    pub capability_recorded_by_adapter: bool,
    pub capability_consumed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDeviceBoundCapabilityAdapterReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    CapabilityDecisionInvalid,
    CapabilityDecisionAlreadyApplied,
    InjectedCapabilityStateAdapterNotAllowed,
    OperationMismatch,
    AuthorityMismatch,
    ChallengeIdMismatch,
    PassportBindingMismatch,
    DeviceBindingMissing,
    DeviceBindingMismatch,
    ProofContractDomainMismatch,
    ProofContractVersionMismatch,
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
    CapabilityIdMismatch,
    CapabilityBindingMismatch,
    CapabilityScopeHashMismatch,
    CapabilityScopesMismatch,
    CapabilityTimeMismatch,
    CapabilityNotDeviceBound,
    UnsafeAdapterAuthorityFlag,
    CapabilityStateAdapterRejected,
    CapabilityStateAdapterEvidenceMismatch,
    CapabilityStateAdapterEvidenceUnsafe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceBoundCapabilityAdapterPosture {
    pub phase_label: &'static str,
    pub device_bound_capability_adapter_added: bool,
    pub injected_capability_state_adapter_added: bool,
    pub capability_recording_evidence_added: bool,
    pub capability_consumption_execution_added: bool,
    pub capability_lifecycle_runtime_added: bool,
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

pub trait NativeLocalDeviceBoundCapabilityAdapter {
    fn apply_device_bound_capability(
        &self,
        request: &NativeDeviceBoundCapabilityAdapterRequestV1,
    ) -> Result<
        NativeDeviceBoundCapabilityAdapterEvidenceV1,
        NativeDeviceBoundCapabilityAdapterReviewError,
    >;
}

pub fn native_device_bound_capability_adapter_posture() -> NativeDeviceBoundCapabilityAdapterPosture
{
    NativeDeviceBoundCapabilityAdapterPosture {
        phase_label: NATIVE_PASSPORT_PHASE11D_LABEL,
        device_bound_capability_adapter_added: true,
        injected_capability_state_adapter_added: true,
        capability_recording_evidence_added: true,
        capability_consumption_execution_added: false,
        capability_lifecycle_runtime_added: false,
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
        forbidden_authority_flags: PHASE11D_FORBIDDEN_CAPABILITY_ADAPTER_AUTHORITY_FLAGS,
    }
}

pub fn execute_native_device_bound_capability_adapter<
    S: NativeLocalDeviceBoundCapabilityAdapter,
>(
    decision: &NativeDeviceBoundCapabilityContractDecisionV1,
    draft: NativeDeviceBoundCapabilityAdapterDraftV1,
    adapter: &S,
) -> Result<
    NativeDeviceBoundCapabilityAdapterEnvelopeV1,
    NativeDeviceBoundCapabilityAdapterReviewError,
> {
    validate_decision(decision)?;
    validate_adapter_draft(decision, &draft)?;

    let request = NativeDeviceBoundCapabilityAdapterRequestV1 {
        operation_kind: draft.operation_kind,
        expected_authority: draft.expected_authority,
        challenge_id: draft.challenge_id.clone(),
        passport_id: draft.passport_id.clone(),
        device_id: draft.device_id.clone(),
        proof_kind: draft.proof_kind,
        transcript_hash: draft.transcript_hash.clone(),
        signer_public_key_label: draft.signer_public_key_label,
        expected_signature_algorithm: draft.expected_signature_algorithm,
        signed_payload_hex: draft.signed_payload_hex.clone(),
        verification_evidence_label: draft.verification_evidence_label,
        verifier_public_key_label: draft.verifier_public_key_label,
        state_adapter_label: draft.state_adapter_label,
        replay_key_hash: draft.replay_key_hash.clone(),
        idempotency_key_hash: draft.idempotency_key_hash.clone(),
        capability_id_hash: draft.capability_id_hash.clone(),
        capability_binding_hash: draft.capability_binding_hash.clone(),
        capability_scope_hash: draft.capability_scope_hash.clone(),
        capability_scopes: draft.capability_scopes.clone(),
        capability_issued_at_ms: draft.capability_issued_at_ms,
        capability_expires_at_ms: draft.capability_expires_at_ms,
        device_bound: draft.device_bound,
        contract_only: true,
    };

    let evidence = adapter.apply_device_bound_capability(&request)?;
    validate_evidence(&request, &evidence)?;

    Ok(NativeDeviceBoundCapabilityAdapterEnvelopeV1 {
        contract_domain: PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_DOMAIN,
        contract_version: PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_VERSION,
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
        capability_state_adapter_label: evidence.capability_state_adapter_label,
        replay_key_hash: draft.replay_key_hash,
        idempotency_key_hash: draft.idempotency_key_hash,
        capability_id_hash: draft.capability_id_hash,
        capability_binding_hash: draft.capability_binding_hash,
        capability_scope_hash: draft.capability_scope_hash,
        capability_scopes: draft.capability_scopes,
        capability_issued_at_ms: draft.capability_issued_at_ms,
        capability_expires_at_ms: draft.capability_expires_at_ms,
        device_bound: true,
        capability_recorded_by_adapter: true,
        capability_consumed: false,
        runtime_io_performed: false,
        wallet_or_ledger_mutated: false,
    })
}

fn validate_decision(
    decision: &NativeDeviceBoundCapabilityContractDecisionV1,
) -> Result<(), NativeDeviceBoundCapabilityAdapterReviewError> {
    if decision.contract_domain != PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN
        || decision.contract_version != PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_VERSION
        || !decision.contract_only
        || !decision.device_bound
        || !decision.replay_recorded_reviewed
        || !decision.challenge_consumed_reviewed
    {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityDecisionInvalid);
    }

    if decision.capability_issued
        || decision.capability_state_changed
        || decision.runtime_io_performed
        || decision.wallet_or_ledger_mutated
    {
        return Err(
            NativeDeviceBoundCapabilityAdapterReviewError::CapabilityDecisionAlreadyApplied,
        );
    }

    if decision.operation_kind != NativeProofSigningOperationKind::RequestProof
        || decision.reviewed_authority != NativePassportProofAuthority::DeviceAuthority
    {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityDecisionInvalid);
    }

    if decision.device_id.is_none() {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::DeviceBindingMissing);
    }

    Ok(())
}

fn validate_adapter_draft(
    decision: &NativeDeviceBoundCapabilityContractDecisionV1,
    draft: &NativeDeviceBoundCapabilityAdapterDraftV1,
) -> Result<(), NativeDeviceBoundCapabilityAdapterReviewError> {
    if draft.contract_domain != PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_DOMAIN {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE11D_DEVICE_BOUND_CAPABILITY_ADAPTER_VERSION {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::ContractVersionMismatch);
    }

    if !draft.allows_injected_capability_state_adapter {
        return Err(
            NativeDeviceBoundCapabilityAdapterReviewError::InjectedCapabilityStateAdapterNotAllowed,
        );
    }

    if draft.operation_kind != decision.operation_kind {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::OperationMismatch);
    }

    if draft.expected_authority != decision.reviewed_authority {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::AuthorityMismatch);
    }

    if draft.challenge_id != decision.challenge_id {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::ChallengeIdMismatch);
    }

    if draft.passport_id != decision.passport_id {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::PassportBindingMismatch);
    }

    let draft_device_id = draft
        .device_id
        .as_ref()
        .ok_or(NativeDeviceBoundCapabilityAdapterReviewError::DeviceBindingMissing)?;
    let decision_device_id = decision
        .device_id
        .as_ref()
        .ok_or(NativeDeviceBoundCapabilityAdapterReviewError::DeviceBindingMissing)?;

    if draft_device_id != decision_device_id {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::DeviceBindingMismatch);
    }

    if draft.proof_contract_domain != decision.proof_contract_domain
        || draft.proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
    {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::ProofContractDomainMismatch);
    }

    if draft.proof_contract_version != decision.proof_contract_version
        || draft.proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
    {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::ProofContractVersionMismatch);
    }

    if draft.request_contract_domain != decision.request_contract_domain
        || draft.request_contract_domain != Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN)
    {
        return Err(
            NativeDeviceBoundCapabilityAdapterReviewError::RequestProofContractDomainMismatch,
        );
    }

    if draft.request_contract_version != decision.request_contract_version
        || draft.request_contract_version != Some(PHASE8C_PROOF_REQUEST_CONTRACT_VERSION)
    {
        return Err(
            NativeDeviceBoundCapabilityAdapterReviewError::RequestProofContractVersionMismatch,
        );
    }

    if draft.proof_kind != decision.proof_kind {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::ProofKindMismatch);
    }

    if draft.transcript_hash != decision.transcript_hash {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::TranscriptHashMismatch);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::MissingSignerPublicKeyLabel);
    }

    if draft.signer_public_key_label != decision.signer_public_key_label {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::SignerPublicKeyLabelMismatch);
    }

    validate_signature_algorithm(draft.expected_authority, draft.expected_signature_algorithm)?;

    if draft.expected_signature_algorithm != decision.expected_signature_algorithm {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::SignatureAlgorithmMismatch);
    }

    if draft.signed_payload_hex != decision.signed_payload_hex {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::SignedPayloadMismatch);
    }

    if draft.verification_evidence_label.is_empty() {
        return Err(
            NativeDeviceBoundCapabilityAdapterReviewError::MissingVerificationEvidenceLabel,
        );
    }

    if draft.verification_evidence_label != decision.verification_evidence_label {
        return Err(
            NativeDeviceBoundCapabilityAdapterReviewError::VerificationEvidenceLabelMismatch,
        );
    }

    if draft.verifier_public_key_label.is_empty() {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::MissingVerifierPublicKeyLabel);
    }

    if draft.verifier_public_key_label != decision.verifier_public_key_label {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::VerifierPublicKeyLabelMismatch);
    }

    if draft.state_adapter_label.is_empty() {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::MissingStateAdapterLabel);
    }

    if draft.state_adapter_label != decision.state_adapter_label {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::StateAdapterLabelMismatch);
    }

    if draft.replay_key_hash != decision.replay_key_hash {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::ReplayKeyMismatch);
    }

    if draft.idempotency_key_hash != decision.idempotency_key_hash {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::IdempotencyKeyMismatch);
    }

    if draft.capability_id_hash != decision.capability_id_hash {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityIdMismatch);
    }

    if draft.capability_binding_hash != decision.capability_binding_hash {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityBindingMismatch);
    }

    if draft.capability_scope_hash != decision.capability_scope_hash {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityScopeHashMismatch);
    }

    if draft.capability_scopes != decision.capability_scopes {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityScopesMismatch);
    }

    if draft.capability_issued_at_ms != decision.capability_issued_at_ms
        || draft.capability_expires_at_ms != decision.capability_expires_at_ms
    {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityTimeMismatch);
    }

    if !draft.device_bound {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityNotDeviceBound);
    }

    validate_no_unsafe_flags(draft)
}

fn validate_signature_algorithm(
    authority: NativePassportProofAuthority,
    algorithm: NativeProofSignatureAlgorithm,
) -> Result<(), NativeDeviceBoundCapabilityAdapterReviewError> {
    if authority != NativePassportProofAuthority::DeviceAuthority
        || algorithm != PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM
    {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::SignatureAlgorithmMismatch);
    }

    Ok(())
}

fn validate_no_unsafe_flags(
    draft: &NativeDeviceBoundCapabilityAdapterDraftV1,
) -> Result<(), NativeDeviceBoundCapabilityAdapterReviewError> {
    if draft.requests_secret_key_access
        || draft.requests_key_loading
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_capability_consumption_execution
        || draft.requests_capability_lifecycle_runtime
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation_inside_native
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::UnsafeAdapterAuthorityFlag);
    }

    Ok(())
}

fn validate_evidence(
    request: &NativeDeviceBoundCapabilityAdapterRequestV1,
    evidence: &NativeDeviceBoundCapabilityAdapterEvidenceV1,
) -> Result<(), NativeDeviceBoundCapabilityAdapterReviewError> {
    if !evidence.accepted {
        return Err(NativeDeviceBoundCapabilityAdapterReviewError::CapabilityStateAdapterRejected);
    }

    if evidence.capability_state_adapter_label.is_empty() {
        return Err(
            NativeDeviceBoundCapabilityAdapterReviewError::CapabilityStateAdapterEvidenceMismatch,
        );
    }

    if evidence.challenge_id != request.challenge_id
        || evidence.device_id != request.device_id
        || evidence.capability_id_hash != request.capability_id_hash
        || evidence.capability_binding_hash != request.capability_binding_hash
        || evidence.capability_scope_hash != request.capability_scope_hash
        || evidence.replay_key_hash != request.replay_key_hash
        || evidence.idempotency_key_hash != request.idempotency_key_hash
    {
        return Err(
            NativeDeviceBoundCapabilityAdapterReviewError::CapabilityStateAdapterEvidenceMismatch,
        );
    }

    if !evidence.capability_recorded || !evidence.device_bound || evidence.capability_consumed {
        return Err(
            NativeDeviceBoundCapabilityAdapterReviewError::CapabilityStateAdapterEvidenceMismatch,
        );
    }

    if evidence.secret_material_exposed
        || evidence.runtime_io_performed
        || evidence.wallet_or_ledger_mutated
    {
        return Err(
            NativeDeviceBoundCapabilityAdapterReviewError::CapabilityStateAdapterEvidenceUnsafe,
        );
    }

    Ok(())
}
