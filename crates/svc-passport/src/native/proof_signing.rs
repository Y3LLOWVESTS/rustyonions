//! RO:WHAT — Native Passport proof-signing contract boundary.
//! RO:WHY — P3 Identity & Keys. Adds preflight review before native signing runtime or key access is enabled.
//! RO:INTERACTS — Phase 8A challenge descriptors, Phase 8B proof descriptors, Phase 8C request-proof descriptors, future native signing adapters.
//! RO:INVARIANTS — validates linkage, authority, unlock posture, transcript hash, signer label, algorithm, and time. Emits only contract decisions: no signing, key access, key derivation, vault unlock, verification, replay mutation, challenge/capability consumption, routes, runtime I/O, wallet, ledger, or secret custody.
//! RO:TEST — tests/native_passport_phase9a_native_proof_signing_contract_boundary.rs.

use super::{
    validate_native_passport_proof_contract_descriptor,
    validate_native_passport_request_proof_contract_descriptor, B3DigestHex, ChallengeIdV1,
    DeviceIdV1, NativePassportChallengeContractDescriptorV1, NativePassportProofAuthority,
    NativePassportProofContractDescriptorV1, NativePassportProofKind,
    NativePassportRequestProofContractDescriptorV1, NativeProofSignatureAlgorithm, PassportIdV1,
    PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_PROOF_CONTRACT_DOMAIN,
    PHASE8B_PROOF_CONTRACT_VERSION, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
    PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
};

pub const NATIVE_PASSPORT_PHASE9A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE9A_NATIVE_PROOF_SIGNING_CONTRACT_BOUNDARY";

pub const PHASE9A_PROOF_SIGNING_BOUNDARY_DOMAIN: &str =
    "native-passport/proof-signing-contract-boundary/v1";

pub const PHASE9A_PROOF_SIGNING_BOUNDARY_VERSION: u16 = 1;

pub const PHASE9A_FORBIDDEN_PROOF_SIGNING_AUTHORITY_FLAGS: &[&str] = &[
    "root_proof_signing_runtime",
    "device_proof_signing_runtime",
    "request_proof_signing_runtime",
    "signature_bytes_export",
    "secret_key_access",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_implementation",
    "proof_signature_verification",
    "request_proof_verification",
    "service_challenge_verification",
    "replay_store_mutation",
    "challenge_consumption",
    "capability_consumption",
    "capability_issuance",
    "runtime_io",
    "route_added",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeProofSigningOperationKind {
    RootProof,
    DeviceProof,
    RequestProof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeProofSigningUnlockState {
    Locked,
    RootUnlocked,
    DeviceUnlocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofSigningContractBoundaryDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub operation_kind: NativeProofSigningOperationKind,
    pub unlocked_authority: NativeProofSigningUnlockState,
    pub challenge_id: ChallengeIdV1,
    pub passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
    pub proof_contract_domain: &'static str,
    pub proof_contract_version: u16,
    pub request_contract_domain: Option<&'static str>,
    pub request_contract_version: Option<u16>,
    pub proof_kind: NativePassportProofKind,
    pub proof_authority: NativePassportProofAuthority,
    pub transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub requested_signature_algorithm: NativeProofSignatureAlgorithm,
    pub signing_requested_at_ms: u64,
    pub requests_root_proof_signing_runtime: bool,
    pub requests_device_proof_signing_runtime: bool,
    pub requests_request_proof_signing_runtime: bool,
    pub emits_signed_payload: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub includes_platform_sealer_implementation: bool,
    pub requests_proof_signature_verification: bool,
    pub requests_request_proof_verification: bool,
    pub requests_service_challenge_verification: bool,
    pub requests_replay_store_mutation: bool,
    pub requests_challenge_consumption: bool,
    pub requests_capability_consumption: bool,
    pub requests_capability_issuance: bool,
    pub requests_runtime_io: bool,
    pub adds_routes: bool,
    pub requests_storage_mutation: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofSigningContractBoundaryDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub operation_kind: NativeProofSigningOperationKind,
    pub accepted_authority: NativePassportProofAuthority,
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
    pub requested_signature_algorithm: NativeProofSignatureAlgorithm,
    pub signing_requested_at_ms: u64,
    pub signing_runtime_enabled: bool,
    pub signed_payload_present: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeProofSigningContractBoundaryReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    ProofContractReferenceInvalid,
    RequestProofContractReferenceInvalid,
    RequestProofReferenceRequired,
    RequestProofReferenceUnexpected,
    ProofContractDomainMismatch,
    ProofContractVersionMismatch,
    RequestProofContractDomainMismatch,
    RequestProofContractVersionMismatch,
    ChallengeIdMismatch,
    PassportBindingMismatch,
    DeviceBindingMismatch,
    ProofKindMismatch,
    ProofAuthorityMismatch,
    TranscriptHashMismatch,
    MissingSignerPublicKeyLabel,
    LockedStateDenied,
    RootUnlockRequired,
    DeviceUnlockRequired,
    SignatureAlgorithmMismatch,
    InvalidSigningRequestedAt,
    UnsafeSigningAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofSigningContractBoundaryPosture {
    pub phase_label: &'static str,
    pub proof_signing_contract_boundary_added: bool,
    pub root_proof_signing_runtime_added: bool,
    pub device_proof_signing_runtime_added: bool,
    pub request_proof_signing_runtime_added: bool,
    pub signed_payload_export_added: bool,
    pub secret_key_access_added: bool,
    pub key_derivation_runtime_added: bool,
    pub vault_unlock_added: bool,
    pub vault_runtime_added: bool,
    pub platform_sealer_implementation_added: bool,
    pub proof_signature_verification_added: bool,
    pub request_proof_verification_added: bool,
    pub replay_store_mutation_added: bool,
    pub challenge_consumption_added: bool,
    pub capability_consumption_added: bool,
    pub capability_issuance_added: bool,
    pub runtime_io_added: bool,
    pub routes_added: bool,
    pub storage_mutation_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_proof_signing_contract_boundary_posture() -> NativeProofSigningContractBoundaryPosture
{
    NativeProofSigningContractBoundaryPosture {
        phase_label: NATIVE_PASSPORT_PHASE9A_LABEL,
        proof_signing_contract_boundary_added: true,
        root_proof_signing_runtime_added: false,
        device_proof_signing_runtime_added: false,
        request_proof_signing_runtime_added: false,
        signed_payload_export_added: false,
        secret_key_access_added: false,
        key_derivation_runtime_added: false,
        vault_unlock_added: false,
        vault_runtime_added: false,
        platform_sealer_implementation_added: false,
        proof_signature_verification_added: false,
        request_proof_verification_added: false,
        replay_store_mutation_added: false,
        challenge_consumption_added: false,
        capability_consumption_added: false,
        capability_issuance_added: false,
        runtime_io_added: false,
        routes_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE9A_FORBIDDEN_PROOF_SIGNING_AUTHORITY_FLAGS,
    }
}

pub fn review_native_proof_signing_contract_boundary(
    challenge: &NativePassportChallengeContractDescriptorV1,
    proof: &NativePassportProofContractDescriptorV1,
    request: Option<&NativePassportRequestProofContractDescriptorV1>,
    draft: NativeProofSigningContractBoundaryDraftV1,
) -> Result<
    NativeProofSigningContractBoundaryDecisionV1,
    NativeProofSigningContractBoundaryReviewError,
> {
    validate_native_passport_proof_contract_descriptor(challenge, proof).map_err(|_| {
        NativeProofSigningContractBoundaryReviewError::ProofContractReferenceInvalid
    })?;

    if draft.contract_domain != PHASE9A_PROOF_SIGNING_BOUNDARY_DOMAIN {
        return Err(NativeProofSigningContractBoundaryReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE9A_PROOF_SIGNING_BOUNDARY_VERSION {
        return Err(NativeProofSigningContractBoundaryReviewError::ContractVersionMismatch);
    }

    if draft.proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
        || draft.proof_contract_domain != proof.contract_domain
    {
        return Err(NativeProofSigningContractBoundaryReviewError::ProofContractDomainMismatch);
    }

    if draft.proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
        || draft.proof_contract_version != proof.contract_version
    {
        return Err(NativeProofSigningContractBoundaryReviewError::ProofContractVersionMismatch);
    }

    if draft.challenge_id != proof.challenge_id || draft.challenge_id != challenge.challenge_id {
        return Err(NativeProofSigningContractBoundaryReviewError::ChallengeIdMismatch);
    }

    if draft.passport_id != proof.passport_id {
        return Err(NativeProofSigningContractBoundaryReviewError::PassportBindingMismatch);
    }

    if draft.device_id != proof.device_id {
        return Err(NativeProofSigningContractBoundaryReviewError::DeviceBindingMismatch);
    }

    if draft.proof_kind != proof.proof_kind {
        return Err(NativeProofSigningContractBoundaryReviewError::ProofKindMismatch);
    }

    let expected_authority = expected_authority_for_operation(draft.operation_kind);
    if draft.proof_authority != proof.proof_authority || draft.proof_authority != expected_authority
    {
        return Err(NativeProofSigningContractBoundaryReviewError::ProofAuthorityMismatch);
    }

    let request_descriptor = validate_request_reference(
        challenge,
        proof,
        request,
        draft.operation_kind,
        draft.request_contract_domain,
        draft.request_contract_version,
    )?;

    let expected_transcript_hash = match request_descriptor {
        Some(request) => &request.request_transcript_hash,
        None => &proof.proof_transcript_hash,
    };

    if &draft.transcript_hash != expected_transcript_hash {
        return Err(NativeProofSigningContractBoundaryReviewError::TranscriptHashMismatch);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(NativeProofSigningContractBoundaryReviewError::MissingSignerPublicKeyLabel);
    }

    validate_unlock(expected_authority, draft.unlocked_authority)?;
    validate_signature_algorithm(expected_authority, draft.requested_signature_algorithm)?;
    validate_signing_time(proof, request_descriptor, draft.signing_requested_at_ms)?;

    if draft.requests_root_proof_signing_runtime
        || draft.requests_device_proof_signing_runtime
        || draft.requests_request_proof_signing_runtime
        || draft.emits_signed_payload
        || draft.requests_secret_key_access
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.includes_platform_sealer_implementation
        || draft.requests_proof_signature_verification
        || draft.requests_request_proof_verification
        || draft.requests_service_challenge_verification
        || draft.requests_replay_store_mutation
        || draft.requests_challenge_consumption
        || draft.requests_capability_consumption
        || draft.requests_capability_issuance
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeProofSigningContractBoundaryReviewError::UnsafeSigningAuthorityFlag);
    }

    Ok(NativeProofSigningContractBoundaryDecisionV1 {
        contract_domain: PHASE9A_PROOF_SIGNING_BOUNDARY_DOMAIN,
        contract_version: PHASE9A_PROOF_SIGNING_BOUNDARY_VERSION,
        operation_kind: draft.operation_kind,
        accepted_authority: expected_authority,
        challenge_id: draft.challenge_id,
        passport_id: draft.passport_id,
        device_id: draft.device_id,
        proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
        proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
        request_contract_domain: request_descriptor.map(|request| request.contract_domain),
        request_contract_version: request_descriptor.map(|request| request.contract_version),
        proof_kind: draft.proof_kind,
        transcript_hash: draft.transcript_hash,
        signer_public_key_label: draft.signer_public_key_label,
        requested_signature_algorithm: draft.requested_signature_algorithm,
        signing_requested_at_ms: draft.signing_requested_at_ms,
        signing_runtime_enabled: false,
        signed_payload_present: false,
        contract_only: true,
    })
}

fn validate_request_reference<'a>(
    challenge: &NativePassportChallengeContractDescriptorV1,
    proof: &NativePassportProofContractDescriptorV1,
    request: Option<&'a NativePassportRequestProofContractDescriptorV1>,
    operation_kind: NativeProofSigningOperationKind,
    request_contract_domain: Option<&'static str>,
    request_contract_version: Option<u16>,
) -> Result<
    Option<&'a NativePassportRequestProofContractDescriptorV1>,
    NativeProofSigningContractBoundaryReviewError,
> {
    if operation_kind != NativeProofSigningOperationKind::RequestProof {
        if request.is_some()
            || request_contract_domain.is_some()
            || request_contract_version.is_some()
        {
            return Err(
                NativeProofSigningContractBoundaryReviewError::RequestProofReferenceUnexpected,
            );
        }

        return Ok(None);
    }

    let request = request
        .ok_or(NativeProofSigningContractBoundaryReviewError::RequestProofReferenceRequired)?;

    validate_native_passport_request_proof_contract_descriptor(challenge, proof, request).map_err(
        |_| NativeProofSigningContractBoundaryReviewError::RequestProofContractReferenceInvalid,
    )?;

    if request_contract_domain != Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN)
        || request_contract_domain != Some(request.contract_domain)
    {
        return Err(
            NativeProofSigningContractBoundaryReviewError::RequestProofContractDomainMismatch,
        );
    }

    if request_contract_version != Some(PHASE8C_PROOF_REQUEST_CONTRACT_VERSION)
        || request_contract_version != Some(request.contract_version)
    {
        return Err(
            NativeProofSigningContractBoundaryReviewError::RequestProofContractVersionMismatch,
        );
    }

    Ok(Some(request))
}

fn expected_authority_for_operation(
    operation_kind: NativeProofSigningOperationKind,
) -> NativePassportProofAuthority {
    match operation_kind {
        NativeProofSigningOperationKind::RootProof => NativePassportProofAuthority::RootAuthority,
        NativeProofSigningOperationKind::DeviceProof
        | NativeProofSigningOperationKind::RequestProof => {
            NativePassportProofAuthority::DeviceAuthority
        }
    }
}

fn validate_unlock(
    expected_authority: NativePassportProofAuthority,
    unlocked_authority: NativeProofSigningUnlockState,
) -> Result<(), NativeProofSigningContractBoundaryReviewError> {
    if unlocked_authority == NativeProofSigningUnlockState::Locked {
        return Err(NativeProofSigningContractBoundaryReviewError::LockedStateDenied);
    }

    match (expected_authority, unlocked_authority) {
        (
            NativePassportProofAuthority::RootAuthority,
            NativeProofSigningUnlockState::RootUnlocked,
        )
        | (
            NativePassportProofAuthority::DeviceAuthority,
            NativeProofSigningUnlockState::DeviceUnlocked,
        ) => Ok(()),
        (NativePassportProofAuthority::RootAuthority, _) => {
            Err(NativeProofSigningContractBoundaryReviewError::RootUnlockRequired)
        }
        (NativePassportProofAuthority::DeviceAuthority, _) => {
            Err(NativeProofSigningContractBoundaryReviewError::DeviceUnlockRequired)
        }
    }
}

fn validate_signature_algorithm(
    expected_authority: NativePassportProofAuthority,
    requested_signature_algorithm: NativeProofSignatureAlgorithm,
) -> Result<(), NativeProofSigningContractBoundaryReviewError> {
    let expected_algorithm = match expected_authority {
        NativePassportProofAuthority::RootAuthority => PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        NativePassportProofAuthority::DeviceAuthority => PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
    };

    if requested_signature_algorithm != expected_algorithm {
        return Err(NativeProofSigningContractBoundaryReviewError::SignatureAlgorithmMismatch);
    }

    Ok(())
}

fn validate_signing_time(
    proof: &NativePassportProofContractDescriptorV1,
    request: Option<&NativePassportRequestProofContractDescriptorV1>,
    signing_requested_at_ms: u64,
) -> Result<(), NativeProofSigningContractBoundaryReviewError> {
    if signing_requested_at_ms < proof.proof_created_at_ms
        || signing_requested_at_ms > proof.challenge_expires_at_ms
    {
        return Err(NativeProofSigningContractBoundaryReviewError::InvalidSigningRequestedAt);
    }

    if let Some(request) = request {
        if signing_requested_at_ms < request.request_created_at_ms
            || signing_requested_at_ms > request.request_expires_at_ms
        {
            return Err(NativeProofSigningContractBoundaryReviewError::InvalidSigningRequestedAt);
        }
    }

    Ok(())
}
