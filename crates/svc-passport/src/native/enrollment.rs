//! RO:WHAT — Native Passport new-device enrollment contract DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines restore-bound device enrollment metadata before enrollment execution, device-key generation, authorization signing, capability issuance, runtime I/O, or routes exist.
//! RO:INTERACTS — Phase 7A restore contracts, shared read-only DeviceClass, future delegated enrollment and purpose-bound challenge phases.
//! RO:INVARIANTS — enrollment DTOs carry public domains, versions, device classes, intent/source labels, and boundary flags only; no secret material, key generation, signing, verification, capability issuance, restore execution, vault runtime, live RPC, routes, wallet, or ledger authority are added.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects domain/version drift, restore linkage mismatch, platform mismatch, unsupported target classes, empty/duplicate/missing enrollment intents or sources, non-contract descriptors, and flags implying runtime authority.
//! RO:TEST — tests/native_passport_phase7b_new_device_enrollment_contract_dto.rs.

use super::{
    DeviceClass, NativePlatformFamily, NativeRestoreContractDescriptorV1,
    PHASE7A_RESTORE_CONTRACT_DOMAIN, PHASE7A_RESTORE_CONTRACT_VERSION,
};

/// Phase label for native new-device enrollment contract DTO foundations.
pub const NATIVE_PASSPORT_PHASE7B_LABEL: &str =
    "NATIVE_PASSPORT_PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DTO";

/// Canonical new-device enrollment contract DTO domain.
pub const PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN: &str =
    "native-passport/new-device-enrollment-contract/v1";

/// New-device enrollment contract version.
pub const PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION: u16 = 1;

/// Required enrollment intents.
pub const PHASE7B_REQUIRED_ENROLLMENT_INTENTS: &[NativeEnrollmentIntent] = &[
    NativeEnrollmentIntent::RequestDeviceEnrollment,
    NativeEnrollmentIntent::BindDeviceAuthorization,
    NativeEnrollmentIntent::PrepareDeviceCapability,
];

/// Required enrollment source anchors.
pub const PHASE7B_REQUIRED_ENROLLMENT_SOURCES: &[NativeEnrollmentSource] =
    &[NativeEnrollmentSource::RestoreContract];

/// Allowed enrollment source labels.
pub const PHASE7B_ALLOWED_ENROLLMENT_SOURCES: &[NativeEnrollmentSource] = &[
    NativeEnrollmentSource::RestoreContract,
    NativeEnrollmentSource::LocalRootApproval,
    NativeEnrollmentSource::DelegatedDeviceInvitation,
];

/// Read-only device classes allowed by the Phase 7B contract.
pub const PHASE7B_ALLOWED_TARGET_DEVICE_CLASSES: &[DeviceClass] = &[
    DeviceClass::TvReadOnly,
    DeviceClass::DesktopReadOnly,
    DeviceClass::MobileReadOnly,
];

/// Authority meanings that Phase 7B DTOs must not grant.
pub const PHASE7B_FORBIDDEN_ENROLLMENT_AUTHORITY_FLAGS: &[&str] = &[
    "enrollment_execution",
    "device_key_generation",
    "device_authorization_signature",
    "signing_runtime",
    "signature_verification_runtime",
    "capability_issuance",
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
    "wallet_spend",
    "ledger_mutation",
];

/// Public new-device enrollment intent labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeEnrollmentIntent {
    /// Request later enrollment of a new local device.
    RequestDeviceEnrollment,
    /// Bind a later device authorization contract.
    BindDeviceAuthorization,
    /// Prepare later issuance of a narrow device-bound capability.
    PrepareDeviceCapability,
}

/// Public new-device enrollment source labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeEnrollmentSource {
    /// Enrollment is anchored to a reviewed restore contract.
    RestoreContract,
    /// Enrollment may later require local recovery-root approval.
    LocalRootApproval,
    /// Enrollment may later consume a delegated device invitation.
    DelegatedDeviceInvitation,
}

/// Public new-device enrollment contract descriptor without enrollment runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNewDeviceEnrollmentContractDescriptorV1 {
    /// Enrollment contract domain.
    pub contract_domain: &'static str,
    /// Enrollment contract version.
    pub contract_version: u16,
    /// Platform family inherited from the restore contract.
    pub platform_family: NativePlatformFamily,
    /// Restore contract domain this enrollment contract binds to.
    pub restore_contract_domain: &'static str,
    /// Restore contract version this enrollment contract binds to.
    pub restore_contract_version: u16,
    /// Read-only class requested for the future device.
    pub target_device_class: DeviceClass,
    /// Public enrollment intent labels.
    pub enrollment_intents: Vec<NativeEnrollmentIntent>,
    /// Public enrollment source labels.
    pub enrollment_sources: Vec<NativeEnrollmentSource>,
    /// Whether this descriptor is contract-only.
    pub contract_only: bool,
}

/// Public new-device enrollment contract draft before enrollment runtime exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNewDeviceEnrollmentContractDraftV1 {
    /// Enrollment contract domain.
    pub contract_domain: &'static str,
    /// Enrollment contract version.
    pub contract_version: u16,
    /// Platform family expected from the restore contract.
    pub platform_family: NativePlatformFamily,
    /// Restore contract domain this enrollment contract binds to.
    pub restore_contract_domain: &'static str,
    /// Restore contract version this enrollment contract binds to.
    pub restore_contract_version: u16,
    /// Read-only class requested for the future device.
    pub target_device_class: DeviceClass,
    /// Requested public enrollment intents.
    pub requested_enrollment_intents: Vec<NativeEnrollmentIntent>,
    /// Requested public enrollment sources.
    pub requested_enrollment_sources: Vec<NativeEnrollmentSource>,
    /// Boundary flag: this DTO must not execute enrollment.
    pub requests_enrollment_execution: bool,
    /// Boundary flag: this DTO must not generate device keys.
    pub requests_device_key_generation: bool,
    /// Boundary flag: this DTO must not produce a device authorization signature.
    pub requests_device_authorization_signature: bool,
    /// Boundary flag: this DTO must not add signing or verification runtime.
    pub requests_signing_or_verification_runtime: bool,
    /// Boundary flag: this DTO must not issue capabilities.
    pub requests_capability_issuance: bool,
    /// Boundary flag: this DTO must not execute restore.
    pub requests_restore_execution: bool,
    /// Boundary flag: this DTO must not derive root or device keys.
    pub requests_key_derivation_runtime: bool,
    /// Boundary flag: this DTO must not validate PIN values.
    pub requests_pin_validation_runtime: bool,
    /// Boundary flag: this DTO must not derive PIN keys.
    pub requests_pin_derivation_runtime: bool,
    /// Boundary flag: this DTO must not unlock a vault.
    pub requests_vault_unlock: bool,
    /// Boundary flag: this DTO must not include vault runtime.
    pub includes_vault_runtime: bool,
    /// Boundary flag: this DTO must not include platform sealer implementation.
    pub includes_platform_sealer_implementation: bool,
    /// Boundary flag: this DTO must not store secret material.
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
    /// Boundary flag: this DTO must not mutate wallet or ledger state.
    pub requests_wallet_or_ledger_mutation: bool,
}

/// Phase 7B new-device enrollment contract review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeNewDeviceEnrollmentContractReviewError {
    /// Enrollment contract domain must match.
    ContractDomainMismatch,
    /// Enrollment contract version must match.
    ContractVersionMismatch,
    /// Platform family must match the restore contract.
    PlatformFamilyMismatch,
    /// Restore contract domain must match.
    RestoreContractDomainMismatch,
    /// Restore contract version must match.
    RestoreContractVersionMismatch,
    /// Restore contract must remain contract-only.
    RestoreContractNotContractOnly,
    /// Target device class must remain within the read-only class ceiling.
    UnsupportedTargetDeviceClass,
    /// At least one enrollment intent is required.
    MissingEnrollmentIntents,
    /// Duplicate enrollment intents are rejected for deterministic review.
    DuplicateEnrollmentIntent,
    /// Required enrollment intents must be present.
    MissingRequiredEnrollmentIntent,
    /// At least one enrollment source is required.
    MissingEnrollmentSources,
    /// Duplicate enrollment sources are rejected for deterministic review.
    DuplicateEnrollmentSource,
    /// Required enrollment source anchors must be present.
    MissingRequiredEnrollmentSource,
    /// Enrollment source labels must be known.
    UnsupportedEnrollmentSource,
    /// Enrollment descriptors must remain contract-only.
    EnrollmentContractNotContractOnly,
    /// DTO flags attempted to carry or exercise unsafe enrollment authority.
    UnsafeEnrollmentAuthorityFlag,
}

/// Phase 7B posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportNewDeviceEnrollmentPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds new-device enrollment contract DTOs.
    pub new_device_enrollment_contract_dtos_added: bool,
    /// Whether this phase executes enrollment.
    pub enrollment_execution_added: bool,
    /// Whether this phase generates device keys.
    pub device_key_generation_added: bool,
    /// Whether this phase produces device authorization signatures.
    pub device_authorization_signature_added: bool,
    /// Whether this phase adds signing or verification runtime.
    pub signing_or_verification_runtime_added: bool,
    /// Whether this phase issues capabilities.
    pub capability_issuance_added: bool,
    /// Whether this phase executes restore.
    pub restore_execution_added: bool,
    /// Whether this phase derives root/device keys.
    pub key_derivation_runtime_added: bool,
    /// Whether this phase adds PIN validation runtime.
    pub pin_validation_runtime_added: bool,
    /// Whether this phase adds PIN derivation runtime.
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
    /// Whether this phase adds wallet or ledger mutation.
    pub wallet_or_ledger_mutation_added: bool,
    /// Whether this phase changes runtime authority.
    pub runtime_authority_changed: bool,
    /// Whether this phase adds native secret custody.
    pub native_secret_implementation_added: bool,
    /// Forbidden authority meanings.
    pub forbidden_authority_flags: &'static [&'static str],
}

/// Return Phase 7B new-device enrollment contract posture.
pub fn native_passport_new_device_enrollment_posture() -> NativePassportNewDeviceEnrollmentPosture {
    NativePassportNewDeviceEnrollmentPosture {
        phase_label: NATIVE_PASSPORT_PHASE7B_LABEL,
        new_device_enrollment_contract_dtos_added: true,
        enrollment_execution_added: false,
        device_key_generation_added: false,
        device_authorization_signature_added: false,
        signing_or_verification_runtime_added: false,
        capability_issuance_added: false,
        restore_execution_added: false,
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
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE7B_FORBIDDEN_ENROLLMENT_AUTHORITY_FLAGS,
    }
}

/// Validate a new-device enrollment descriptor without executing enrollment.
pub fn validate_native_new_device_enrollment_contract_descriptor(
    restore_contract: &NativeRestoreContractDescriptorV1,
    descriptor: &NativeNewDeviceEnrollmentContractDescriptorV1,
) -> Result<(), NativeNewDeviceEnrollmentContractReviewError> {
    validate_restore_contract_reference(restore_contract)?;

    if descriptor.contract_domain != PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN {
        return Err(NativeNewDeviceEnrollmentContractReviewError::ContractDomainMismatch);
    }

    if descriptor.contract_version != PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION {
        return Err(NativeNewDeviceEnrollmentContractReviewError::ContractVersionMismatch);
    }

    if descriptor.platform_family != restore_contract.platform_family {
        return Err(NativeNewDeviceEnrollmentContractReviewError::PlatformFamilyMismatch);
    }

    if descriptor.restore_contract_domain != restore_contract.contract_domain
        || descriptor.restore_contract_domain != PHASE7A_RESTORE_CONTRACT_DOMAIN
    {
        return Err(NativeNewDeviceEnrollmentContractReviewError::RestoreContractDomainMismatch);
    }

    if descriptor.restore_contract_version != restore_contract.contract_version
        || descriptor.restore_contract_version != PHASE7A_RESTORE_CONTRACT_VERSION
    {
        return Err(NativeNewDeviceEnrollmentContractReviewError::RestoreContractVersionMismatch);
    }

    validate_target_device_class(descriptor.target_device_class)?;
    validate_enrollment_intents(&descriptor.enrollment_intents)?;
    validate_enrollment_sources(&descriptor.enrollment_sources)?;

    if !descriptor.contract_only {
        return Err(
            NativeNewDeviceEnrollmentContractReviewError::EnrollmentContractNotContractOnly,
        );
    }

    Ok(())
}

/// Review a new-device enrollment draft into a contract-only descriptor.
///
/// This is contract DTO review only. It does not enroll devices, generate keys,
/// sign or verify authorization material, issue capabilities, execute restore,
/// derive keys, validate PINs, unlock vaults, call platform sealers, store or
/// export secrets, encrypt, decrypt, contact live RPC, add routes, perform
/// runtime I/O, mutate wallets, or mutate ledgers.
pub fn review_native_new_device_enrollment_contract_draft(
    restore_contract: &NativeRestoreContractDescriptorV1,
    draft: NativeNewDeviceEnrollmentContractDraftV1,
) -> Result<
    NativeNewDeviceEnrollmentContractDescriptorV1,
    NativeNewDeviceEnrollmentContractReviewError,
> {
    validate_restore_contract_reference(restore_contract)?;

    if draft.contract_domain != PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN {
        return Err(NativeNewDeviceEnrollmentContractReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION {
        return Err(NativeNewDeviceEnrollmentContractReviewError::ContractVersionMismatch);
    }

    if draft.platform_family != restore_contract.platform_family {
        return Err(NativeNewDeviceEnrollmentContractReviewError::PlatformFamilyMismatch);
    }

    if draft.restore_contract_domain != restore_contract.contract_domain
        || draft.restore_contract_domain != PHASE7A_RESTORE_CONTRACT_DOMAIN
    {
        return Err(NativeNewDeviceEnrollmentContractReviewError::RestoreContractDomainMismatch);
    }

    if draft.restore_contract_version != restore_contract.contract_version
        || draft.restore_contract_version != PHASE7A_RESTORE_CONTRACT_VERSION
    {
        return Err(NativeNewDeviceEnrollmentContractReviewError::RestoreContractVersionMismatch);
    }

    validate_target_device_class(draft.target_device_class)?;
    validate_enrollment_intents(&draft.requested_enrollment_intents)?;
    validate_enrollment_sources(&draft.requested_enrollment_sources)?;

    if draft.requests_enrollment_execution
        || draft.requests_device_key_generation
        || draft.requests_device_authorization_signature
        || draft.requests_signing_or_verification_runtime
        || draft.requests_capability_issuance
        || draft.requests_restore_execution
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
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeNewDeviceEnrollmentContractReviewError::UnsafeEnrollmentAuthorityFlag);
    }

    Ok(NativeNewDeviceEnrollmentContractDescriptorV1 {
        contract_domain: PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_DOMAIN,
        contract_version: PHASE7B_NEW_DEVICE_ENROLLMENT_CONTRACT_VERSION,
        platform_family: draft.platform_family,
        restore_contract_domain: PHASE7A_RESTORE_CONTRACT_DOMAIN,
        restore_contract_version: PHASE7A_RESTORE_CONTRACT_VERSION,
        target_device_class: draft.target_device_class,
        enrollment_intents: draft.requested_enrollment_intents,
        enrollment_sources: draft.requested_enrollment_sources,
        contract_only: true,
    })
}

fn validate_restore_contract_reference(
    restore_contract: &NativeRestoreContractDescriptorV1,
) -> Result<(), NativeNewDeviceEnrollmentContractReviewError> {
    if restore_contract.contract_domain != PHASE7A_RESTORE_CONTRACT_DOMAIN {
        return Err(NativeNewDeviceEnrollmentContractReviewError::RestoreContractDomainMismatch);
    }

    if restore_contract.contract_version != PHASE7A_RESTORE_CONTRACT_VERSION {
        return Err(NativeNewDeviceEnrollmentContractReviewError::RestoreContractVersionMismatch);
    }

    if !restore_contract.contract_only {
        return Err(NativeNewDeviceEnrollmentContractReviewError::RestoreContractNotContractOnly);
    }

    Ok(())
}

fn validate_target_device_class(
    target_device_class: DeviceClass,
) -> Result<(), NativeNewDeviceEnrollmentContractReviewError> {
    if !PHASE7B_ALLOWED_TARGET_DEVICE_CLASSES.contains(&target_device_class) {
        return Err(NativeNewDeviceEnrollmentContractReviewError::UnsupportedTargetDeviceClass);
    }

    Ok(())
}

fn validate_enrollment_intents(
    intents: &[NativeEnrollmentIntent],
) -> Result<(), NativeNewDeviceEnrollmentContractReviewError> {
    if intents.is_empty() {
        return Err(NativeNewDeviceEnrollmentContractReviewError::MissingEnrollmentIntents);
    }

    for (index, intent) in intents.iter().enumerate() {
        if intents[..index].contains(intent) {
            return Err(NativeNewDeviceEnrollmentContractReviewError::DuplicateEnrollmentIntent);
        }
    }

    for required in PHASE7B_REQUIRED_ENROLLMENT_INTENTS {
        if !intents.contains(required) {
            return Err(
                NativeNewDeviceEnrollmentContractReviewError::MissingRequiredEnrollmentIntent,
            );
        }
    }

    Ok(())
}

fn validate_enrollment_sources(
    sources: &[NativeEnrollmentSource],
) -> Result<(), NativeNewDeviceEnrollmentContractReviewError> {
    if sources.is_empty() {
        return Err(NativeNewDeviceEnrollmentContractReviewError::MissingEnrollmentSources);
    }

    for (index, source) in sources.iter().enumerate() {
        if sources[..index].contains(source) {
            return Err(NativeNewDeviceEnrollmentContractReviewError::DuplicateEnrollmentSource);
        }

        if !PHASE7B_ALLOWED_ENROLLMENT_SOURCES.contains(source) {
            return Err(NativeNewDeviceEnrollmentContractReviewError::UnsupportedEnrollmentSource);
        }
    }

    for required in PHASE7B_REQUIRED_ENROLLMENT_SOURCES {
        if !sources.contains(required) {
            return Err(
                NativeNewDeviceEnrollmentContractReviewError::MissingRequiredEnrollmentSource,
            );
        }
    }

    Ok(())
}
