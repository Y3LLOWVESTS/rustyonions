//! RO:WHAT — Native Passport proof-verification contract boundary.
//! RO:WHY — P3 Identity & Keys. Adds preflight review for signed proof envelopes before any cryptographic verification runtime, replay mutation, challenge consumption, or capability decision exists.
//! RO:INTERACTS — Phase 9B signed-payload envelopes, Phase 9A operation/authority labels, future verifier adapters.
//! RO:INVARIANTS — validates signed envelope binding, signer label, authority, algorithm, transcript hash, payload shape, request/proof contract references, and review timing. This phase is contract-boundary only: no cryptographic verification, no replay-store mutation, no challenge consumption, no capability consumption/issuance, no routes, no runtime I/O, no storage mutation, no wallet/ledger mutation, and no secret/key access.
//! RO:TEST — tests/native_passport_phase10a_native_proof_verification_contract_boundary.rs.

use super::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, NativePassportProofAuthority, NativePassportProofKind,
    NativeProofSignatureAlgorithm, NativeProofSignedPayloadHex,
    NativeProofSigningAdapterEnvelopeV1, NativeProofSigningOperationKind, PassportIdV1,
    PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_PROOF_CONTRACT_DOMAIN,
    PHASE8B_PROOF_CONTRACT_VERSION, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
    PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
    PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN, PHASE9B_PROOF_SIGNING_ADAPTER_VERSION,
};

pub const NATIVE_PASSPORT_PHASE10A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE10A_NATIVE_PROOF_VERIFICATION_CONTRACT_BOUNDARY";

pub const PHASE10A_PROOF_VERIFICATION_BOUNDARY_DOMAIN: &str =
    "native-passport/proof-verification-contract-boundary/v1";

pub const PHASE10A_PROOF_VERIFICATION_BOUNDARY_VERSION: u16 = 1;

pub const PHASE10A_FORBIDDEN_PROOF_VERIFICATION_AUTHORITY_FLAGS: &[&str] = &[
    "cryptographic_signature_verification_runtime",
    "request_proof_verification_runtime",
    "service_challenge_verification",
    "replay_store_mutation",
    "challenge_consumption",
    "capability_consumption",
    "capability_issuance",
    "capability_refresh",
    "capability_revocation",
    "secret_key_access",
    "key_loading",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_unseal",
    "runtime_io",
    "route_added",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofVerificationContractBoundaryDraftV1 {
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
    pub expects_signed_payload_present: bool,
    pub requests_cryptographic_signature_verification_runtime: bool,
    pub requests_request_proof_verification_runtime: bool,
    pub requests_service_challenge_verification: bool,
    pub requests_replay_store_mutation: bool,
    pub requests_challenge_consumption: bool,
    pub requests_capability_consumption: bool,
    pub requests_capability_issuance: bool,
    pub requests_capability_lifecycle_runtime: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_loading: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_runtime_io: bool,
    pub adds_routes: bool,
    pub requests_storage_mutation: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofVerificationContractBoundaryDecisionV1 {
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
    pub cryptographic_verification_performed: bool,
    pub replay_state_mutated: bool,
    pub challenge_consumed: bool,
    pub capability_state_changed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeProofVerificationContractBoundaryReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    SigningEnvelopeDomainMismatch,
    SigningEnvelopeVersionMismatch,
    SigningEnvelopeUnsigned,
    SigningEnvelopeUnsafe,
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
    SignedPayloadExpected,
    VerificationTimeBeforeSigning,
    UnsafeVerificationAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofVerificationContractBoundaryPosture {
    pub phase_label: &'static str,
    pub proof_verification_contract_boundary_added: bool,
    pub cryptographic_signature_verification_runtime_added: bool,
    pub request_proof_verification_runtime_added: bool,
    pub service_challenge_verification_added: bool,
    pub replay_store_mutation_added: bool,
    pub challenge_consumption_added: bool,
    pub capability_consumption_added: bool,
    pub capability_issuance_added: bool,
    pub capability_lifecycle_runtime_added: bool,
    pub secret_key_access_added: bool,
    pub key_loading_added: bool,
    pub key_derivation_runtime_added: bool,
    pub vault_unlock_added: bool,
    pub vault_runtime_added: bool,
    pub platform_sealer_unseal_added: bool,
    pub runtime_io_added: bool,
    pub routes_added: bool,
    pub storage_mutation_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_proof_verification_contract_boundary_posture(
) -> NativeProofVerificationContractBoundaryPosture {
    NativeProofVerificationContractBoundaryPosture {
        phase_label: NATIVE_PASSPORT_PHASE10A_LABEL,
        proof_verification_contract_boundary_added: true,
        cryptographic_signature_verification_runtime_added: false,
        request_proof_verification_runtime_added: false,
        service_challenge_verification_added: false,
        replay_store_mutation_added: false,
        challenge_consumption_added: false,
        capability_consumption_added: false,
        capability_issuance_added: false,
        capability_lifecycle_runtime_added: false,
        secret_key_access_added: false,
        key_loading_added: false,
        key_derivation_runtime_added: false,
        vault_unlock_added: false,
        vault_runtime_added: false,
        platform_sealer_unseal_added: false,
        runtime_io_added: false,
        routes_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE10A_FORBIDDEN_PROOF_VERIFICATION_AUTHORITY_FLAGS,
    }
}

pub fn review_native_proof_verification_contract_boundary(
    envelope: &NativeProofSigningAdapterEnvelopeV1,
    draft: NativeProofVerificationContractBoundaryDraftV1,
) -> Result<
    NativeProofVerificationContractBoundaryDecisionV1,
    NativeProofVerificationContractBoundaryReviewError,
> {
    validate_signing_envelope(envelope)?;

    if draft.contract_domain != PHASE10A_PROOF_VERIFICATION_BOUNDARY_DOMAIN {
        return Err(NativeProofVerificationContractBoundaryReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE10A_PROOF_VERIFICATION_BOUNDARY_VERSION {
        return Err(NativeProofVerificationContractBoundaryReviewError::ContractVersionMismatch);
    }

    validate_draft_against_envelope(envelope, &draft)?;
    validate_no_unsafe_flags(&draft)?;

    Ok(NativeProofVerificationContractBoundaryDecisionV1 {
        contract_domain: PHASE10A_PROOF_VERIFICATION_BOUNDARY_DOMAIN,
        contract_version: PHASE10A_PROOF_VERIFICATION_BOUNDARY_VERSION,
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
        cryptographic_verification_performed: false,
        replay_state_mutated: false,
        challenge_consumed: false,
        capability_state_changed: false,
        runtime_io_performed: false,
        wallet_or_ledger_mutated: false,
        contract_only: true,
    })
}

fn validate_signing_envelope(
    envelope: &NativeProofSigningAdapterEnvelopeV1,
) -> Result<(), NativeProofVerificationContractBoundaryReviewError> {
    if envelope.contract_domain != PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN {
        return Err(
            NativeProofVerificationContractBoundaryReviewError::SigningEnvelopeDomainMismatch,
        );
    }

    if envelope.contract_version != PHASE9B_PROOF_SIGNING_ADAPTER_VERSION {
        return Err(
            NativeProofVerificationContractBoundaryReviewError::SigningEnvelopeVersionMismatch,
        );
    }

    if !envelope.signed_payload_present {
        return Err(NativeProofVerificationContractBoundaryReviewError::SigningEnvelopeUnsigned);
    }

    if envelope.secret_material_exposed
        || envelope.runtime_io_performed
        || envelope.wallet_or_ledger_mutated
    {
        return Err(NativeProofVerificationContractBoundaryReviewError::SigningEnvelopeUnsafe);
    }

    Ok(())
}

fn validate_draft_against_envelope(
    envelope: &NativeProofSigningAdapterEnvelopeV1,
    draft: &NativeProofVerificationContractBoundaryDraftV1,
) -> Result<(), NativeProofVerificationContractBoundaryReviewError> {
    if draft.operation_kind != envelope.operation_kind {
        return Err(NativeProofVerificationContractBoundaryReviewError::OperationMismatch);
    }

    if draft.expected_authority != envelope.signer_authority {
        return Err(NativeProofVerificationContractBoundaryReviewError::AuthorityMismatch);
    }

    if draft.challenge_id != envelope.challenge_id {
        return Err(NativeProofVerificationContractBoundaryReviewError::ChallengeIdMismatch);
    }

    if draft.passport_id != envelope.passport_id {
        return Err(NativeProofVerificationContractBoundaryReviewError::PassportBindingMismatch);
    }

    if draft.device_id != envelope.device_id {
        return Err(NativeProofVerificationContractBoundaryReviewError::DeviceBindingMismatch);
    }

    if draft.proof_contract_domain != envelope.proof_contract_domain
        || draft.proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
    {
        return Err(
            NativeProofVerificationContractBoundaryReviewError::ProofContractDomainMismatch,
        );
    }

    if draft.proof_contract_version != envelope.proof_contract_version
        || draft.proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
    {
        return Err(
            NativeProofVerificationContractBoundaryReviewError::ProofContractVersionMismatch,
        );
    }

    if draft.request_contract_domain != envelope.request_contract_domain {
        return Err(
            NativeProofVerificationContractBoundaryReviewError::RequestProofContractDomainMismatch,
        );
    }

    if draft.request_contract_version != envelope.request_contract_version {
        return Err(
            NativeProofVerificationContractBoundaryReviewError::RequestProofContractVersionMismatch,
        );
    }

    if let Some(domain) = draft.request_contract_domain {
        if domain != PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN {
            return Err(
                NativeProofVerificationContractBoundaryReviewError::RequestProofContractDomainMismatch,
            );
        }
    }

    if let Some(version) = draft.request_contract_version {
        if version != PHASE8C_PROOF_REQUEST_CONTRACT_VERSION {
            return Err(
                NativeProofVerificationContractBoundaryReviewError::RequestProofContractVersionMismatch,
            );
        }
    }

    if draft.proof_kind != envelope.proof_kind {
        return Err(NativeProofVerificationContractBoundaryReviewError::ProofKindMismatch);
    }

    if draft.transcript_hash != envelope.transcript_hash {
        return Err(NativeProofVerificationContractBoundaryReviewError::TranscriptHashMismatch);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(
            NativeProofVerificationContractBoundaryReviewError::MissingSignerPublicKeyLabel,
        );
    }

    if draft.signer_public_key_label != envelope.signer_public_key_label {
        return Err(
            NativeProofVerificationContractBoundaryReviewError::SignerPublicKeyLabelMismatch,
        );
    }

    validate_signature_algorithm(draft.expected_authority, draft.expected_signature_algorithm)?;

    if draft.expected_signature_algorithm != envelope.requested_signature_algorithm {
        return Err(NativeProofVerificationContractBoundaryReviewError::SignatureAlgorithmMismatch);
    }

    if draft.signed_payload_hex != envelope.signed_payload_hex {
        return Err(NativeProofVerificationContractBoundaryReviewError::SignedPayloadMismatch);
    }

    if !draft.expects_signed_payload_present {
        return Err(NativeProofVerificationContractBoundaryReviewError::SignedPayloadExpected);
    }

    if draft.verification_requested_at_ms < envelope.signing_requested_at_ms {
        return Err(
            NativeProofVerificationContractBoundaryReviewError::VerificationTimeBeforeSigning,
        );
    }

    Ok(())
}

fn validate_signature_algorithm(
    authority: NativePassportProofAuthority,
    algorithm: NativeProofSignatureAlgorithm,
) -> Result<(), NativeProofVerificationContractBoundaryReviewError> {
    let expected = match authority {
        NativePassportProofAuthority::RootAuthority => PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        NativePassportProofAuthority::DeviceAuthority => PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
    };

    if algorithm != expected {
        return Err(NativeProofVerificationContractBoundaryReviewError::SignatureAlgorithmMismatch);
    }

    Ok(())
}

fn validate_no_unsafe_flags(
    draft: &NativeProofVerificationContractBoundaryDraftV1,
) -> Result<(), NativeProofVerificationContractBoundaryReviewError> {
    if draft.requests_cryptographic_signature_verification_runtime
        || draft.requests_request_proof_verification_runtime
        || draft.requests_service_challenge_verification
        || draft.requests_replay_store_mutation
        || draft.requests_challenge_consumption
        || draft.requests_capability_consumption
        || draft.requests_capability_issuance
        || draft.requests_capability_lifecycle_runtime
        || draft.requests_secret_key_access
        || draft.requests_key_loading
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(
            NativeProofVerificationContractBoundaryReviewError::UnsafeVerificationAuthorityFlag,
        );
    }

    Ok(())
}
