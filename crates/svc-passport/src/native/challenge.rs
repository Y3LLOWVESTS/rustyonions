//! RO:WHAT — Native Passport purpose-bound proof challenge contract DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines signed-challenge metadata and pre-sign review before proof signing or proof verification runtime exists.
//! RO:INTERACTS — Phase 7 restore/new-device/delegated enrollment contracts, shared Passport/Device/Challenge ID DTOs, future ron-auth canonical transcript builders.
//! RO:INVARIANTS — Challenge DTOs carry public contract metadata only: domain, version, IDs, purpose, audience, network, scope, time, nonce, body-hash, signature placeholders, and transcript labels. This phase adds no service signing, service verification, root proof signing, device proof signing, proof verification, replay-store mutation, capability issuance, runtime I/O, routes, storage, wallet, ledger, vault, PIN, or secret custody.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects domain/version drift, reserved wallet purpose, missing bindings, missing operation body hash for mutation-like purposes, empty/duplicate scopes, unsafe authority flags, invalid time windows, overlong TTLs, missing service-signature placeholder, and JSON-byte signing posture.
//! RO:TEST — tests/native_passport_phase8a_proof_challenge_contract_dto.rs.

use super::{B3DigestHex, ChallengeIdV1, DeviceIdV1, NativePassportScope, PassportIdV1};

/// Phase label for native purpose-bound challenge contract DTO foundations.
pub const NATIVE_PASSPORT_PHASE8A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE8A_PROOF_CHALLENGE_CONTRACT_DTO";

/// Canonical purpose-bound challenge contract DTO domain.
pub const PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN: &str =
    "native-passport/proof-challenge-contract/v1";

/// Purpose-bound challenge contract version.
pub const PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION: u16 = 1;

/// Maximum accepted challenge TTL for this contract DTO layer.
pub const PHASE8A_MAX_CHALLENGE_TTL_MS: u64 = ron_proto::PASSPORT_CHALLENGE_V1_MAX_TTL_MS;

/// Maximum accepted clock skew metadata for this contract DTO layer.
pub const PHASE8A_MAX_CLOCK_SKEW_MS: u64 = ron_proto::PASSPORT_CHALLENGE_V1_MAX_CLOCK_SKEW_MS;

/// Required canonical transcript codec.
pub const PHASE8A_REQUIRED_TRANSCRIPT_CODEC: NativeChallengeTranscriptCodec =
    NativeChallengeTranscriptCodec::CanonicalB3V1;

/// Required service signature algorithm label.
pub const PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM: NativeChallengeSignatureAlgorithm =
    NativeChallengeSignatureAlgorithm::ServiceEd25519V1;

/// Allowed read-only challenge scopes at Phase 8A.
pub const PHASE8A_ALLOWED_CHALLENGE_SCOPES: &[NativePassportScope] = &[
    NativePassportScope::IdentityRead,
    NativePassportScope::CatalogRead,
    NativePassportScope::ContentRead,
    NativePassportScope::EntitlementRead,
    NativePassportScope::ReceiptsRead,
    NativePassportScope::ConfirmedRocRead,
    NativePassportScope::CapabilityRevokeSelf,
];

/// Challenge authority meanings that Phase 8A DTOs must not grant.
pub const PHASE8A_FORBIDDEN_CHALLENGE_AUTHORITY_FLAGS: &[&str] = &[
    "service_challenge_signing",
    "service_challenge_verification",
    "root_proof_signing",
    "device_proof_signing",
    "proof_signature_verification",
    "request_proof_signing",
    "request_proof_verification",
    "replay_store_mutation",
    "challenge_consumption",
    "capability_issuance",
    "capability_refresh",
    "capability_revocation",
    "device_authorization_execution",
    "device_revocation_execution",
    "username_claim_execution",
    "site_update_execution",
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

/// Protocol-owned closed purpose vocabulary used by the legacy Phase 8A contract adapter.
pub use ron_proto::PassportChallengePurposeV1 as NativePassportChallengePurpose;

/// Canonical transcript codec label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeChallengeTranscriptCodec {
    /// Canonical BLAKE3 transcript, never ordinary JSON bytes.
    CanonicalB3V1,
    /// Explicitly rejected JSON-byte signing posture.
    JsonBytesRejected,
}

/// Service signature algorithm label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeChallengeSignatureAlgorithm {
    /// Service Ed25519 signature algorithm label.
    ServiceEd25519V1,
    /// Explicitly rejected placeholder for missing/unknown algorithm.
    MissingOrUnknown,
}

/// Contract-only descriptor for a purpose-bound challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportChallengeContractDescriptorV1 {
    /// Challenge contract domain.
    pub contract_domain: &'static str,
    /// Challenge contract version.
    pub contract_version: u16,
    /// Validated challenge ID.
    pub challenge_id: ChallengeIdV1,
    /// Network identifier.
    pub network_id: &'static str,
    /// Environment identifier.
    pub environment: &'static str,
    /// Audience identifier.
    pub audience: &'static str,
    /// Issuing service identifier.
    pub issuing_service_id: &'static str,
    /// Service key identifier.
    pub service_key_id: &'static str,
    /// Challenge purpose.
    pub purpose: NativePassportChallengePurpose,
    /// Requested scope list.
    pub requested_scopes: Vec<NativePassportScope>,
    /// Optional Passport binding.
    pub passport_id: Option<PassportIdV1>,
    /// Optional Device binding.
    pub device_id: Option<DeviceIdV1>,
    /// Optional operation body hash binding.
    pub operation_body_hash: Option<B3DigestHex>,
    /// Public nonce digest label.
    pub nonce_hex: B3DigestHex,
    /// Issued-at timestamp in milliseconds.
    pub issued_at_ms: u64,
    /// Expiry timestamp in milliseconds.
    pub expires_at_ms: u64,
    /// Allowed clock skew metadata.
    pub max_clock_skew_ms: u64,
    /// Canonical transcript codec.
    pub transcript_codec: NativeChallengeTranscriptCodec,
    /// Service signature algorithm label.
    pub service_signature_algorithm: NativeChallengeSignatureAlgorithm,
    /// Whether a service-signature placeholder is present.
    pub service_signature_placeholder_present: bool,
    /// Whether this descriptor is contract-only.
    pub contract_only: bool,
}

/// Contract-only draft for a purpose-bound challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportChallengeContractDraftV1 {
    /// Challenge contract domain.
    pub contract_domain: &'static str,
    /// Challenge contract version.
    pub contract_version: u16,
    /// Validated challenge ID.
    pub challenge_id: ChallengeIdV1,
    /// Network identifier.
    pub network_id: &'static str,
    /// Environment identifier.
    pub environment: &'static str,
    /// Audience identifier.
    pub audience: &'static str,
    /// Issuing service identifier.
    pub issuing_service_id: &'static str,
    /// Service key identifier.
    pub service_key_id: &'static str,
    /// Challenge purpose.
    pub purpose: NativePassportChallengePurpose,
    /// Requested scope list.
    pub requested_scopes: Vec<NativePassportScope>,
    /// Optional Passport binding.
    pub passport_id: Option<PassportIdV1>,
    /// Optional Device binding.
    pub device_id: Option<DeviceIdV1>,
    /// Optional operation body hash binding.
    pub operation_body_hash: Option<B3DigestHex>,
    /// Public nonce digest label.
    pub nonce_hex: B3DigestHex,
    /// Issued-at timestamp in milliseconds.
    pub issued_at_ms: u64,
    /// Expiry timestamp in milliseconds.
    pub expires_at_ms: u64,
    /// Allowed clock skew metadata.
    pub max_clock_skew_ms: u64,
    /// Canonical transcript codec.
    pub transcript_codec: NativeChallengeTranscriptCodec,
    /// Service signature algorithm label.
    pub service_signature_algorithm: NativeChallengeSignatureAlgorithm,
    /// Whether a service-signature placeholder is present.
    pub service_signature_placeholder_present: bool,
    /// Boundary flag: this DTO must not create service signatures.
    pub requests_service_challenge_signing: bool,
    /// Boundary flag: this DTO must not verify service signatures.
    pub requests_service_challenge_verification: bool,
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
    /// Boundary flag: this DTO must not mutate replay state.
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

/// Phase 8A proof challenge contract review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePassportChallengeContractReviewError {
    /// Challenge contract domain must match.
    ContractDomainMismatch,
    /// Challenge contract version must match.
    ContractVersionMismatch,
    /// Required public text field was empty.
    EmptyField(&'static str),
    /// Reserved wallet purpose remains disabled.
    ReservedWalletPurpose,
    /// Challenge purpose requires Passport binding.
    MissingPassportBinding,
    /// Challenge purpose requires Device binding.
    MissingDeviceBinding,
    /// Challenge purpose requires operation body hash binding.
    MissingOperationBodyHash,
    /// Requested scopes must not be empty.
    EmptyRequestedScopes,
    /// Duplicate requested scope rejected.
    DuplicateRequestedScope,
    /// Requested scope outside Phase 8A ceiling.
    ScopeOutsideReadOnlyCeiling,
    /// Challenge expiry must be after issued-at.
    InvalidExpiryWindow,
    /// Challenge TTL exceeded the Phase 8A ceiling.
    ChallengeTtlTooLong,
    /// Clock skew metadata exceeded the Phase 8A ceiling.
    ClockSkewTooLong,
    /// Transcript codec must be canonical and not JSON-byte signing.
    InvalidTranscriptCodec,
    /// Service signature algorithm must match expected label.
    InvalidServiceSignatureAlgorithm,
    /// Service signature placeholder must be present.
    MissingServiceSignaturePlaceholder,
    /// Challenge descriptors must remain contract-only.
    ChallengeContractNotContractOnly,
    /// DTO flags attempted to carry or exercise unsafe authority.
    UnsafeChallengeAuthorityFlag,
}

/// Phase 8A posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportChallengeContractPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds challenge contract DTOs.
    pub proof_challenge_contract_dtos_added: bool,
    /// Whether this phase signs service challenges.
    pub service_challenge_signing_added: bool,
    /// Whether this phase verifies service challenges.
    pub service_challenge_verification_added: bool,
    /// Whether this phase signs root proofs.
    pub root_proof_signing_added: bool,
    /// Whether this phase signs device proofs.
    pub device_proof_signing_added: bool,
    /// Whether this phase verifies proofs.
    pub proof_signature_verification_added: bool,
    /// Whether this phase signs or verifies request proofs.
    pub request_proof_runtime_added: bool,
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

/// Return Phase 8A challenge contract posture.
pub fn native_passport_challenge_contract_posture() -> NativePassportChallengeContractPosture {
    NativePassportChallengeContractPosture {
        phase_label: NATIVE_PASSPORT_PHASE8A_LABEL,
        proof_challenge_contract_dtos_added: true,
        service_challenge_signing_added: false,
        service_challenge_verification_added: false,
        root_proof_signing_added: false,
        device_proof_signing_added: false,
        proof_signature_verification_added: false,
        request_proof_runtime_added: false,
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
        forbidden_authority_flags: PHASE8A_FORBIDDEN_CHALLENGE_AUTHORITY_FLAGS,
    }
}

/// Validate a purpose-bound challenge descriptor without signing or verifying proofs.
pub fn validate_native_passport_challenge_contract_descriptor(
    descriptor: &NativePassportChallengeContractDescriptorV1,
) -> Result<(), NativePassportChallengeContractReviewError> {
    validate_contract_fields(
        descriptor.contract_domain,
        descriptor.contract_version,
        descriptor.network_id,
        descriptor.environment,
        descriptor.audience,
        descriptor.issuing_service_id,
        descriptor.service_key_id,
        descriptor.purpose,
        &descriptor.requested_scopes,
        descriptor.passport_id.as_ref(),
        descriptor.device_id.as_ref(),
        descriptor.operation_body_hash.as_ref(),
        descriptor.issued_at_ms,
        descriptor.expires_at_ms,
        descriptor.max_clock_skew_ms,
        descriptor.transcript_codec,
        descriptor.service_signature_algorithm,
        descriptor.service_signature_placeholder_present,
    )?;

    if !descriptor.contract_only {
        return Err(NativePassportChallengeContractReviewError::ChallengeContractNotContractOnly);
    }

    Ok(())
}

/// Review a purpose-bound challenge draft into a contract-only descriptor.
///
/// This is challenge DTO review only. It does not create or verify service
/// signatures, sign root/device/request proofs, verify proofs, consume
/// challenges, mutate replay stores, issue capabilities, execute device or
/// namespace state transitions, derive keys, validate PINs, unlock vaults,
/// store secrets, contact live RPC, add routes, or mutate wallet/ledger state.
pub fn review_native_passport_challenge_contract_draft(
    draft: NativePassportChallengeContractDraftV1,
) -> Result<NativePassportChallengeContractDescriptorV1, NativePassportChallengeContractReviewError>
{
    validate_contract_fields(
        draft.contract_domain,
        draft.contract_version,
        draft.network_id,
        draft.environment,
        draft.audience,
        draft.issuing_service_id,
        draft.service_key_id,
        draft.purpose,
        &draft.requested_scopes,
        draft.passport_id.as_ref(),
        draft.device_id.as_ref(),
        draft.operation_body_hash.as_ref(),
        draft.issued_at_ms,
        draft.expires_at_ms,
        draft.max_clock_skew_ms,
        draft.transcript_codec,
        draft.service_signature_algorithm,
        draft.service_signature_placeholder_present,
    )?;

    if draft.requests_service_challenge_signing
        || draft.requests_service_challenge_verification
        || draft.requests_root_proof_signing
        || draft.requests_device_proof_signing
        || draft.requests_proof_signature_verification
        || draft.requests_request_proof_signing
        || draft.requests_request_proof_verification
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
        return Err(NativePassportChallengeContractReviewError::UnsafeChallengeAuthorityFlag);
    }

    Ok(NativePassportChallengeContractDescriptorV1 {
        contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
        contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
        challenge_id: draft.challenge_id,
        network_id: draft.network_id,
        environment: draft.environment,
        audience: draft.audience,
        issuing_service_id: draft.issuing_service_id,
        service_key_id: draft.service_key_id,
        purpose: draft.purpose,
        requested_scopes: draft.requested_scopes,
        passport_id: draft.passport_id,
        device_id: draft.device_id,
        operation_body_hash: draft.operation_body_hash,
        nonce_hex: draft.nonce_hex,
        issued_at_ms: draft.issued_at_ms,
        expires_at_ms: draft.expires_at_ms,
        max_clock_skew_ms: draft.max_clock_skew_ms,
        transcript_codec: PHASE8A_REQUIRED_TRANSCRIPT_CODEC,
        service_signature_algorithm: PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM,
        service_signature_placeholder_present: true,
        contract_only: true,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_contract_fields(
    contract_domain: &'static str,
    contract_version: u16,
    network_id: &'static str,
    environment: &'static str,
    audience: &'static str,
    issuing_service_id: &'static str,
    service_key_id: &'static str,
    purpose: NativePassportChallengePurpose,
    requested_scopes: &[NativePassportScope],
    passport_id: Option<&PassportIdV1>,
    device_id: Option<&DeviceIdV1>,
    operation_body_hash: Option<&B3DigestHex>,
    issued_at_ms: u64,
    expires_at_ms: u64,
    max_clock_skew_ms: u64,
    transcript_codec: NativeChallengeTranscriptCodec,
    service_signature_algorithm: NativeChallengeSignatureAlgorithm,
    service_signature_placeholder_present: bool,
) -> Result<(), NativePassportChallengeContractReviewError> {
    if contract_domain != PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN {
        return Err(NativePassportChallengeContractReviewError::ContractDomainMismatch);
    }

    if contract_version != PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION {
        return Err(NativePassportChallengeContractReviewError::ContractVersionMismatch);
    }

    for (field, value) in [
        ("network_id", network_id),
        ("environment", environment),
        ("audience", audience),
        ("issuing_service_id", issuing_service_id),
        ("service_key_id", service_key_id),
    ] {
        if value.is_empty() {
            return Err(NativePassportChallengeContractReviewError::EmptyField(
                field,
            ));
        }
    }

    if purpose == NativePassportChallengePurpose::WalletAuthorizationRequestReserved {
        return Err(NativePassportChallengeContractReviewError::ReservedWalletPurpose);
    }

    if purpose_requires_passport_binding(purpose) && passport_id.is_none() {
        return Err(NativePassportChallengeContractReviewError::MissingPassportBinding);
    }

    if purpose_requires_device_binding(purpose) && device_id.is_none() {
        return Err(NativePassportChallengeContractReviewError::MissingDeviceBinding);
    }

    if purpose_requires_operation_body_hash(purpose) && operation_body_hash.is_none() {
        return Err(NativePassportChallengeContractReviewError::MissingOperationBodyHash);
    }

    validate_requested_scopes(requested_scopes)?;
    validate_time_window(issued_at_ms, expires_at_ms, max_clock_skew_ms)?;

    if transcript_codec != PHASE8A_REQUIRED_TRANSCRIPT_CODEC {
        return Err(NativePassportChallengeContractReviewError::InvalidTranscriptCodec);
    }

    if service_signature_algorithm != PHASE8A_REQUIRED_SERVICE_SIGNATURE_ALGORITHM {
        return Err(NativePassportChallengeContractReviewError::InvalidServiceSignatureAlgorithm);
    }

    if !service_signature_placeholder_present {
        return Err(NativePassportChallengeContractReviewError::MissingServiceSignaturePlaceholder);
    }

    Ok(())
}

fn purpose_requires_passport_binding(purpose: NativePassportChallengePurpose) -> bool {
    matches!(
        purpose,
        NativePassportChallengePurpose::RegisterRoot
            | NativePassportChallengePurpose::AuthorizeDevice
            | NativePassportChallengePurpose::RevokeDevice
            | NativePassportChallengePurpose::ProveSession
            | NativePassportChallengePurpose::IssueCapability
            | NativePassportChallengePurpose::RefreshCapability
            | NativePassportChallengePurpose::ClaimUsername
            | NativePassportChallengePurpose::TransferUsername
            | NativePassportChallengePurpose::ReleaseUsername
            | NativePassportChallengePurpose::PublishProfile
            | NativePassportChallengePurpose::RegisterSite
            | NativePassportChallengePurpose::UpdateSite
    )
}

fn purpose_requires_device_binding(purpose: NativePassportChallengePurpose) -> bool {
    matches!(
        purpose,
        NativePassportChallengePurpose::RevokeDevice
            | NativePassportChallengePurpose::ProveSession
            | NativePassportChallengePurpose::IssueCapability
            | NativePassportChallengePurpose::RefreshCapability
            | NativePassportChallengePurpose::PublishProfile
            | NativePassportChallengePurpose::RegisterSite
            | NativePassportChallengePurpose::UpdateSite
    )
}

fn purpose_requires_operation_body_hash(purpose: NativePassportChallengePurpose) -> bool {
    matches!(
        purpose,
        NativePassportChallengePurpose::RegisterRoot
            | NativePassportChallengePurpose::AuthorizeDevice
            | NativePassportChallengePurpose::RevokeDevice
            | NativePassportChallengePurpose::IssueCapability
            | NativePassportChallengePurpose::RefreshCapability
            | NativePassportChallengePurpose::ClaimUsername
            | NativePassportChallengePurpose::TransferUsername
            | NativePassportChallengePurpose::ReleaseUsername
            | NativePassportChallengePurpose::PublishProfile
            | NativePassportChallengePurpose::RegisterSite
            | NativePassportChallengePurpose::UpdateSite
    )
}

fn validate_requested_scopes(
    scopes: &[NativePassportScope],
) -> Result<(), NativePassportChallengeContractReviewError> {
    if scopes.is_empty() {
        return Err(NativePassportChallengeContractReviewError::EmptyRequestedScopes);
    }

    for (index, scope) in scopes.iter().enumerate() {
        if scopes[..index].contains(scope) {
            return Err(NativePassportChallengeContractReviewError::DuplicateRequestedScope);
        }

        if !PHASE8A_ALLOWED_CHALLENGE_SCOPES.contains(scope) {
            return Err(NativePassportChallengeContractReviewError::ScopeOutsideReadOnlyCeiling);
        }
    }

    Ok(())
}

fn validate_time_window(
    issued_at_ms: u64,
    expires_at_ms: u64,
    max_clock_skew_ms: u64,
) -> Result<(), NativePassportChallengeContractReviewError> {
    if expires_at_ms <= issued_at_ms {
        return Err(NativePassportChallengeContractReviewError::InvalidExpiryWindow);
    }

    if expires_at_ms - issued_at_ms > PHASE8A_MAX_CHALLENGE_TTL_MS {
        return Err(NativePassportChallengeContractReviewError::ChallengeTtlTooLong);
    }

    if max_clock_skew_ms > PHASE8A_MAX_CLOCK_SKEW_MS {
        return Err(NativePassportChallengeContractReviewError::ClockSkewTooLong);
    }

    Ok(())
}
