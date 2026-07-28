//! RO:WHAT — Native Passport delegated enrollment contract DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines delegated invitation/approval metadata after restore and new-device enrollment contracts, before any delegated enrollment runtime exists.
//! RO:INTERACTS — Phase 7A restore contracts, Phase 7B new-device enrollment contracts, future purpose-bound challenge/proof contracts.
//! RO:INVARIANTS — public invitation/source/authority/target/challenge/proof/policy labels only; no signing, verification, key generation, restore/enrollment execution, capability issuance, storage, routes, runtime I/O, wallet, ledger, or secret custody.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects domain/version drift, restore/enrollment linkage mismatch, platform mismatch, unsupported target class, empty/duplicate/missing delegated labels, non-contract descriptors, and flags implying runtime authority.
//! RO:TEST — tests/native_passport_phase7c_delegated_enrollment_contract_dto.rs.

use super::{
    validate_native_new_device_enrollment_contract_descriptor, DeviceClass,
    NativeNewDeviceEnrollmentContractDescriptorV1, NativePlatformFamily,
    NativeRestoreContractDescriptorV1, PHASE7A_RESTORE_CONTRACT_DOMAIN,
    PHASE7A_RESTORE_CONTRACT_VERSION, PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN,
    PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION,
};

/// Phase label for native delegated enrollment contract DTO foundations.
pub const NATIVE_PASSPORT_PHASE7C_LABEL: &str =
    "NATIVE_PASSPORT_PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DTO";

/// Canonical delegated enrollment contract DTO domain.
pub const PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DOMAIN: &str =
    "native-passport/delegated-enrollment-contract/v1";

/// Delegated enrollment contract version.
pub const PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_VERSION: u16 = 1;

/// Required delegated enrollment sources.
pub const PHASE7C_REQUIRED_DELEGATED_SOURCES: &[NativeDelegatedEnrollmentSource] = &[
    NativeDelegatedEnrollmentSource::RestoreContract,
    NativeDelegatedEnrollmentSource::NewDeviceEnrollmentContract,
    NativeDelegatedEnrollmentSource::DelegatedInvitationMetadata,
];

/// Required approving authority labels.
pub const PHASE7C_REQUIRED_APPROVING_AUTHORITIES: &[NativeDelegatedApprovingAuthority] = &[
    NativeDelegatedApprovingAuthority::RecoveryRootApproval,
    NativeDelegatedApprovingAuthority::ExistingAuthorizedDevice,
];

/// Required public challenge placeholders.
pub const PHASE7C_REQUIRED_CHALLENGE_PLACEHOLDERS: &[NativeDelegatedChallengePlaceholder] = &[
    NativeDelegatedChallengePlaceholder::InvitationChallenge,
    NativeDelegatedChallengePlaceholder::EnrollmentChallenge,
];

/// Required public proof placeholders.
pub const PHASE7C_REQUIRED_PROOF_PLACEHOLDERS: &[NativeDelegatedProofPlaceholder] = &[
    NativeDelegatedProofPlaceholder::InvitationProof,
    NativeDelegatedProofPlaceholder::DevicePossessionProof,
];

/// Required public policy metadata labels.
pub const PHASE7C_REQUIRED_POLICY_METADATA: &[NativeDelegatedPolicyMetadata] = &[
    NativeDelegatedPolicyMetadata::SingleUseInvitation,
    NativeDelegatedPolicyMetadata::ExpiresBeforeCapabilityIssuance,
    NativeDelegatedPolicyMetadata::ReadOnlyCapabilityCeiling,
];

/// Read-only device classes allowed by the Phase 7C delegated enrollment contract.
pub const PHASE7C_ALLOWED_TARGET_DEVICE_CLASSES: &[DeviceClass] = &[
    DeviceClass::TvReadOnly,
    DeviceClass::DesktopReadOnly,
    DeviceClass::MobileReadOnly,
];

/// Authority meanings that Phase 7C DTOs must not grant.
pub const PHASE7C_FORBIDDEN_DELEGATED_AUTHORITY_FLAGS: &[&str] = &[
    "delegated_enrollment_execution",
    "invitation_signing",
    "invitation_verification",
    "challenge_runtime",
    "proof_signing",
    "proof_verification",
    "capability_issuance",
    "device_key_generation",
    "restore_execution",
    "root_key_derivation",
    "device_key_derivation",
    "pin_validation_runtime",
    "pin_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_implementation",
    "secret_storage",
    "secret_export",
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

/// Public delegated enrollment source labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeDelegatedEnrollmentSource {
    /// Delegated enrollment is anchored to a reviewed restore contract.
    RestoreContract,
    /// Delegated enrollment is anchored to a reviewed new-device enrollment contract.
    NewDeviceEnrollmentContract,
    /// Delegated invitation metadata is public contract metadata only.
    DelegatedInvitationMetadata,
}

/// Public approving authority labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeDelegatedApprovingAuthority {
    /// Later approval may come from the recovery root.
    RecoveryRootApproval,
    /// Later approval may come from an existing authorized device.
    ExistingAuthorizedDevice,
    /// Later approval may be refused by policy before runtime exists.
    PolicyRefusal,
}

/// Public challenge placeholder labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeDelegatedChallengePlaceholder {
    /// Public placeholder for a future invitation challenge.
    InvitationChallenge,
    /// Public placeholder for a future enrollment challenge.
    EnrollmentChallenge,
}

/// Public proof placeholder labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeDelegatedProofPlaceholder {
    /// Public placeholder for a future invitation proof.
    InvitationProof,
    /// Public placeholder for a future device possession proof.
    DevicePossessionProof,
}

/// Public delegated enrollment policy metadata labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeDelegatedPolicyMetadata {
    /// Invitation should later be single-use.
    SingleUseInvitation,
    /// Invitation must expire before any later capability issuance.
    ExpiresBeforeCapabilityIssuance,
    /// Delegated device must stay inside a read-only capability ceiling.
    ReadOnlyCapabilityCeiling,
}

/// Public delegated enrollment contract descriptor without delegated enrollment runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDelegatedEnrollmentContractDescriptorV1 {
    /// Delegated enrollment contract domain.
    pub contract_domain: &'static str,
    /// Delegated enrollment contract version.
    pub contract_version: u16,
    /// Platform family inherited from the restore/new-device contracts.
    pub platform_family: NativePlatformFamily,
    /// Restore contract domain this delegated contract binds to.
    pub restore_contract_domain: &'static str,
    /// Restore contract version this delegated contract binds to.
    pub restore_contract_version: u16,
    /// New-device enrollment contract domain this delegated contract binds to.
    pub enrollment_contract_domain: &'static str,
    /// New-device enrollment contract version this delegated contract binds to.
    pub enrollment_contract_version: u16,
    /// Read-only target class for the delegated device.
    pub target_device_class: DeviceClass,
    /// Public delegated enrollment source labels.
    pub delegated_sources: Vec<NativeDelegatedEnrollmentSource>,
    /// Public approving authority labels.
    pub approving_authorities: Vec<NativeDelegatedApprovingAuthority>,
    /// Public challenge placeholders only.
    pub challenge_placeholders: Vec<NativeDelegatedChallengePlaceholder>,
    /// Public proof placeholders only.
    pub proof_placeholders: Vec<NativeDelegatedProofPlaceholder>,
    /// Public expiry/capability policy metadata only.
    pub policy_metadata: Vec<NativeDelegatedPolicyMetadata>,
    /// Whether this descriptor is contract-only.
    pub contract_only: bool,
}

/// Public delegated enrollment contract draft before delegated enrollment runtime exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDelegatedEnrollmentContractDraftV1 {
    /// Delegated enrollment contract domain.
    pub contract_domain: &'static str,
    /// Delegated enrollment contract version.
    pub contract_version: u16,
    /// Platform family expected from the restore/new-device contracts.
    pub platform_family: NativePlatformFamily,
    /// Restore contract domain this delegated contract binds to.
    pub restore_contract_domain: &'static str,
    /// Restore contract version this delegated contract binds to.
    pub restore_contract_version: u16,
    /// New-device enrollment contract domain this delegated contract binds to.
    pub enrollment_contract_domain: &'static str,
    /// New-device enrollment contract version this delegated contract binds to.
    pub enrollment_contract_version: u16,
    /// Read-only target class for the delegated device.
    pub target_device_class: DeviceClass,
    /// Requested public delegated source labels.
    pub requested_delegated_sources: Vec<NativeDelegatedEnrollmentSource>,
    /// Requested public approving authority labels.
    pub requested_approving_authorities: Vec<NativeDelegatedApprovingAuthority>,
    /// Requested public challenge placeholders.
    pub requested_challenge_placeholders: Vec<NativeDelegatedChallengePlaceholder>,
    /// Requested public proof placeholders.
    pub requested_proof_placeholders: Vec<NativeDelegatedProofPlaceholder>,
    /// Requested public policy metadata labels.
    pub requested_policy_metadata: Vec<NativeDelegatedPolicyMetadata>,
    /// Boundary flag: this DTO must not execute delegated enrollment.
    pub requests_delegated_enrollment_execution: bool,
    /// Boundary flag: this DTO must not sign invitations.
    pub requests_invitation_signing: bool,
    /// Boundary flag: this DTO must not verify invitations.
    pub requests_invitation_verification: bool,
    /// Boundary flag: this DTO must not issue live challenges.
    pub requests_challenge_runtime: bool,
    /// Boundary flag: this DTO must not sign or verify proofs.
    pub requests_proof_signing_or_verification: bool,
    /// Boundary flag: this DTO must not issue capabilities.
    pub requests_capability_issuance: bool,
    /// Boundary flag: this DTO must not generate device keys.
    pub requests_device_key_generation: bool,
    /// Boundary flag: this DTO must not derive keys.
    pub requests_key_derivation_runtime: bool,
    /// Boundary flag: this DTO must not execute restore.
    pub requests_restore_execution: bool,
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
    /// Boundary flag: this DTO must not store invitation or secret material.
    pub stores_invitation_or_secret_material: bool,
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

/// Phase 7C delegated enrollment contract review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDelegatedEnrollmentContractReviewError {
    /// Delegated contract domain must match.
    ContractDomainMismatch,
    /// Delegated contract version must match.
    ContractVersionMismatch,
    /// Platform family must match the restore/new-device contracts.
    PlatformFamilyMismatch,
    /// Restore contract domain must match.
    RestoreContractDomainMismatch,
    /// Restore contract version must match.
    RestoreContractVersionMismatch,
    /// New-device enrollment contract domain must match.
    EnrollmentContractDomainMismatch,
    /// New-device enrollment contract version must match.
    EnrollmentContractVersionMismatch,
    /// Restore and enrollment contracts must already validate together.
    EnrollmentContractReferenceInvalid,
    /// Target device class must remain read-only.
    UnsupportedTargetDeviceClass,
    /// At least one delegated source is required.
    MissingDelegatedSources,
    /// Duplicate delegated sources are rejected.
    DuplicateDelegatedSource,
    /// Required delegated source is missing.
    MissingRequiredDelegatedSource,
    /// At least one approving authority is required.
    MissingApprovingAuthorities,
    /// Duplicate approving authorities are rejected.
    DuplicateApprovingAuthority,
    /// Required approving authority is missing.
    MissingRequiredApprovingAuthority,
    /// At least one challenge placeholder is required.
    MissingChallengePlaceholders,
    /// Duplicate challenge placeholders are rejected.
    DuplicateChallengePlaceholder,
    /// Required challenge placeholder is missing.
    MissingRequiredChallengePlaceholder,
    /// At least one proof placeholder is required.
    MissingProofPlaceholders,
    /// Duplicate proof placeholders are rejected.
    DuplicateProofPlaceholder,
    /// Required proof placeholder is missing.
    MissingRequiredProofPlaceholder,
    /// At least one policy metadata label is required.
    MissingPolicyMetadata,
    /// Duplicate policy metadata labels are rejected.
    DuplicatePolicyMetadata,
    /// Required policy metadata label is missing.
    MissingRequiredPolicyMetadata,
    /// Delegated enrollment descriptors must remain contract-only.
    DelegatedEnrollmentContractNotContractOnly,
    /// DTO flags attempted to carry or exercise unsafe delegated authority.
    UnsafeDelegatedAuthorityFlag,
}

/// Phase 7C posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportDelegatedEnrollmentPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds delegated enrollment contract DTOs.
    pub delegated_enrollment_contract_dtos_added: bool,
    /// Whether this phase executes delegated enrollment.
    pub delegated_enrollment_execution_added: bool,
    /// Whether this phase signs invitations.
    pub invitation_signing_added: bool,
    /// Whether this phase verifies invitations.
    pub invitation_verification_added: bool,
    /// Whether this phase adds challenge runtime.
    pub challenge_runtime_added: bool,
    /// Whether this phase signs or verifies proofs.
    pub proof_signing_or_verification_runtime_added: bool,
    /// Whether this phase issues capabilities.
    pub capability_issuance_added: bool,
    /// Whether this phase generates device keys.
    pub device_key_generation_added: bool,
    /// Whether this phase derives root/device keys.
    pub key_derivation_runtime_added: bool,
    /// Whether this phase validates PIN values.
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
    /// Whether this phase exports secrets/material.
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

/// Return Phase 7C delegated enrollment contract posture.
pub fn native_passport_delegated_enrollment_posture() -> NativePassportDelegatedEnrollmentPosture {
    NativePassportDelegatedEnrollmentPosture {
        phase_label: NATIVE_PASSPORT_PHASE7C_LABEL,
        delegated_enrollment_contract_dtos_added: true,
        delegated_enrollment_execution_added: false,
        invitation_signing_added: false,
        invitation_verification_added: false,
        challenge_runtime_added: false,
        proof_signing_or_verification_runtime_added: false,
        capability_issuance_added: false,
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
        forbidden_authority_flags: PHASE7C_FORBIDDEN_DELEGATED_AUTHORITY_FLAGS,
    }
}

/// Validate a delegated enrollment descriptor without executing delegated enrollment.
pub fn validate_native_delegated_enrollment_contract_descriptor(
    restore_contract: &NativeRestoreContractDescriptorV1,
    enrollment_contract: &NativeNewDeviceEnrollmentContractDescriptorV1,
    descriptor: &NativeDelegatedEnrollmentContractDescriptorV1,
) -> Result<(), NativeDelegatedEnrollmentContractReviewError> {
    validate_contract_references(restore_contract, enrollment_contract)?;

    if descriptor.contract_domain != PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DOMAIN {
        return Err(NativeDelegatedEnrollmentContractReviewError::ContractDomainMismatch);
    }

    if descriptor.contract_version != PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_VERSION {
        return Err(NativeDelegatedEnrollmentContractReviewError::ContractVersionMismatch);
    }

    if descriptor.platform_family != restore_contract.platform_family
        || descriptor.platform_family != enrollment_contract.platform_family
    {
        return Err(NativeDelegatedEnrollmentContractReviewError::PlatformFamilyMismatch);
    }

    if descriptor.restore_contract_domain != PHASE7A_RESTORE_CONTRACT_DOMAIN
        || descriptor.restore_contract_domain != restore_contract.contract_domain
    {
        return Err(NativeDelegatedEnrollmentContractReviewError::RestoreContractDomainMismatch);
    }

    if descriptor.restore_contract_version != PHASE7A_RESTORE_CONTRACT_VERSION
        || descriptor.restore_contract_version != restore_contract.contract_version
    {
        return Err(NativeDelegatedEnrollmentContractReviewError::RestoreContractVersionMismatch);
    }

    if descriptor.enrollment_contract_domain != PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN
        || descriptor.enrollment_contract_domain != enrollment_contract.contract_domain
    {
        return Err(NativeDelegatedEnrollmentContractReviewError::EnrollmentContractDomainMismatch);
    }

    if descriptor.enrollment_contract_version != PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION
        || descriptor.enrollment_contract_version != enrollment_contract.contract_version
    {
        return Err(
            NativeDelegatedEnrollmentContractReviewError::EnrollmentContractVersionMismatch,
        );
    }

    validate_target_device_class(descriptor.target_device_class)?;
    validate_delegated_sources(&descriptor.delegated_sources)?;
    validate_approving_authorities(&descriptor.approving_authorities)?;
    validate_challenge_placeholders(&descriptor.challenge_placeholders)?;
    validate_proof_placeholders(&descriptor.proof_placeholders)?;
    validate_policy_metadata(&descriptor.policy_metadata)?;

    if !descriptor.contract_only {
        return Err(
            NativeDelegatedEnrollmentContractReviewError::DelegatedEnrollmentContractNotContractOnly,
        );
    }

    Ok(())
}

/// Review a delegated enrollment draft into a contract-only descriptor.
///
/// This is contract DTO review only. It does not execute delegated enrollment,
/// sign or verify invitations, issue challenges, sign or verify proofs, issue
/// capabilities, generate device keys, derive keys, validate PINs, unlock
/// vaults, store or export secrets, encrypt, decrypt, contact live RPC, mutate
/// storage, add routes, perform runtime I/O, mutate wallets, or mutate ledgers.
pub fn review_native_delegated_enrollment_contract_draft(
    restore_contract: &NativeRestoreContractDescriptorV1,
    enrollment_contract: &NativeNewDeviceEnrollmentContractDescriptorV1,
    draft: NativeDelegatedEnrollmentContractDraftV1,
) -> Result<
    NativeDelegatedEnrollmentContractDescriptorV1,
    NativeDelegatedEnrollmentContractReviewError,
> {
    validate_contract_references(restore_contract, enrollment_contract)?;

    if draft.contract_domain != PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DOMAIN {
        return Err(NativeDelegatedEnrollmentContractReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_VERSION {
        return Err(NativeDelegatedEnrollmentContractReviewError::ContractVersionMismatch);
    }

    if draft.platform_family != restore_contract.platform_family
        || draft.platform_family != enrollment_contract.platform_family
    {
        return Err(NativeDelegatedEnrollmentContractReviewError::PlatformFamilyMismatch);
    }

    if draft.restore_contract_domain != PHASE7A_RESTORE_CONTRACT_DOMAIN
        || draft.restore_contract_domain != restore_contract.contract_domain
    {
        return Err(NativeDelegatedEnrollmentContractReviewError::RestoreContractDomainMismatch);
    }

    if draft.restore_contract_version != PHASE7A_RESTORE_CONTRACT_VERSION
        || draft.restore_contract_version != restore_contract.contract_version
    {
        return Err(NativeDelegatedEnrollmentContractReviewError::RestoreContractVersionMismatch);
    }

    if draft.enrollment_contract_domain != PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN
        || draft.enrollment_contract_domain != enrollment_contract.contract_domain
    {
        return Err(NativeDelegatedEnrollmentContractReviewError::EnrollmentContractDomainMismatch);
    }

    if draft.enrollment_contract_version != PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION
        || draft.enrollment_contract_version != enrollment_contract.contract_version
    {
        return Err(
            NativeDelegatedEnrollmentContractReviewError::EnrollmentContractVersionMismatch,
        );
    }

    validate_target_device_class(draft.target_device_class)?;
    validate_delegated_sources(&draft.requested_delegated_sources)?;
    validate_approving_authorities(&draft.requested_approving_authorities)?;
    validate_challenge_placeholders(&draft.requested_challenge_placeholders)?;
    validate_proof_placeholders(&draft.requested_proof_placeholders)?;
    validate_policy_metadata(&draft.requested_policy_metadata)?;

    if draft.requests_delegated_enrollment_execution
        || draft.requests_invitation_signing
        || draft.requests_invitation_verification
        || draft.requests_challenge_runtime
        || draft.requests_proof_signing_or_verification
        || draft.requests_capability_issuance
        || draft.requests_device_key_generation
        || draft.requests_key_derivation_runtime
        || draft.requests_restore_execution
        || draft.requests_pin_validation_runtime
        || draft.requests_pin_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.includes_platform_sealer_implementation
        || draft.stores_invitation_or_secret_material
        || draft.exports_secret_or_material
        || draft.requests_encryption_or_decryption
        || draft.requests_live_rpc
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeDelegatedEnrollmentContractReviewError::UnsafeDelegatedAuthorityFlag);
    }

    Ok(NativeDelegatedEnrollmentContractDescriptorV1 {
        contract_domain: PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_DOMAIN,
        contract_version: PHASE7C_DELEGATED_ENROLLMENT_CONTRACT_VERSION,
        platform_family: draft.platform_family,
        restore_contract_domain: PHASE7A_RESTORE_CONTRACT_DOMAIN,
        restore_contract_version: PHASE7A_RESTORE_CONTRACT_VERSION,
        enrollment_contract_domain: PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN,
        enrollment_contract_version: PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION,
        target_device_class: draft.target_device_class,
        delegated_sources: draft.requested_delegated_sources,
        approving_authorities: draft.requested_approving_authorities,
        challenge_placeholders: draft.requested_challenge_placeholders,
        proof_placeholders: draft.requested_proof_placeholders,
        policy_metadata: draft.requested_policy_metadata,
        contract_only: true,
    })
}

fn validate_contract_references(
    restore_contract: &NativeRestoreContractDescriptorV1,
    enrollment_contract: &NativeNewDeviceEnrollmentContractDescriptorV1,
) -> Result<(), NativeDelegatedEnrollmentContractReviewError> {
    validate_native_new_device_enrollment_contract_descriptor(restore_contract, enrollment_contract)
        .map_err(|_| {
            NativeDelegatedEnrollmentContractReviewError::EnrollmentContractReferenceInvalid
        })
}

fn validate_target_device_class(
    target_device_class: DeviceClass,
) -> Result<(), NativeDelegatedEnrollmentContractReviewError> {
    if !PHASE7C_ALLOWED_TARGET_DEVICE_CLASSES.contains(&target_device_class) {
        return Err(NativeDelegatedEnrollmentContractReviewError::UnsupportedTargetDeviceClass);
    }

    Ok(())
}

fn validate_delegated_sources(
    values: &[NativeDelegatedEnrollmentSource],
) -> Result<(), NativeDelegatedEnrollmentContractReviewError> {
    if values.is_empty() {
        return Err(NativeDelegatedEnrollmentContractReviewError::MissingDelegatedSources);
    }

    for (index, value) in values.iter().enumerate() {
        if values[..index].contains(value) {
            return Err(NativeDelegatedEnrollmentContractReviewError::DuplicateDelegatedSource);
        }
    }

    for required in PHASE7C_REQUIRED_DELEGATED_SOURCES {
        if !values.contains(required) {
            return Err(
                NativeDelegatedEnrollmentContractReviewError::MissingRequiredDelegatedSource,
            );
        }
    }

    Ok(())
}

fn validate_approving_authorities(
    values: &[NativeDelegatedApprovingAuthority],
) -> Result<(), NativeDelegatedEnrollmentContractReviewError> {
    if values.is_empty() {
        return Err(NativeDelegatedEnrollmentContractReviewError::MissingApprovingAuthorities);
    }

    for (index, value) in values.iter().enumerate() {
        if values[..index].contains(value) {
            return Err(NativeDelegatedEnrollmentContractReviewError::DuplicateApprovingAuthority);
        }
    }

    for required in PHASE7C_REQUIRED_APPROVING_AUTHORITIES {
        if !values.contains(required) {
            return Err(
                NativeDelegatedEnrollmentContractReviewError::MissingRequiredApprovingAuthority,
            );
        }
    }

    Ok(())
}

fn validate_challenge_placeholders(
    values: &[NativeDelegatedChallengePlaceholder],
) -> Result<(), NativeDelegatedEnrollmentContractReviewError> {
    if values.is_empty() {
        return Err(NativeDelegatedEnrollmentContractReviewError::MissingChallengePlaceholders);
    }

    for (index, value) in values.iter().enumerate() {
        if values[..index].contains(value) {
            return Err(
                NativeDelegatedEnrollmentContractReviewError::DuplicateChallengePlaceholder,
            );
        }
    }

    for required in PHASE7C_REQUIRED_CHALLENGE_PLACEHOLDERS {
        if !values.contains(required) {
            return Err(
                NativeDelegatedEnrollmentContractReviewError::MissingRequiredChallengePlaceholder,
            );
        }
    }

    Ok(())
}

fn validate_proof_placeholders(
    values: &[NativeDelegatedProofPlaceholder],
) -> Result<(), NativeDelegatedEnrollmentContractReviewError> {
    if values.is_empty() {
        return Err(NativeDelegatedEnrollmentContractReviewError::MissingProofPlaceholders);
    }

    for (index, value) in values.iter().enumerate() {
        if values[..index].contains(value) {
            return Err(NativeDelegatedEnrollmentContractReviewError::DuplicateProofPlaceholder);
        }
    }

    for required in PHASE7C_REQUIRED_PROOF_PLACEHOLDERS {
        if !values.contains(required) {
            return Err(
                NativeDelegatedEnrollmentContractReviewError::MissingRequiredProofPlaceholder,
            );
        }
    }

    Ok(())
}

fn validate_policy_metadata(
    values: &[NativeDelegatedPolicyMetadata],
) -> Result<(), NativeDelegatedEnrollmentContractReviewError> {
    if values.is_empty() {
        return Err(NativeDelegatedEnrollmentContractReviewError::MissingPolicyMetadata);
    }

    for (index, value) in values.iter().enumerate() {
        if values[..index].contains(value) {
            return Err(NativeDelegatedEnrollmentContractReviewError::DuplicatePolicyMetadata);
        }
    }

    for required in PHASE7C_REQUIRED_POLICY_METADATA {
        if !values.contains(required) {
            return Err(
                NativeDelegatedEnrollmentContractReviewError::MissingRequiredPolicyMetadata,
            );
        }
    }

    Ok(())
}
