//! RO:WHAT — Native Passport root/device proof contract DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines proof metadata and challenge linkage after Phase 8A purpose-bound challenges, before proof signing or verification runtime exists.
//! RO:INTERACTS — Phase 8A challenge contract DTOs, shared Passport/Device/Challenge ID DTOs, future ron-auth canonical proof transcript builders.
//! RO:INVARIANTS — Proof DTOs carry public contract metadata only: domain, version, challenge binding, purpose, proof kind, authority label, scope, transcript hashes, public signer label, signature placeholders, and timing. This phase adds no root signing, device signing, proof verification, replay-store mutation, challenge consumption, capability issuance, runtime I/O, routes, storage, wallet, ledger, vault, PIN, KDF, encryption, or secret custody.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects domain/version drift, challenge linkage drift, purpose/body/scope binding drift, wrong authority for purpose, wrong proof kind for purpose, non-canonical transcript posture, missing placeholders, invalid proof time, non-contract descriptors, and flags implying runtime authority.
//! RO:TEST — tests/native_passport_phase8b_proof_contract_dto.rs.

use super::{
    validate_native_passport_challenge_contract_descriptor, B3DigestHex, ChallengeIdV1, DeviceIdV1,
    NativeChallengeTranscriptCodec, NativePassportChallengeContractDescriptorV1,
    NativePassportChallengePurpose, NativePassportScope, PassportIdV1,
    PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
};

/// Phase label for native root/device proof contract DTO foundations.
pub const NATIVE_PASSPORT_PHASE8B_LABEL: &str = "NATIVE_PASSPORT_PHASE8B_PROOF_CONTRACT_DTO";

/// Canonical proof contract DTO domain.
pub const PHASE8B_PROOF_CONTRACT_DOMAIN: &str = "native-passport/proof-contract/v1";

/// Proof contract version.
pub const PHASE8B_PROOF_CONTRACT_VERSION: u16 = 1;

/// Required proof transcript codec.
pub const PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC: NativeChallengeTranscriptCodec =
    NativeChallengeTranscriptCodec::CanonicalB3V1;

/// Root proof signature algorithm label.
pub const PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM: NativeProofSignatureAlgorithm =
    NativeProofSignatureAlgorithm::RootEd25519V1;

/// Device proof signature algorithm label.
pub const PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM: NativeProofSignatureAlgorithm =
    NativeProofSignatureAlgorithm::DeviceEd25519V1;

/// Proof authority meanings that Phase 8B DTOs must not grant.
pub const PHASE8B_FORBIDDEN_PROOF_AUTHORITY_FLAGS: &[&str] = &[
    "root_proof_signing",
    "device_proof_signing",
    "proof_signature_verification",
    "request_proof_signing",
    "request_proof_verification",
    "service_challenge_signing",
    "service_challenge_verification",
    "replay_store_mutation",
    "challenge_consumption",
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

/// Proof kind labels for Phase 8B contract DTOs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativePassportProofKind {
    /// Root registration proof.
    RootRegistration,
    /// Root device authorization proof.
    RootDeviceAuthorization,
    /// Root device revocation proof.
    RootDeviceRevocation,
    /// Device session proof.
    DeviceSession,
    /// Device capability proof.
    DeviceCapability,
    /// Device namespace proof.
    NamespaceTransition,
}

/// Authority label expected to sign a future proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativePassportProofAuthority {
    /// Root authority is required.
    RootAuthority,
    /// Device authority is required.
    DeviceAuthority,
}

/// Proof signature algorithm label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeProofSignatureAlgorithm {
    /// Root Ed25519 proof signature label.
    RootEd25519V1,
    /// Device Ed25519 proof signature label.
    DeviceEd25519V1,
    /// Explicitly rejected placeholder for missing/unknown algorithm.
    MissingOrUnknown,
}

/// Contract-only descriptor for a root/device proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportProofContractDescriptorV1 {
    /// Proof contract domain.
    pub contract_domain: &'static str,
    /// Proof contract version.
    pub contract_version: u16,
    /// Bound challenge contract domain.
    pub challenge_contract_domain: &'static str,
    /// Bound challenge contract version.
    pub challenge_contract_version: u16,
    /// Bound challenge ID.
    pub challenge_id: ChallengeIdV1,
    /// Challenge purpose.
    pub purpose: NativePassportChallengePurpose,
    /// Proof kind label.
    pub proof_kind: NativePassportProofKind,
    /// Expected proof authority label.
    pub proof_authority: NativePassportProofAuthority,
    /// Requested scope list copied from the challenge.
    pub requested_scopes: Vec<NativePassportScope>,
    /// Passport binding copied from the challenge.
    pub passport_id: Option<PassportIdV1>,
    /// Device binding copied from the challenge.
    pub device_id: Option<DeviceIdV1>,
    /// Operation body hash binding copied from the challenge.
    pub operation_body_hash: Option<B3DigestHex>,
    /// Canonical challenge transcript hash label.
    pub challenge_transcript_hash: B3DigestHex,
    /// Canonical proof transcript hash label.
    pub proof_transcript_hash: B3DigestHex,
    /// Public signer key label, not secret material.
    pub signer_public_key_label: &'static str,
    /// Proof transcript codec.
    pub proof_transcript_codec: NativeChallengeTranscriptCodec,
    /// Proof signature algorithm label.
    pub proof_signature_algorithm: NativeProofSignatureAlgorithm,
    /// Whether the service challenge placeholder is present.
    pub service_challenge_placeholder_present: bool,
    /// Whether the proof signature placeholder is present.
    pub proof_signature_placeholder_present: bool,
    /// Challenge issued-at timestamp.
    pub challenge_issued_at_ms: u64,
    /// Challenge expiry timestamp.
    pub challenge_expires_at_ms: u64,
    /// Proof creation timestamp.
    pub proof_created_at_ms: u64,
    /// Whether this descriptor is contract-only.
    pub contract_only: bool,
}

/// Contract-only draft for a root/device proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportProofContractDraftV1 {
    /// Proof contract domain.
    pub contract_domain: &'static str,
    /// Proof contract version.
    pub contract_version: u16,
    /// Bound challenge contract domain.
    pub challenge_contract_domain: &'static str,
    /// Bound challenge contract version.
    pub challenge_contract_version: u16,
    /// Bound challenge ID.
    pub challenge_id: ChallengeIdV1,
    /// Challenge purpose.
    pub purpose: NativePassportChallengePurpose,
    /// Proof kind label.
    pub proof_kind: NativePassportProofKind,
    /// Expected proof authority label.
    pub proof_authority: NativePassportProofAuthority,
    /// Requested scope list copied from the challenge.
    pub requested_scopes: Vec<NativePassportScope>,
    /// Passport binding copied from the challenge.
    pub passport_id: Option<PassportIdV1>,
    /// Device binding copied from the challenge.
    pub device_id: Option<DeviceIdV1>,
    /// Operation body hash binding copied from the challenge.
    pub operation_body_hash: Option<B3DigestHex>,
    /// Canonical challenge transcript hash label.
    pub challenge_transcript_hash: B3DigestHex,
    /// Canonical proof transcript hash label.
    pub proof_transcript_hash: B3DigestHex,
    /// Public signer key label, not secret material.
    pub signer_public_key_label: &'static str,
    /// Proof transcript codec.
    pub proof_transcript_codec: NativeChallengeTranscriptCodec,
    /// Proof signature algorithm label.
    pub proof_signature_algorithm: NativeProofSignatureAlgorithm,
    /// Whether the service challenge placeholder is present.
    pub service_challenge_placeholder_present: bool,
    /// Whether the proof signature placeholder is present.
    pub proof_signature_placeholder_present: bool,
    /// Proof creation timestamp.
    pub proof_created_at_ms: u64,
    /// Boundary flag: this DTO must not sign root proofs.
    pub requests_root_proof_signing: bool,
    /// Boundary flag: this DTO must not sign device proofs.
    pub requests_device_proof_signing: bool,
    /// Boundary flag: this DTO must not verify proof signatures.
    pub requests_proof_signature_verification: bool,
    /// Boundary flag: this DTO must not sign request proofs.
    pub requests_request_proof_signing: bool,
    /// Boundary flag: this DTO must not verify request proofs.
    pub requests_request_proof_verification: bool,
    /// Boundary flag: this DTO must not create service challenge signatures.
    pub requests_service_challenge_signing: bool,
    /// Boundary flag: this DTO must not verify service challenge signatures.
    pub requests_service_challenge_verification: bool,
    /// Boundary flag: this DTO must not mutate replay stores.
    pub requests_replay_store_mutation: bool,
    /// Boundary flag: this DTO must not consume challenges.
    pub requests_challenge_consumption: bool,
    /// Boundary flag: this DTO must not issue capabilities.
    pub requests_capability_issuance: bool,
    /// Boundary flag: this DTO must not refresh or revoke capabilities.
    pub requests_capability_lifecycle_runtime: bool,
    /// Boundary flag: this DTO must not execute device authorization/revocation.
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

/// Phase 8B proof contract review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePassportProofContractReviewError {
    /// Proof contract domain must match.
    ContractDomainMismatch,
    /// Proof contract version must match.
    ContractVersionMismatch,
    /// Referenced challenge contract failed validation.
    ChallengeContractReferenceInvalid,
    /// Challenge contract domain must match.
    ChallengeContractDomainMismatch,
    /// Challenge contract version must match.
    ChallengeContractVersionMismatch,
    /// Challenge ID must match.
    ChallengeIdMismatch,
    /// Purpose must match.
    PurposeMismatch,
    /// Requested scopes must match.
    RequestedScopeMismatch,
    /// Passport binding must match.
    PassportBindingMismatch,
    /// Device binding must match.
    DeviceBindingMismatch,
    /// Operation body hash binding must match.
    OperationBodyHashMismatch,
    /// Proof kind must match the purpose.
    InvalidProofKindForPurpose,
    /// Proof authority must match the purpose.
    InvalidProofAuthorityForPurpose,
    /// Signer public key label must be present.
    MissingSignerPublicKeyLabel,
    /// Proof transcript codec must be canonical.
    InvalidProofTranscriptCodec,
    /// Proof signature algorithm must match authority.
    InvalidProofSignatureAlgorithm,
    /// Service challenge placeholder must be present.
    MissingServiceChallengePlaceholder,
    /// Proof signature placeholder must be present.
    MissingProofSignaturePlaceholder,
    /// Proof creation time must fall inside challenge validity.
    InvalidProofCreatedAt,
    /// Proof descriptors must remain contract-only.
    ProofContractNotContractOnly,
    /// DTO flags attempted to carry or exercise unsafe proof authority.
    UnsafeProofAuthorityFlag,
}

/// Phase 8B posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportProofContractPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds proof contract DTOs.
    pub proof_contract_dtos_added: bool,
    /// Whether this phase signs root proofs.
    pub root_proof_signing_added: bool,
    /// Whether this phase signs device proofs.
    pub device_proof_signing_added: bool,
    /// Whether this phase verifies proof signatures.
    pub proof_signature_verification_added: bool,
    /// Whether this phase signs or verifies request proofs.
    pub request_proof_runtime_added: bool,
    /// Whether this phase signs service challenges.
    pub service_challenge_signing_added: bool,
    /// Whether this phase verifies service challenges.
    pub service_challenge_verification_added: bool,
    /// Whether this phase mutates replay stores.
    pub replay_store_mutation_added: bool,
    /// Whether this phase consumes challenges.
    pub challenge_consumption_added: bool,
    /// Whether this phase issues capabilities.
    pub capability_issuance_added: bool,
    /// Whether this phase adds capability lifecycle runtime.
    pub capability_lifecycle_runtime_added: bool,
    /// Whether this phase executes device authorization/revocation.
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

/// Return Phase 8B proof contract posture.
pub fn native_passport_proof_contract_posture() -> NativePassportProofContractPosture {
    NativePassportProofContractPosture {
        phase_label: NATIVE_PASSPORT_PHASE8B_LABEL,
        proof_contract_dtos_added: true,
        root_proof_signing_added: false,
        device_proof_signing_added: false,
        proof_signature_verification_added: false,
        request_proof_runtime_added: false,
        service_challenge_signing_added: false,
        service_challenge_verification_added: false,
        replay_store_mutation_added: false,
        challenge_consumption_added: false,
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
        forbidden_authority_flags: PHASE8B_FORBIDDEN_PROOF_AUTHORITY_FLAGS,
    }
}

/// Validate a proof descriptor against a reviewed challenge descriptor.
pub fn validate_native_passport_proof_contract_descriptor(
    challenge: &NativePassportChallengeContractDescriptorV1,
    descriptor: &NativePassportProofContractDescriptorV1,
) -> Result<(), NativePassportProofContractReviewError> {
    validate_proof_fields(
        challenge,
        descriptor.contract_domain,
        descriptor.contract_version,
        descriptor.challenge_contract_domain,
        descriptor.challenge_contract_version,
        &descriptor.challenge_id,
        descriptor.purpose,
        descriptor.proof_kind,
        descriptor.proof_authority,
        &descriptor.requested_scopes,
        descriptor.passport_id.as_ref(),
        descriptor.device_id.as_ref(),
        descriptor.operation_body_hash.as_ref(),
        descriptor.signer_public_key_label,
        descriptor.proof_transcript_codec,
        descriptor.proof_signature_algorithm,
        descriptor.service_challenge_placeholder_present,
        descriptor.proof_signature_placeholder_present,
        descriptor.proof_created_at_ms,
    )?;

    if !descriptor.contract_only {
        return Err(NativePassportProofContractReviewError::ProofContractNotContractOnly);
    }

    Ok(())
}

/// Review a proof draft into a contract-only descriptor.
///
/// This is proof DTO review only. It does not sign root proofs, sign device
/// proofs, verify proof signatures, verify request proofs, verify service
/// challenge signatures, mutate replay stores, consume challenges, issue
/// capabilities, execute device or namespace transitions, derive keys, validate
/// PINs, unlock vaults, store secrets, contact live RPC, add routes, or mutate
/// wallet/ledger state.
pub fn review_native_passport_proof_contract_draft(
    challenge: &NativePassportChallengeContractDescriptorV1,
    draft: NativePassportProofContractDraftV1,
) -> Result<NativePassportProofContractDescriptorV1, NativePassportProofContractReviewError> {
    validate_proof_fields(
        challenge,
        draft.contract_domain,
        draft.contract_version,
        draft.challenge_contract_domain,
        draft.challenge_contract_version,
        &draft.challenge_id,
        draft.purpose,
        draft.proof_kind,
        draft.proof_authority,
        &draft.requested_scopes,
        draft.passport_id.as_ref(),
        draft.device_id.as_ref(),
        draft.operation_body_hash.as_ref(),
        draft.signer_public_key_label,
        draft.proof_transcript_codec,
        draft.proof_signature_algorithm,
        draft.service_challenge_placeholder_present,
        draft.proof_signature_placeholder_present,
        draft.proof_created_at_ms,
    )?;

    if draft.requests_root_proof_signing
        || draft.requests_device_proof_signing
        || draft.requests_proof_signature_verification
        || draft.requests_request_proof_signing
        || draft.requests_request_proof_verification
        || draft.requests_service_challenge_signing
        || draft.requests_service_challenge_verification
        || draft.requests_replay_store_mutation
        || draft.requests_challenge_consumption
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
        return Err(NativePassportProofContractReviewError::UnsafeProofAuthorityFlag);
    }

    Ok(NativePassportProofContractDescriptorV1 {
        contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
        contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
        challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
        challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
        challenge_id: draft.challenge_id,
        purpose: draft.purpose,
        proof_kind: draft.proof_kind,
        proof_authority: draft.proof_authority,
        requested_scopes: draft.requested_scopes,
        passport_id: draft.passport_id,
        device_id: draft.device_id,
        operation_body_hash: draft.operation_body_hash,
        challenge_transcript_hash: draft.challenge_transcript_hash,
        proof_transcript_hash: draft.proof_transcript_hash,
        signer_public_key_label: draft.signer_public_key_label,
        proof_transcript_codec: PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC,
        proof_signature_algorithm: draft.proof_signature_algorithm,
        service_challenge_placeholder_present: true,
        proof_signature_placeholder_present: true,
        challenge_issued_at_ms: challenge.issued_at_ms,
        challenge_expires_at_ms: challenge.expires_at_ms,
        proof_created_at_ms: draft.proof_created_at_ms,
        contract_only: true,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_proof_fields(
    challenge: &NativePassportChallengeContractDescriptorV1,
    contract_domain: &'static str,
    contract_version: u16,
    challenge_contract_domain: &'static str,
    challenge_contract_version: u16,
    challenge_id: &ChallengeIdV1,
    purpose: NativePassportChallengePurpose,
    proof_kind: NativePassportProofKind,
    proof_authority: NativePassportProofAuthority,
    requested_scopes: &[NativePassportScope],
    passport_id: Option<&PassportIdV1>,
    device_id: Option<&DeviceIdV1>,
    operation_body_hash: Option<&B3DigestHex>,
    signer_public_key_label: &'static str,
    proof_transcript_codec: NativeChallengeTranscriptCodec,
    proof_signature_algorithm: NativeProofSignatureAlgorithm,
    service_challenge_placeholder_present: bool,
    proof_signature_placeholder_present: bool,
    proof_created_at_ms: u64,
) -> Result<(), NativePassportProofContractReviewError> {
    validate_native_passport_challenge_contract_descriptor(challenge)
        .map_err(|_| NativePassportProofContractReviewError::ChallengeContractReferenceInvalid)?;

    if contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN {
        return Err(NativePassportProofContractReviewError::ContractDomainMismatch);
    }

    if contract_version != PHASE8B_PROOF_CONTRACT_VERSION {
        return Err(NativePassportProofContractReviewError::ContractVersionMismatch);
    }

    if challenge_contract_domain != PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN
        || challenge_contract_domain != challenge.contract_domain
    {
        return Err(NativePassportProofContractReviewError::ChallengeContractDomainMismatch);
    }

    if challenge_contract_version != PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION
        || challenge_contract_version != challenge.contract_version
    {
        return Err(NativePassportProofContractReviewError::ChallengeContractVersionMismatch);
    }

    if challenge_id != &challenge.challenge_id {
        return Err(NativePassportProofContractReviewError::ChallengeIdMismatch);
    }

    if purpose != challenge.purpose {
        return Err(NativePassportProofContractReviewError::PurposeMismatch);
    }

    if requested_scopes != challenge.requested_scopes.as_slice() {
        return Err(NativePassportProofContractReviewError::RequestedScopeMismatch);
    }

    if passport_id != challenge.passport_id.as_ref() {
        return Err(NativePassportProofContractReviewError::PassportBindingMismatch);
    }

    if device_id != challenge.device_id.as_ref() {
        return Err(NativePassportProofContractReviewError::DeviceBindingMismatch);
    }

    if operation_body_hash != challenge.operation_body_hash.as_ref() {
        return Err(NativePassportProofContractReviewError::OperationBodyHashMismatch);
    }

    if expected_proof_kind_for_purpose(purpose) != proof_kind {
        return Err(NativePassportProofContractReviewError::InvalidProofKindForPurpose);
    }

    let expected_authority = expected_proof_authority_for_purpose(purpose);
    if expected_authority != proof_authority {
        return Err(NativePassportProofContractReviewError::InvalidProofAuthorityForPurpose);
    }

    if signer_public_key_label.is_empty() {
        return Err(NativePassportProofContractReviewError::MissingSignerPublicKeyLabel);
    }

    if proof_transcript_codec != PHASE8B_REQUIRED_PROOF_TRANSCRIPT_CODEC {
        return Err(NativePassportProofContractReviewError::InvalidProofTranscriptCodec);
    }

    if proof_signature_algorithm != expected_signature_algorithm_for_authority(expected_authority) {
        return Err(NativePassportProofContractReviewError::InvalidProofSignatureAlgorithm);
    }

    if !service_challenge_placeholder_present {
        return Err(NativePassportProofContractReviewError::MissingServiceChallengePlaceholder);
    }

    if !proof_signature_placeholder_present {
        return Err(NativePassportProofContractReviewError::MissingProofSignaturePlaceholder);
    }

    if proof_created_at_ms < challenge.issued_at_ms || proof_created_at_ms > challenge.expires_at_ms
    {
        return Err(NativePassportProofContractReviewError::InvalidProofCreatedAt);
    }

    Ok(())
}

fn expected_proof_kind_for_purpose(
    purpose: NativePassportChallengePurpose,
) -> NativePassportProofKind {
    match purpose {
        NativePassportChallengePurpose::RegisterRoot => NativePassportProofKind::RootRegistration,
        NativePassportChallengePurpose::AuthorizeDevice => {
            NativePassportProofKind::RootDeviceAuthorization
        }
        NativePassportChallengePurpose::RevokeDevice => {
            NativePassportProofKind::RootDeviceRevocation
        }
        NativePassportChallengePurpose::ProveSession => NativePassportProofKind::DeviceSession,
        NativePassportChallengePurpose::IssueCapability
        | NativePassportChallengePurpose::RefreshCapability => {
            NativePassportProofKind::DeviceCapability
        }
        NativePassportChallengePurpose::ClaimUsername
        | NativePassportChallengePurpose::TransferUsername
        | NativePassportChallengePurpose::ReleaseUsername
        | NativePassportChallengePurpose::PublishProfile
        | NativePassportChallengePurpose::RegisterSite
        | NativePassportChallengePurpose::UpdateSite => {
            NativePassportProofKind::NamespaceTransition
        }
        NativePassportChallengePurpose::WalletAuthorizationRequestReserved => {
            NativePassportProofKind::DeviceSession
        }
    }
}

fn expected_proof_authority_for_purpose(
    purpose: NativePassportChallengePurpose,
) -> NativePassportProofAuthority {
    match purpose {
        NativePassportChallengePurpose::RegisterRoot
        | NativePassportChallengePurpose::AuthorizeDevice
        | NativePassportChallengePurpose::RevokeDevice => {
            NativePassportProofAuthority::RootAuthority
        }
        NativePassportChallengePurpose::ProveSession
        | NativePassportChallengePurpose::IssueCapability
        | NativePassportChallengePurpose::RefreshCapability
        | NativePassportChallengePurpose::ClaimUsername
        | NativePassportChallengePurpose::TransferUsername
        | NativePassportChallengePurpose::ReleaseUsername
        | NativePassportChallengePurpose::PublishProfile
        | NativePassportChallengePurpose::RegisterSite
        | NativePassportChallengePurpose::UpdateSite
        | NativePassportChallengePurpose::WalletAuthorizationRequestReserved => {
            NativePassportProofAuthority::DeviceAuthority
        }
    }
}

fn expected_signature_algorithm_for_authority(
    authority: NativePassportProofAuthority,
) -> NativeProofSignatureAlgorithm {
    match authority {
        NativePassportProofAuthority::RootAuthority => PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        NativePassportProofAuthority::DeviceAuthority => PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
    }
}
