//! RO:WHAT — Native Passport request-proof contract DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines request proof metadata after Phase 8A challenges and Phase 8B root/device proof DTOs, before native request-proof signing or service verification runtime exists.
//! RO:INTERACTS — Phase 8A challenge descriptors, Phase 8B proof descriptors, shared Passport/Device/Challenge ID DTOs, future ron-auth request transcript builders.
//! RO:INVARIANTS — Request-proof DTOs carry public contract metadata only: domain, version, request method/path/query/body hash, audience/environment/network, scope, challenge/proof linkage, transcript hash labels, signature placeholders, and timing. This phase adds no request-proof signing, request-proof verification, proof signature verification, replay-store mutation, challenge consumption, capability use/issuance, runtime I/O, routes, storage, wallet, ledger, vault, PIN, KDF, encryption, or secret custody.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects domain/version drift, challenge/proof reference drift, audience/environment/network drift, method/body mismatch, path/origin drift, scope outside proof, time drift, non-canonical transcript posture, missing placeholders, non-contract descriptors, and flags implying runtime authority.
//! RO:TEST — tests/native_passport_phase8c_proof_request_contract_dto.rs.

use super::{
    validate_native_passport_proof_contract_descriptor, B3DigestHex, ChallengeIdV1, DeviceIdV1,
    NativeChallengeTranscriptCodec, NativePassportChallengeContractDescriptorV1,
    NativePassportProofAuthority, NativePassportProofContractDescriptorV1, NativePassportProofKind,
    NativePassportScope, PassportIdV1, PHASE8B_PROOF_CONTRACT_DOMAIN,
    PHASE8B_PROOF_CONTRACT_VERSION, PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC,
};

/// Phase label for native request-proof contract DTO foundations.
pub const NATIVE_PASSPORT_PHASE8C_LABEL: &str =
    "NATIVE_PASSPORT_PHASE8C_PROOF_REQUEST_CONTRACT_DTO";

/// Canonical request-proof contract DTO domain.
pub const PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN: &str = "native-passport/proof-request-contract/v1";

/// Request-proof contract version.
pub const PHASE8C_PROOF_REQUEST_CONTRACT_VERSION: u16 = 1;

/// Required request transcript codec.
pub const PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC: NativeChallengeTranscriptCodec =
    NativeChallengeTranscriptCodec::CanonicalB3V1;

/// Maximum accepted request-proof DTO TTL.
pub const PHASE8C_MAX_REQUEST_TTL_MS: u64 = 120_000;

/// Maximum accepted request-proof clock skew metadata.
pub const PHASE8C_MAX_CLOCK_SKEW_MS: u64 = 30_000;

/// Allowed request methods.
pub const PHASE8C_ALLOWED_REQUEST_METHODS: &[NativePassportRequestMethod] = &[
    NativePassportRequestMethod::Get,
    NativePassportRequestMethod::Head,
    NativePassportRequestMethod::Post,
    NativePassportRequestMethod::Put,
    NativePassportRequestMethod::Patch,
    NativePassportRequestMethod::Delete,
];

/// Request-proof authority meanings that Phase 8C DTOs must not grant.
pub const PHASE8C_FORBIDDEN_REQUEST_PROOF_AUTHORITY_FLAGS: &[&str] = &[
    "request_proof_signing",
    "request_proof_verification",
    "proof_signature_verification",
    "service_challenge_verification",
    "replay_store_mutation",
    "challenge_consumption",
    "capability_consumption",
    "capability_issuance",
    "capability_refresh",
    "capability_revocation",
    "device_authorization_execution",
    "device_revocation_execution",
    "namespace_execution",
    "json_byte_signing",
    "device_key_generation",
    "key_derivation_runtime",
    "pin_validation_runtime",
    "pin_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_implementation",
    "secret_storage",
    "material_export",
    "encryption_runtime",
    "decryption_runtime",
    "live_rpc",
    "runtime_io",
    "route_added",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
];

/// HTTP-like request method labels for canonical request-proof binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativePassportRequestMethod {
    /// GET request.
    Get,
    /// HEAD request.
    Head,
    /// POST request.
    Post,
    /// PUT request.
    Put,
    /// PATCH request.
    Patch,
    /// DELETE request.
    Delete,
}

/// Contract-only descriptor for a request proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportRequestProofContractDescriptorV1 {
    /// Request-proof contract domain.
    pub contract_domain: &'static str,
    /// Request-proof contract version.
    pub contract_version: u16,
    /// Bound proof contract domain.
    pub proof_contract_domain: &'static str,
    /// Bound proof contract version.
    pub proof_contract_version: u16,
    /// Bound challenge ID.
    pub challenge_id: ChallengeIdV1,
    /// Network copied from the challenge.
    pub network_id: &'static str,
    /// Environment copied from the challenge.
    pub environment: &'static str,
    /// Audience copied from the challenge.
    pub audience: &'static str,
    /// Request target origin.
    pub request_target_origin: &'static str,
    /// Request method.
    pub request_method: NativePassportRequestMethod,
    /// Canonical request path.
    pub request_path: &'static str,
    /// Optional canonical query hash.
    pub request_query_hash: Option<B3DigestHex>,
    /// Optional canonical body hash.
    pub request_body_hash: Option<B3DigestHex>,
    /// Requested scopes for this request.
    pub requested_scopes: Vec<NativePassportScope>,
    /// Passport binding copied from proof/challenge.
    pub passport_id: Option<PassportIdV1>,
    /// Device binding copied from proof/challenge.
    pub device_id: Option<DeviceIdV1>,
    /// Proof kind copied from proof.
    pub proof_kind: NativePassportProofKind,
    /// Proof authority copied from proof.
    pub proof_authority: NativePassportProofAuthority,
    /// Canonical proof transcript hash copied from proof.
    pub proof_transcript_hash: B3DigestHex,
    /// Canonical request transcript hash.
    pub request_transcript_hash: B3DigestHex,
    /// Public request nonce hash.
    pub request_nonce_hash: B3DigestHex,
    /// Request transcript codec.
    pub request_transcript_codec: NativeChallengeTranscriptCodec,
    /// Whether the proof signature placeholder is present.
    pub proof_signature_placeholder_present: bool,
    /// Request-proof creation timestamp.
    pub request_created_at_ms: u64,
    /// Request-proof expiry timestamp.
    pub request_expires_at_ms: u64,
    /// Allowed request-proof clock skew metadata.
    pub max_clock_skew_ms: u64,
    /// Whether this descriptor is contract-only.
    pub contract_only: bool,
}

/// Contract-only draft for a request proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportRequestProofContractDraftV1 {
    /// Request-proof contract domain.
    pub contract_domain: &'static str,
    /// Request-proof contract version.
    pub contract_version: u16,
    /// Bound proof contract domain.
    pub proof_contract_domain: &'static str,
    /// Bound proof contract version.
    pub proof_contract_version: u16,
    /// Bound challenge ID.
    pub challenge_id: ChallengeIdV1,
    /// Network copied from the challenge.
    pub network_id: &'static str,
    /// Environment copied from the challenge.
    pub environment: &'static str,
    /// Audience copied from the challenge.
    pub audience: &'static str,
    /// Request target origin.
    pub request_target_origin: &'static str,
    /// Request method.
    pub request_method: NativePassportRequestMethod,
    /// Canonical request path.
    pub request_path: &'static str,
    /// Optional canonical query hash.
    pub request_query_hash: Option<B3DigestHex>,
    /// Optional canonical body hash.
    pub request_body_hash: Option<B3DigestHex>,
    /// Requested scopes for this request.
    pub requested_scopes: Vec<NativePassportScope>,
    /// Passport binding copied from proof/challenge.
    pub passport_id: Option<PassportIdV1>,
    /// Device binding copied from proof/challenge.
    pub device_id: Option<DeviceIdV1>,
    /// Proof kind copied from proof.
    pub proof_kind: NativePassportProofKind,
    /// Proof authority copied from proof.
    pub proof_authority: NativePassportProofAuthority,
    /// Canonical proof transcript hash copied from proof.
    pub proof_transcript_hash: B3DigestHex,
    /// Canonical request transcript hash.
    pub request_transcript_hash: B3DigestHex,
    /// Public request nonce hash.
    pub request_nonce_hash: B3DigestHex,
    /// Request transcript codec.
    pub request_transcript_codec: NativeChallengeTranscriptCodec,
    /// Whether the proof signature placeholder is present.
    pub proof_signature_placeholder_present: bool,
    /// Request-proof creation timestamp.
    pub request_created_at_ms: u64,
    /// Request-proof expiry timestamp.
    pub request_expires_at_ms: u64,
    /// Allowed request-proof clock skew metadata.
    pub max_clock_skew_ms: u64,
    /// Boundary flag: this DTO must not sign request proofs.
    pub requests_request_proof_signing: bool,
    /// Boundary flag: this DTO must not verify request proofs.
    pub requests_request_proof_verification: bool,
    /// Boundary flag: this DTO must not verify proof signatures.
    pub requests_proof_signature_verification: bool,
    /// Boundary flag: this DTO must not verify service challenges.
    pub requests_service_challenge_verification: bool,
    /// Boundary flag: this DTO must not mutate replay stores.
    pub requests_replay_store_mutation: bool,
    /// Boundary flag: this DTO must not consume challenges.
    pub requests_challenge_consumption: bool,
    /// Boundary flag: this DTO must not consume capabilities.
    pub requests_capability_consumption: bool,
    /// Boundary flag: this DTO must not issue capabilities.
    pub requests_capability_issuance: bool,
    /// Boundary flag: this DTO must not refresh or revoke capabilities.
    pub requests_capability_lifecycle_runtime: bool,
    /// Boundary flag: this DTO must not execute device authority changes.
    pub requests_device_authority_execution: bool,
    /// Boundary flag: this DTO must not execute namespace transitions.
    pub requests_namespace_execution: bool,
    /// Boundary flag: this DTO must not sign ordinary JSON bytes.
    pub requests_json_byte_signing: bool,
    /// Boundary flag: this DTO must not generate device keys.
    pub requests_device_key_generation: bool,
    /// Boundary flag: this DTO must not derive keys.
    pub requests_key_derivation_runtime: bool,
    /// Boundary flag: this DTO must not validate PIN values.
    pub requests_pin_validation_runtime: bool,
    /// Boundary flag: this DTO must not derive PIN keys.
    pub requests_pin_derivation_runtime: bool,
    /// Boundary flag: this DTO must not unlock vaults.
    pub requests_vault_unlock: bool,
    /// Boundary flag: this DTO must not include vault runtime.
    pub includes_vault_runtime: bool,
    /// Boundary flag: this DTO must not include platform sealer implementation.
    pub includes_platform_sealer_implementation: bool,
    /// Boundary flag: this DTO must not store secrets.
    pub stores_secret_material: bool,
    /// Boundary flag: this DTO must not export secret/material.
    pub exports_secret_or_material: bool,
    /// Boundary flag: this DTO must not encrypt or decrypt data.
    pub requests_encryption_or_decryption: bool,
    /// Boundary flag: this DTO must not contact live RPC.
    pub requests_live_rpc: bool,
    /// Boundary flag: this DTO must not perform runtime I/O.
    pub requests_runtime_io: bool,
    /// Boundary flag: this DTO must not add routes.
    pub adds_routes: bool,
    /// Boundary flag: this DTO must not mutate storage.
    pub requests_storage_mutation: bool,
    /// Boundary flag: this DTO must not mutate wallet or ledger state.
    pub requests_wallet_or_ledger_mutation: bool,
}

/// Phase 8C request-proof contract review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePassportRequestProofContractReviewError {
    /// Request-proof contract domain must match.
    ContractDomainMismatch,
    /// Request-proof contract version must match.
    ContractVersionMismatch,
    /// Referenced proof contract failed validation.
    ProofContractReferenceInvalid,
    /// Proof contract domain must match.
    ProofContractDomainMismatch,
    /// Proof contract version must match.
    ProofContractVersionMismatch,
    /// Challenge ID must match proof/challenge.
    ChallengeIdMismatch,
    /// Network must match challenge.
    NetworkMismatch,
    /// Environment must match challenge.
    EnvironmentMismatch,
    /// Audience must match challenge.
    AudienceMismatch,
    /// Request target origin must be present.
    EmptyRequestTargetOrigin,
    /// Request path must be present and absolute.
    InvalidRequestPath,
    /// Request scopes must not be empty.
    EmptyRequestedScopes,
    /// Duplicate request scope rejected.
    DuplicateRequestedScope,
    /// Request scope was not granted by proof.
    ScopeNotGrantedByProof,
    /// Passport binding must match proof.
    PassportBindingMismatch,
    /// Device binding must match proof.
    DeviceBindingMismatch,
    /// Proof kind must match proof.
    ProofKindMismatch,
    /// Proof authority must match proof.
    ProofAuthorityMismatch,
    /// Proof transcript hash must match proof.
    ProofTranscriptHashMismatch,
    /// Safe methods must not carry body hash.
    UnexpectedRequestBodyHash,
    /// Mutation-like methods require body hash.
    MissingRequestBodyHash,
    /// Request-proof expiry must be after creation.
    InvalidRequestWindow,
    /// Request-proof TTL exceeded Phase 8C ceiling.
    RequestTtlTooLong,
    /// Request-proof timing must stay inside proof/challenge timing.
    RequestOutsideProofWindow,
    /// Clock skew metadata exceeded Phase 8C ceiling.
    ClockSkewTooLong,
    /// Request transcript codec must be canonical.
    InvalidRequestTranscriptCodec,
    /// Proof signature placeholder must be present.
    MissingProofSignaturePlaceholder,
    /// Request-proof descriptors must remain contract-only.
    RequestProofContractNotContractOnly,
    /// DTO flags attempted to carry or exercise unsafe request-proof authority.
    UnsafeRequestProofAuthorityFlag,
}

/// Phase 8C posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportRequestProofContractPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds request-proof contract DTOs.
    pub request_proof_contract_dtos_added: bool,
    /// Whether this phase signs request proofs.
    pub request_proof_signing_added: bool,
    /// Whether this phase verifies request proofs.
    pub request_proof_verification_added: bool,
    /// Whether this phase verifies proof signatures.
    pub proof_signature_verification_added: bool,
    /// Whether this phase mutates replay stores.
    pub replay_store_mutation_added: bool,
    /// Whether this phase consumes challenges.
    pub challenge_consumption_added: bool,
    /// Whether this phase consumes capabilities.
    pub capability_consumption_added: bool,
    /// Whether this phase issues capabilities.
    pub capability_issuance_added: bool,
    /// Whether this phase adds capability lifecycle runtime.
    pub capability_lifecycle_runtime_added: bool,
    /// Whether this phase executes device authority changes.
    pub device_authority_execution_added: bool,
    /// Whether this phase executes namespace transitions.
    pub namespace_execution_added: bool,
    /// Whether this phase signs JSON bytes.
    pub json_byte_signing_added: bool,
    /// Whether this phase generates device keys.
    pub device_key_generation_added: bool,
    /// Whether this phase derives keys.
    pub key_derivation_runtime_added: bool,
    /// Whether this phase validates PINs.
    pub pin_validation_runtime_added: bool,
    /// Whether this phase derives PIN keys.
    pub pin_derivation_runtime_added: bool,
    /// Whether this phase unlocks vaults.
    pub vault_unlock_added: bool,
    /// Whether this phase adds vault runtime.
    pub vault_runtime_added: bool,
    /// Whether this phase adds platform sealer implementation.
    pub platform_sealer_implementation_added: bool,
    /// Whether this phase stores secrets.
    pub secret_storage_added: bool,
    /// Whether this phase exports material.
    pub material_export_added: bool,
    /// Whether this phase adds encryption runtime.
    pub encryption_runtime_added: bool,
    /// Whether this phase adds decryption runtime.
    pub decryption_runtime_added: bool,
    /// Whether this phase contacts live RPC.
    pub live_rpc_added: bool,
    /// Whether this phase adds runtime I/O.
    pub runtime_io_added: bool,
    /// Whether this phase adds routes.
    pub routes_added: bool,
    /// Whether this phase mutates storage.
    pub storage_mutation_added: bool,
    /// Whether this phase mutates wallet or ledger state.
    pub wallet_or_ledger_mutation_added: bool,
    /// Whether this phase changes runtime authority.
    pub runtime_authority_changed: bool,
    /// Whether this phase adds native secret custody.
    pub native_secret_implementation_added: bool,
    /// Forbidden authority meanings.
    pub forbidden_authority_flags: &'static [&'static str],
}

/// Return Phase 8C request-proof contract posture.
pub fn native_passport_request_proof_contract_posture() -> NativePassportRequestProofContractPosture
{
    NativePassportRequestProofContractPosture {
        phase_label: NATIVE_PASSPORT_PHASE8C_LABEL,
        request_proof_contract_dtos_added: true,
        request_proof_signing_added: false,
        request_proof_verification_added: false,
        proof_signature_verification_added: false,
        replay_store_mutation_added: false,
        challenge_consumption_added: false,
        capability_consumption_added: false,
        capability_issuance_added: false,
        capability_lifecycle_runtime_added: false,
        device_authority_execution_added: false,
        namespace_execution_added: false,
        json_byte_signing_added: false,
        device_key_generation_added: false,
        key_derivation_runtime_added: false,
        pin_validation_runtime_added: false,
        pin_derivation_runtime_added: false,
        vault_unlock_added: false,
        vault_runtime_added: false,
        platform_sealer_implementation_added: false,
        secret_storage_added: false,
        material_export_added: false,
        encryption_runtime_added: false,
        decryption_runtime_added: false,
        live_rpc_added: false,
        runtime_io_added: false,
        routes_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE8C_FORBIDDEN_REQUEST_PROOF_AUTHORITY_FLAGS,
    }
}

/// Validate a request-proof descriptor against reviewed challenge and proof descriptors.
pub fn validate_native_passport_request_proof_contract_descriptor(
    challenge: &NativePassportChallengeContractDescriptorV1,
    proof: &NativePassportProofContractDescriptorV1,
    descriptor: &NativePassportRequestProofContractDescriptorV1,
) -> Result<(), NativePassportRequestProofContractReviewError> {
    validate_request_proof_fields(
        challenge,
        proof,
        descriptor.contract_domain,
        descriptor.contract_version,
        descriptor.proof_contract_domain,
        descriptor.proof_contract_version,
        &descriptor.challenge_id,
        descriptor.network_id,
        descriptor.environment,
        descriptor.audience,
        descriptor.request_target_origin,
        descriptor.request_method,
        descriptor.request_path,
        descriptor.request_body_hash.as_ref(),
        &descriptor.requested_scopes,
        descriptor.passport_id.as_ref(),
        descriptor.device_id.as_ref(),
        descriptor.proof_kind,
        descriptor.proof_authority,
        &descriptor.proof_transcript_hash,
        descriptor.request_transcript_codec,
        descriptor.proof_signature_placeholder_present,
        descriptor.request_created_at_ms,
        descriptor.request_expires_at_ms,
        descriptor.max_clock_skew_ms,
    )?;

    if !descriptor.contract_only {
        return Err(
            NativePassportRequestProofContractReviewError::RequestProofContractNotContractOnly,
        );
    }

    Ok(())
}

/// Review a request-proof draft into a contract-only descriptor.
///
/// This is request-proof DTO review only. It does not sign or verify request
/// proofs, verify proof signatures, mutate replay stores, consume challenges,
/// consume or issue capabilities, execute device or namespace transitions,
/// derive keys, validate PINs, unlock vaults, store secrets, contact live RPC,
/// add routes, perform runtime I/O, or mutate wallet/ledger state.
pub fn review_native_passport_request_proof_contract_draft(
    challenge: &NativePassportChallengeContractDescriptorV1,
    proof: &NativePassportProofContractDescriptorV1,
    draft: NativePassportRequestProofContractDraftV1,
) -> Result<
    NativePassportRequestProofContractDescriptorV1,
    NativePassportRequestProofContractReviewError,
> {
    validate_request_proof_fields(
        challenge,
        proof,
        draft.contract_domain,
        draft.contract_version,
        draft.proof_contract_domain,
        draft.proof_contract_version,
        &draft.challenge_id,
        draft.network_id,
        draft.environment,
        draft.audience,
        draft.request_target_origin,
        draft.request_method,
        draft.request_path,
        draft.request_body_hash.as_ref(),
        &draft.requested_scopes,
        draft.passport_id.as_ref(),
        draft.device_id.as_ref(),
        draft.proof_kind,
        draft.proof_authority,
        &draft.proof_transcript_hash,
        draft.request_transcript_codec,
        draft.proof_signature_placeholder_present,
        draft.request_created_at_ms,
        draft.request_expires_at_ms,
        draft.max_clock_skew_ms,
    )?;

    if draft.requests_request_proof_signing
        || draft.requests_request_proof_verification
        || draft.requests_proof_signature_verification
        || draft.requests_service_challenge_verification
        || draft.requests_replay_store_mutation
        || draft.requests_challenge_consumption
        || draft.requests_capability_consumption
        || draft.requests_capability_issuance
        || draft.requests_capability_lifecycle_runtime
        || draft.requests_device_authority_execution
        || draft.requests_namespace_execution
        || draft.requests_json_byte_signing
        || draft.requests_device_key_generation
        || draft.requests_key_derivation_runtime
        || draft.requests_pin_validation_runtime
        || draft.requests_pin_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.includes_platform_sealer_implementation
        || draft.stores_secret_material
        || draft.exports_secret_or_material
        || draft.requests_encryption_or_decryption
        || draft.requests_live_rpc
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativePassportRequestProofContractReviewError::UnsafeRequestProofAuthorityFlag);
    }

    Ok(NativePassportRequestProofContractDescriptorV1 {
        contract_domain: PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN,
        contract_version: PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
        proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
        proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
        challenge_id: draft.challenge_id,
        network_id: draft.network_id,
        environment: draft.environment,
        audience: draft.audience,
        request_target_origin: draft.request_target_origin,
        request_method: draft.request_method,
        request_path: draft.request_path,
        request_query_hash: draft.request_query_hash,
        request_body_hash: draft.request_body_hash,
        requested_scopes: draft.requested_scopes,
        passport_id: draft.passport_id,
        device_id: draft.device_id,
        proof_kind: draft.proof_kind,
        proof_authority: draft.proof_authority,
        proof_transcript_hash: draft.proof_transcript_hash,
        request_transcript_hash: draft.request_transcript_hash,
        request_nonce_hash: draft.request_nonce_hash,
        request_transcript_codec: PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC,
        proof_signature_placeholder_present: true,
        request_created_at_ms: draft.request_created_at_ms,
        request_expires_at_ms: draft.request_expires_at_ms,
        max_clock_skew_ms: draft.max_clock_skew_ms,
        contract_only: true,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_request_proof_fields(
    challenge: &NativePassportChallengeContractDescriptorV1,
    proof: &NativePassportProofContractDescriptorV1,
    contract_domain: &'static str,
    contract_version: u16,
    proof_contract_domain: &'static str,
    proof_contract_version: u16,
    challenge_id: &ChallengeIdV1,
    network_id: &'static str,
    environment: &'static str,
    audience: &'static str,
    request_target_origin: &'static str,
    request_method: NativePassportRequestMethod,
    request_path: &'static str,
    request_body_hash: Option<&B3DigestHex>,
    requested_scopes: &[NativePassportScope],
    passport_id: Option<&PassportIdV1>,
    device_id: Option<&DeviceIdV1>,
    proof_kind: NativePassportProofKind,
    proof_authority: NativePassportProofAuthority,
    proof_transcript_hash: &B3DigestHex,
    request_transcript_codec: NativeChallengeTranscriptCodec,
    proof_signature_placeholder_present: bool,
    request_created_at_ms: u64,
    request_expires_at_ms: u64,
    max_clock_skew_ms: u64,
) -> Result<(), NativePassportRequestProofContractReviewError> {
    validate_native_passport_proof_contract_descriptor(challenge, proof).map_err(|_| {
        NativePassportRequestProofContractReviewError::ProofContractReferenceInvalid
    })?;

    if contract_domain != PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN {
        return Err(NativePassportRequestProofContractReviewError::ContractDomainMismatch);
    }

    if contract_version != PHASE8C_PROOF_REQUEST_CONTRACT_VERSION {
        return Err(NativePassportRequestProofContractReviewError::ContractVersionMismatch);
    }

    if proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
        || proof_contract_domain != proof.contract_domain
    {
        return Err(NativePassportRequestProofContractReviewError::ProofContractDomainMismatch);
    }

    if proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
        || proof_contract_version != proof.contract_version
    {
        return Err(NativePassportRequestProofContractReviewError::ProofContractVersionMismatch);
    }

    if challenge_id != &proof.challenge_id || challenge_id != &challenge.challenge_id {
        return Err(NativePassportRequestProofContractReviewError::ChallengeIdMismatch);
    }

    if network_id != challenge.network_id {
        return Err(NativePassportRequestProofContractReviewError::NetworkMismatch);
    }

    if environment != challenge.environment {
        return Err(NativePassportRequestProofContractReviewError::EnvironmentMismatch);
    }

    if audience != challenge.audience {
        return Err(NativePassportRequestProofContractReviewError::AudienceMismatch);
    }

    validate_request_target(request_target_origin, request_path)?;
    validate_request_body_binding(request_method, request_body_hash)?;
    validate_request_scopes(requested_scopes, &proof.requested_scopes)?;

    if passport_id != proof.passport_id.as_ref() {
        return Err(NativePassportRequestProofContractReviewError::PassportBindingMismatch);
    }

    if device_id != proof.device_id.as_ref() {
        return Err(NativePassportRequestProofContractReviewError::DeviceBindingMismatch);
    }

    if proof_kind != proof.proof_kind {
        return Err(NativePassportRequestProofContractReviewError::ProofKindMismatch);
    }

    if proof_authority != proof.proof_authority {
        return Err(NativePassportRequestProofContractReviewError::ProofAuthorityMismatch);
    }

    if proof_transcript_hash != &proof.proof_transcript_hash {
        return Err(NativePassportRequestProofContractReviewError::ProofTranscriptHashMismatch);
    }

    validate_request_time_window(
        proof,
        request_created_at_ms,
        request_expires_at_ms,
        max_clock_skew_ms,
    )?;

    if request_transcript_codec != PHASE8C_REQUIRED_REQUEST_TRANSCRIPT_CODEC
        || request_transcript_codec != PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC
    {
        return Err(NativePassportRequestProofContractReviewError::InvalidRequestTranscriptCodec);
    }

    if !proof_signature_placeholder_present {
        return Err(
            NativePassportRequestProofContractReviewError::MissingProofSignaturePlaceholder,
        );
    }

    Ok(())
}

fn validate_request_target(
    request_target_origin: &'static str,
    request_path: &'static str,
) -> Result<(), NativePassportRequestProofContractReviewError> {
    if request_target_origin.is_empty() {
        return Err(NativePassportRequestProofContractReviewError::EmptyRequestTargetOrigin);
    }

    if request_path.is_empty() || !request_path.starts_with('/') {
        return Err(NativePassportRequestProofContractReviewError::InvalidRequestPath);
    }

    Ok(())
}

fn validate_request_body_binding(
    request_method: NativePassportRequestMethod,
    request_body_hash: Option<&B3DigestHex>,
) -> Result<(), NativePassportRequestProofContractReviewError> {
    if method_forbids_body_hash(request_method) && request_body_hash.is_some() {
        return Err(NativePassportRequestProofContractReviewError::UnexpectedRequestBodyHash);
    }

    if method_requires_body_hash(request_method) && request_body_hash.is_none() {
        return Err(NativePassportRequestProofContractReviewError::MissingRequestBodyHash);
    }

    Ok(())
}

fn method_forbids_body_hash(request_method: NativePassportRequestMethod) -> bool {
    matches!(
        request_method,
        NativePassportRequestMethod::Get | NativePassportRequestMethod::Head
    )
}

fn method_requires_body_hash(request_method: NativePassportRequestMethod) -> bool {
    matches!(
        request_method,
        NativePassportRequestMethod::Post
            | NativePassportRequestMethod::Put
            | NativePassportRequestMethod::Patch
            | NativePassportRequestMethod::Delete
    )
}

fn validate_request_scopes(
    requested_scopes: &[NativePassportScope],
    proof_scopes: &[NativePassportScope],
) -> Result<(), NativePassportRequestProofContractReviewError> {
    if requested_scopes.is_empty() {
        return Err(NativePassportRequestProofContractReviewError::EmptyRequestedScopes);
    }

    for (index, scope) in requested_scopes.iter().enumerate() {
        if requested_scopes[..index].contains(scope) {
            return Err(NativePassportRequestProofContractReviewError::DuplicateRequestedScope);
        }

        if !proof_scopes.contains(scope) {
            return Err(NativePassportRequestProofContractReviewError::ScopeNotGrantedByProof);
        }
    }

    Ok(())
}

fn validate_request_time_window(
    proof: &NativePassportProofContractDescriptorV1,
    request_created_at_ms: u64,
    request_expires_at_ms: u64,
    max_clock_skew_ms: u64,
) -> Result<(), NativePassportRequestProofContractReviewError> {
    if request_expires_at_ms <= request_created_at_ms {
        return Err(NativePassportRequestProofContractReviewError::InvalidRequestWindow);
    }

    if request_expires_at_ms - request_created_at_ms > PHASE8C_MAX_REQUEST_TTL_MS {
        return Err(NativePassportRequestProofContractReviewError::RequestTtlTooLong);
    }

    if request_created_at_ms < proof.proof_created_at_ms
        || request_expires_at_ms > proof.challenge_expires_at_ms
    {
        return Err(NativePassportRequestProofContractReviewError::RequestOutsideProofWindow);
    }

    if max_clock_skew_ms > PHASE8C_MAX_CLOCK_SKEW_MS {
        return Err(NativePassportRequestProofContractReviewError::ClockSkewTooLong);
    }

    Ok(())
}
