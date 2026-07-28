//! RO:WHAT — Native Passport injected proof-verification adapter seam.
//! RO:WHY — P3 Identity & Keys. Allows a future platform-local/public-key verifier to assess a signed payload only after Phase 10A accepted the exact signed-envelope binding.
//! RO:INTERACTS — Phase 10A proof-verification boundary decisions, Phase 9B signed-payload envelopes, future verifier implementations.
//! RO:INVARIANTS — validates boundary decision, signed payload, public signer label, authority, algorithm, transcript hash, timing, and unsafe flags before calling an injected verifier. It does not load keys, unlock vaults, derive keys, mutate replay/storage, consume challenges/capabilities, issue capabilities, add routes, perform runtime I/O, or touch wallet/ledger state.
//! RO:TEST — tests/native_passport_phase10b_native_proof_verification_adapter.rs.

use super::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, NativePassportProofAuthority, NativePassportProofKind,
    NativeProofSignatureAlgorithm, NativeProofSignedPayloadHex, NativeProofSigningOperationKind,
    NativeProofVerificationContractBoundaryDecisionV1, PassportIdV1,
    PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_PROOF_CONTRACT_DOMAIN,
    PHASE8B_PROOF_CONTRACT_VERSION, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
    PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
};

pub const NATIVE_PASSPORT_PHASE10B_LABEL: &str =
    "NATIVE_PASSPORT_PHASE10B_NATIVE_PROOF_VERIFICATION_ADAPTER";

pub const PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN: &str =
    "native-passport/proof-verification-adapter/v1";

pub const PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION: u16 = 1;

pub const PHASE10B_FORBIDDEN_VERIFICATION_ADAPTER_AUTHORITY_FLAGS: &[&str] = &[
    "secret_key_access",
    "key_loading",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_unseal",
    "replay_store_mutation",
    "challenge_consumption",
    "capability_consumption",
    "capability_issuance",
    "capability_refresh",
    "capability_revocation",
    "runtime_io",
    "route_added",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofVerificationAdapterDraftV1 {
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
    pub verification_requested_at_ms: u64,
    pub allows_injected_local_verifier: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_loading: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_replay_store_mutation: bool,
    pub requests_challenge_consumption: bool,
    pub requests_capability_consumption: bool,
    pub requests_capability_issuance: bool,
    pub requests_capability_lifecycle_runtime: bool,
    pub requests_runtime_io: bool,
    pub adds_routes: bool,
    pub requests_storage_mutation: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofVerificationAdapterRequestV1 {
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
    pub verification_requested_at_ms: u64,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofVerificationAdapterEvidenceV1 {
    pub accepted: bool,
    pub evidence_label: &'static str,
    pub verifier_public_key_label: &'static str,
    pub transcript_hash: B3DigestHex,
    pub signed_payload_hex: NativeProofSignedPayloadHex,
    pub public_key_only: bool,
    pub secret_material_exposed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofVerificationAdapterEnvelopeV1 {
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
    pub verification_requested_at_ms: u64,
    pub evidence_label: &'static str,
    pub verifier_public_key_label: &'static str,
    pub signed_payload_accepted: bool,
    pub replay_state_mutated: bool,
    pub challenge_consumed: bool,
    pub capability_state_changed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeProofVerificationAdapterReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    BoundaryDecisionInvalid,
    BoundaryDecisionAlreadyMutatedState,
    InjectedVerifierNotAllowed,
    OperationMismatch,
    AuthorityMismatch,
    ChallengeIdMismatch,
    PassportBindingMismatch,
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
    VerificationTimeMismatch,
    UnsafeAdapterAuthorityFlag,
    VerifierRejected,
    VerifierEvidenceMismatch,
    VerifierEvidenceUnsafe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofVerificationAdapterPosture {
    pub phase_label: &'static str,
    pub proof_verification_adapter_added: bool,
    pub injected_local_verifier_adapter_added: bool,
    pub root_proof_verification_adapter_added: bool,
    pub device_proof_verification_adapter_added: bool,
    pub request_proof_verification_adapter_added: bool,
    pub secret_key_access_added: bool,
    pub key_loading_added: bool,
    pub key_derivation_runtime_added: bool,
    pub vault_unlock_added: bool,
    pub vault_runtime_added: bool,
    pub platform_sealer_unseal_added: bool,
    pub replay_store_mutation_added: bool,
    pub challenge_consumption_added: bool,
    pub capability_consumption_added: bool,
    pub capability_issuance_added: bool,
    pub capability_lifecycle_runtime_added: bool,
    pub runtime_io_added: bool,
    pub routes_added: bool,
    pub storage_mutation_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub trait NativeLocalProofVerificationAdapter {
    fn assess_signed_payload(
        &self,
        request: &NativeProofVerificationAdapterRequestV1,
    ) -> Result<NativeProofVerificationAdapterEvidenceV1, NativeProofVerificationAdapterReviewError>;
}

pub fn native_proof_verification_adapter_posture() -> NativeProofVerificationAdapterPosture {
    NativeProofVerificationAdapterPosture {
        phase_label: NATIVE_PASSPORT_PHASE10B_LABEL,
        proof_verification_adapter_added: true,
        injected_local_verifier_adapter_added: true,
        root_proof_verification_adapter_added: true,
        device_proof_verification_adapter_added: true,
        request_proof_verification_adapter_added: true,
        secret_key_access_added: false,
        key_loading_added: false,
        key_derivation_runtime_added: false,
        vault_unlock_added: false,
        vault_runtime_added: false,
        platform_sealer_unseal_added: false,
        replay_store_mutation_added: false,
        challenge_consumption_added: false,
        capability_consumption_added: false,
        capability_issuance_added: false,
        capability_lifecycle_runtime_added: false,
        runtime_io_added: false,
        routes_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE10B_FORBIDDEN_VERIFICATION_ADAPTER_AUTHORITY_FLAGS,
    }
}

pub fn execute_native_proof_verification_adapter<S: NativeLocalProofVerificationAdapter>(
    decision: &NativeProofVerificationContractBoundaryDecisionV1,
    draft: NativeProofVerificationAdapterDraftV1,
    verifier: &S,
) -> Result<NativeProofVerificationAdapterEnvelopeV1, NativeProofVerificationAdapterReviewError> {
    validate_decision(decision)?;
    validate_adapter_draft(decision, &draft)?;

    let request = NativeProofVerificationAdapterRequestV1 {
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
        verification_requested_at_ms: draft.verification_requested_at_ms,
        contract_only: true,
    };

    let evidence = verifier.assess_signed_payload(&request)?;
    validate_evidence(&request, &evidence)?;

    Ok(NativeProofVerificationAdapterEnvelopeV1 {
        contract_domain: PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN,
        contract_version: PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION,
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
        verification_requested_at_ms: draft.verification_requested_at_ms,
        evidence_label: evidence.evidence_label,
        verifier_public_key_label: evidence.verifier_public_key_label,
        signed_payload_accepted: true,
        replay_state_mutated: false,
        challenge_consumed: false,
        capability_state_changed: false,
        runtime_io_performed: false,
        wallet_or_ledger_mutated: false,
    })
}

fn validate_decision(
    decision: &NativeProofVerificationContractBoundaryDecisionV1,
) -> Result<(), NativeProofVerificationAdapterReviewError> {
    if !decision.contract_only {
        return Err(NativeProofVerificationAdapterReviewError::BoundaryDecisionInvalid);
    }

    if decision.replay_state_mutated
        || decision.challenge_consumed
        || decision.capability_state_changed
        || decision.runtime_io_performed
        || decision.wallet_or_ledger_mutated
    {
        return Err(NativeProofVerificationAdapterReviewError::BoundaryDecisionAlreadyMutatedState);
    }

    Ok(())
}

fn validate_adapter_draft(
    decision: &NativeProofVerificationContractBoundaryDecisionV1,
    draft: &NativeProofVerificationAdapterDraftV1,
) -> Result<(), NativeProofVerificationAdapterReviewError> {
    if draft.contract_domain != PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN {
        return Err(NativeProofVerificationAdapterReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION {
        return Err(NativeProofVerificationAdapterReviewError::ContractVersionMismatch);
    }

    if !draft.allows_injected_local_verifier {
        return Err(NativeProofVerificationAdapterReviewError::InjectedVerifierNotAllowed);
    }

    if draft.operation_kind != decision.operation_kind {
        return Err(NativeProofVerificationAdapterReviewError::OperationMismatch);
    }

    if draft.expected_authority != decision.reviewed_authority {
        return Err(NativeProofVerificationAdapterReviewError::AuthorityMismatch);
    }

    if draft.challenge_id != decision.challenge_id {
        return Err(NativeProofVerificationAdapterReviewError::ChallengeIdMismatch);
    }

    if draft.passport_id != decision.passport_id {
        return Err(NativeProofVerificationAdapterReviewError::PassportBindingMismatch);
    }

    if draft.device_id != decision.device_id {
        return Err(NativeProofVerificationAdapterReviewError::DeviceBindingMismatch);
    }

    if draft.proof_contract_domain != decision.proof_contract_domain
        || draft.proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
    {
        return Err(NativeProofVerificationAdapterReviewError::ProofContractDomainMismatch);
    }

    if draft.proof_contract_version != decision.proof_contract_version
        || draft.proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
    {
        return Err(NativeProofVerificationAdapterReviewError::ProofContractVersionMismatch);
    }

    if draft.request_contract_domain != decision.request_contract_domain {
        return Err(NativeProofVerificationAdapterReviewError::RequestProofContractDomainMismatch);
    }

    if draft.request_contract_version != decision.request_contract_version {
        return Err(NativeProofVerificationAdapterReviewError::RequestProofContractVersionMismatch);
    }

    if let Some(domain) = draft.request_contract_domain {
        if domain != PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN {
            return Err(
                NativeProofVerificationAdapterReviewError::RequestProofContractDomainMismatch,
            );
        }
    }

    if let Some(version) = draft.request_contract_version {
        if version != PHASE8C_PROOF_REQUEST_CONTRACT_VERSION {
            return Err(
                NativeProofVerificationAdapterReviewError::RequestProofContractVersionMismatch,
            );
        }
    }

    if draft.proof_kind != decision.proof_kind {
        return Err(NativeProofVerificationAdapterReviewError::ProofKindMismatch);
    }

    if draft.transcript_hash != decision.transcript_hash {
        return Err(NativeProofVerificationAdapterReviewError::TranscriptHashMismatch);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(NativeProofVerificationAdapterReviewError::MissingSignerPublicKeyLabel);
    }

    if draft.signer_public_key_label != decision.signer_public_key_label {
        return Err(NativeProofVerificationAdapterReviewError::SignerPublicKeyLabelMismatch);
    }

    validate_signature_algorithm(draft.expected_authority, draft.expected_signature_algorithm)?;

    if draft.expected_signature_algorithm != decision.expected_signature_algorithm {
        return Err(NativeProofVerificationAdapterReviewError::SignatureAlgorithmMismatch);
    }

    if draft.signed_payload_hex != decision.signed_payload_hex {
        return Err(NativeProofVerificationAdapterReviewError::SignedPayloadMismatch);
    }

    if draft.verification_requested_at_ms != decision.verification_requested_at_ms {
        return Err(NativeProofVerificationAdapterReviewError::VerificationTimeMismatch);
    }

    validate_no_unsafe_flags(draft)
}

fn validate_signature_algorithm(
    authority: NativePassportProofAuthority,
    algorithm: NativeProofSignatureAlgorithm,
) -> Result<(), NativeProofVerificationAdapterReviewError> {
    let expected = match authority {
        NativePassportProofAuthority::RootAuthority => PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        NativePassportProofAuthority::DeviceAuthority => PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
    };

    if algorithm != expected {
        return Err(NativeProofVerificationAdapterReviewError::SignatureAlgorithmMismatch);
    }

    Ok(())
}

fn validate_no_unsafe_flags(
    draft: &NativeProofVerificationAdapterDraftV1,
) -> Result<(), NativeProofVerificationAdapterReviewError> {
    if draft.requests_secret_key_access
        || draft.requests_key_loading
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_replay_store_mutation
        || draft.requests_challenge_consumption
        || draft.requests_capability_consumption
        || draft.requests_capability_issuance
        || draft.requests_capability_lifecycle_runtime
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeProofVerificationAdapterReviewError::UnsafeAdapterAuthorityFlag);
    }

    Ok(())
}

fn validate_evidence(
    request: &NativeProofVerificationAdapterRequestV1,
    evidence: &NativeProofVerificationAdapterEvidenceV1,
) -> Result<(), NativeProofVerificationAdapterReviewError> {
    if !evidence.accepted {
        return Err(NativeProofVerificationAdapterReviewError::VerifierRejected);
    }

    if evidence.evidence_label.is_empty() || evidence.verifier_public_key_label.is_empty() {
        return Err(NativeProofVerificationAdapterReviewError::VerifierEvidenceMismatch);
    }

    if evidence.transcript_hash != request.transcript_hash
        || evidence.signed_payload_hex != request.signed_payload_hex
    {
        return Err(NativeProofVerificationAdapterReviewError::VerifierEvidenceMismatch);
    }

    if !evidence.public_key_only
        || evidence.secret_material_exposed
        || evidence.runtime_io_performed
        || evidence.wallet_or_ledger_mutated
    {
        return Err(NativeProofVerificationAdapterReviewError::VerifierEvidenceUnsafe);
    }

    Ok(())
}
