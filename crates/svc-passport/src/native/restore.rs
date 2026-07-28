//! RO:WHAT — Native Passport restore contract DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines restore/new-device/delegated-enrollment contract metadata before restore execution, key derivation, vault unlock, delegated enrollment runtime, or route integration exists.
//! RO:INTERACTS — Phase 6B PIN unlock contract, Phase 6A vault header, Phase 5 secure-surface contracts.
//! RO:INVARIANTS — restore contract DTOs carry only public domains, versions, intent labels, source labels, and boundary flags; no mnemonic, seed phrase, private key, PIN value, derived key, ciphertext, plaintext, sealed bytes, storage, vault unlock, enrollment execution, routes, capabilities, wallet, or ledger authority are added.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects domain/version drift, platform mismatch, PIN contract mismatch, vault header mismatch, empty/duplicate restore intents, invalid source labels, and flags implying restore execution, key derivation, vault unlock, secret import/export/storage, crypto runtime, delegated enrollment runtime, route/runtime I/O, or wallet/ledger mutation.
//! RO:TEST — tests/native_passport_phase7a_restore_contract_dto.rs.

use super::{
    NativePinUnlockContractDescriptorV1, NativePlatformFamily,
    PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN, PHASE6A_VAULT_HEADER_VERSION,
    PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN, PHASE6B_PIN_UNLOCK_CONTRACT_VERSION,
};

/// Phase label for native restore contract DTO foundations.
pub const NATIVE_PASSPORT_PHASE7A_LABEL: &str = "NATIVE_PASSPORT_PHASE7A_RESTORE_CONTRACT_DTO";

/// Canonical restore contract DTO domain.
pub const PHASE7A_RESTORE_CONTRACT_DOMAIN: &str = "native-passport/restore-contract/v1";

/// Restore contract version.
pub const PHASE7A_RESTORE_CONTRACT_VERSION: u16 = 1;

/// Required restore intents.
pub const PHASE7A_REQUIRED_RESTORE_INTENTS: &[NativeRestoreIntent] = &[
    NativeRestoreIntent::RestorePassportRoot,
    NativeRestoreIntent::RestoreDeviceKey,
    NativeRestoreIntent::PrepareDelegatedEnrollment,
];

/// Allowed restore source labels.
pub const PHASE7A_ALLOWED_RESTORE_SOURCES: &[NativeRestoreSource] = &[
    NativeRestoreSource::LocalVaultHeader,
    NativeRestoreSource::PlatformSealerContract,
    NativeRestoreSource::DelegatedDeviceInvitation,
];

/// Authority meanings that Phase 7A DTOs must not grant.
pub const PHASE7A_FORBIDDEN_RESTORE_AUTHORITY_FLAGS: &[&str] = &[
    "restore_execution",
    "mnemonic_import",
    "seed_phrase_import",
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
    "delegated_enrollment_runtime",
    "runtime_io",
    "route_added",
    "capability_issuance",
    "wallet_spend",
    "ledger_mutation",
];

/// Public restore intent labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeRestoreIntent {
    /// Restore the Passport recovery/root compartment later.
    RestorePassportRoot,
    /// Restore the local device-key compartment later.
    RestoreDeviceKey,
    /// Prepare a delegated enrollment handshake later.
    PrepareDelegatedEnrollment,
}

/// Public restore source labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeRestoreSource {
    /// Restore source refers to a local vault header contract.
    LocalVaultHeader,
    /// Restore source refers to the platform sealer contract surface.
    PlatformSealerContract,
    /// Restore source refers to a delegated device invitation.
    DelegatedDeviceInvitation,
}

/// Public restore contract descriptor without restore runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRestoreContractDescriptorV1 {
    /// Restore contract domain.
    pub contract_domain: &'static str,
    /// Restore contract version.
    pub contract_version: u16,
    /// Platform family inherited from the PIN contract.
    pub platform_family: NativePlatformFamily,
    /// PIN contract domain this restore contract binds to.
    pub pin_contract_domain: &'static str,
    /// PIN contract version this restore contract binds to.
    pub pin_contract_version: u16,
    /// Vault header domain this restore contract binds to.
    pub vault_header_domain: &'static str,
    /// Vault header version this restore contract binds to.
    pub vault_header_version: u16,
    /// Public restore intent labels.
    pub restore_intents: Vec<NativeRestoreIntent>,
    /// Public restore source labels.
    pub restore_sources: Vec<NativeRestoreSource>,
    /// Whether this descriptor is contract-only.
    pub contract_only: bool,
}

/// Public restore contract draft before restore, import, derivation, unlock, or enrollment runtime exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRestoreContractDraftV1 {
    /// Restore contract domain.
    pub contract_domain: &'static str,
    /// Restore contract version.
    pub contract_version: u16,
    /// Platform family expected from the PIN contract.
    pub platform_family: NativePlatformFamily,
    /// PIN contract domain this restore contract binds to.
    pub pin_contract_domain: &'static str,
    /// PIN contract version this restore contract binds to.
    pub pin_contract_version: u16,
    /// Vault header domain this restore contract binds to.
    pub vault_header_domain: &'static str,
    /// Vault header version this restore contract binds to.
    pub vault_header_version: u16,
    /// Requested public restore intents.
    pub requested_restore_intents: Vec<NativeRestoreIntent>,
    /// Requested public restore sources.
    pub requested_restore_sources: Vec<NativeRestoreSource>,
    /// Boundary flag: this DTO must not execute restore.
    pub requests_restore_execution: bool,
    /// Boundary flag: this DTO must not import mnemonic or seed phrase material.
    pub imports_recovery_phrase_material: bool,
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
    /// Boundary flag: this DTO must not execute delegated enrollment.
    pub requests_delegated_enrollment_runtime: bool,
    /// Boundary flag: this DTO must not perform runtime I/O.
    pub requests_runtime_io: bool,
    /// Boundary flag: this DTO must not add routes.
    pub adds_routes: bool,
    /// Boundary flag: this DTO must not issue capabilities.
    pub issues_capabilities: bool,
    /// Boundary flag: this DTO must not mutate wallet or ledger state.
    pub requests_wallet_or_ledger_mutation: bool,
}

/// Phase 7A restore contract review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeRestoreContractReviewError {
    /// Restore contract domain must match.
    ContractDomainMismatch,
    /// Restore contract version must match.
    ContractVersionMismatch,
    /// Platform family must match the PIN contract.
    PlatformFamilyMismatch,
    /// PIN contract domain must match.
    PinContractDomainMismatch,
    /// PIN contract version must match.
    PinContractVersionMismatch,
    /// Vault header domain must match.
    VaultHeaderDomainMismatch,
    /// Vault header version must match.
    VaultHeaderVersionMismatch,
    /// At least one restore intent is required.
    MissingRestoreIntents,
    /// Duplicate restore intents are rejected for deterministic review.
    DuplicateRestoreIntent,
    /// At least one restore source is required.
    MissingRestoreSources,
    /// Duplicate restore sources are rejected for deterministic review.
    DuplicateRestoreSource,
    /// Required restore intents must be present.
    MissingRequiredRestoreIntent,
    /// Restore source labels must be known.
    UnsupportedRestoreSource,
    /// DTO flags attempted to carry or exercise unsafe restore authority.
    UnsafeRestoreAuthorityFlag,
}

/// Phase 7A posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportRestorePosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds restore contract DTOs.
    pub restore_contract_dtos_added: bool,
    /// Whether this phase executes restore.
    pub restore_execution_added: bool,
    /// Whether this phase imports mnemonic/seed phrase material.
    pub mnemonic_or_seed_import_added: bool,
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
    /// Whether this phase adds delegated enrollment runtime.
    pub delegated_enrollment_runtime_added: bool,
    /// Whether this phase adds runtime I/O.
    pub runtime_io_added: bool,
    /// Whether this phase adds routes.
    pub routes_added: bool,
    /// Whether this phase issues capabilities.
    pub capability_issuance_added: bool,
    /// Whether this phase changes runtime authority.
    pub runtime_authority_changed: bool,
    /// Whether this phase adds native secret custody.
    pub native_secret_implementation_added: bool,
    /// Forbidden authority meanings.
    pub forbidden_authority_flags: &'static [&'static str],
}

/// Return Phase 7A restore contract posture.
pub fn native_passport_restore_posture() -> NativePassportRestorePosture {
    NativePassportRestorePosture {
        phase_label: NATIVE_PASSPORT_PHASE7A_LABEL,
        restore_contract_dtos_added: true,
        restore_execution_added: false,
        mnemonic_or_seed_import_added: false,
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
        delegated_enrollment_runtime_added: false,
        runtime_io_added: false,
        routes_added: false,
        capability_issuance_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE7A_FORBIDDEN_RESTORE_AUTHORITY_FLAGS,
    }
}

/// Validate a restore contract descriptor without executing restore.
pub fn validate_native_restore_contract_descriptor(
    pin_contract: &NativePinUnlockContractDescriptorV1,
    descriptor: &NativeRestoreContractDescriptorV1,
) -> Result<(), NativeRestoreContractReviewError> {
    if descriptor.contract_domain != PHASE7A_RESTORE_CONTRACT_DOMAIN {
        return Err(NativeRestoreContractReviewError::ContractDomainMismatch);
    }

    if descriptor.contract_version != PHASE7A_RESTORE_CONTRACT_VERSION {
        return Err(NativeRestoreContractReviewError::ContractVersionMismatch);
    }

    if descriptor.platform_family != pin_contract.platform_family {
        return Err(NativeRestoreContractReviewError::PlatformFamilyMismatch);
    }

    if descriptor.pin_contract_domain != pin_contract.contract_domain
        || descriptor.pin_contract_domain != PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN
    {
        return Err(NativeRestoreContractReviewError::PinContractDomainMismatch);
    }

    if descriptor.pin_contract_version != pin_contract.contract_version
        || descriptor.pin_contract_version != PHASE6B_PIN_UNLOCK_CONTRACT_VERSION
    {
        return Err(NativeRestoreContractReviewError::PinContractVersionMismatch);
    }

    if descriptor.vault_header_domain != pin_contract.vault_header_domain
        || descriptor.vault_header_domain != PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN
    {
        return Err(NativeRestoreContractReviewError::VaultHeaderDomainMismatch);
    }

    if descriptor.vault_header_version != pin_contract.vault_header_version
        || descriptor.vault_header_version != PHASE6A_VAULT_HEADER_VERSION
    {
        return Err(NativeRestoreContractReviewError::VaultHeaderVersionMismatch);
    }

    validate_restore_intents(&descriptor.restore_intents)?;
    validate_restore_sources(&descriptor.restore_sources)
}

/// Review a restore contract draft into a descriptor.
///
/// This is contract DTO review only. It does not restore keys, import seed phrases,
/// derive keys, validate PINs, unlock vaults, call OS secure storage, store secrets,
/// export material, encrypt, decrypt, execute delegated enrollment, add routes,
/// issue capabilities, mutate wallets, or mutate ledgers.
pub fn review_native_restore_contract_draft(
    pin_contract: &NativePinUnlockContractDescriptorV1,
    draft: NativeRestoreContractDraftV1,
) -> Result<NativeRestoreContractDescriptorV1, NativeRestoreContractReviewError> {
    if draft.contract_domain != PHASE7A_RESTORE_CONTRACT_DOMAIN {
        return Err(NativeRestoreContractReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE7A_RESTORE_CONTRACT_VERSION {
        return Err(NativeRestoreContractReviewError::ContractVersionMismatch);
    }

    if draft.platform_family != pin_contract.platform_family {
        return Err(NativeRestoreContractReviewError::PlatformFamilyMismatch);
    }

    if draft.pin_contract_domain != pin_contract.contract_domain
        || draft.pin_contract_domain != PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN
    {
        return Err(NativeRestoreContractReviewError::PinContractDomainMismatch);
    }

    if draft.pin_contract_version != pin_contract.contract_version
        || draft.pin_contract_version != PHASE6B_PIN_UNLOCK_CONTRACT_VERSION
    {
        return Err(NativeRestoreContractReviewError::PinContractVersionMismatch);
    }

    if draft.vault_header_domain != pin_contract.vault_header_domain
        || draft.vault_header_domain != PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN
    {
        return Err(NativeRestoreContractReviewError::VaultHeaderDomainMismatch);
    }

    if draft.vault_header_version != pin_contract.vault_header_version
        || draft.vault_header_version != PHASE6A_VAULT_HEADER_VERSION
    {
        return Err(NativeRestoreContractReviewError::VaultHeaderVersionMismatch);
    }

    validate_restore_intents(&draft.requested_restore_intents)?;
    validate_restore_sources(&draft.requested_restore_sources)?;

    if draft.requests_restore_execution
        || draft.imports_recovery_phrase_material
        || draft.requests_key_derivation_runtime
        || draft.requests_pin_validation_runtime
        || draft.requests_pin_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.includes_platform_sealer_implementation
        || draft.stores_secret_material
        || draft.exports_secret_or_material
        || draft.requests_encryption_or_decryption
        || draft.requests_delegated_enrollment_runtime
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.issues_capabilities
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeRestoreContractReviewError::UnsafeRestoreAuthorityFlag);
    }

    Ok(NativeRestoreContractDescriptorV1 {
        contract_domain: PHASE7A_RESTORE_CONTRACT_DOMAIN,
        contract_version: PHASE7A_RESTORE_CONTRACT_VERSION,
        platform_family: draft.platform_family,
        pin_contract_domain: PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN,
        pin_contract_version: PHASE6B_PIN_UNLOCK_CONTRACT_VERSION,
        vault_header_domain: PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN,
        vault_header_version: PHASE6A_VAULT_HEADER_VERSION,
        restore_intents: draft.requested_restore_intents,
        restore_sources: draft.requested_restore_sources,
        contract_only: true,
    })
}

fn validate_restore_intents(
    intents: &[NativeRestoreIntent],
) -> Result<(), NativeRestoreContractReviewError> {
    if intents.is_empty() {
        return Err(NativeRestoreContractReviewError::MissingRestoreIntents);
    }

    for (index, intent) in intents.iter().enumerate() {
        if intents[..index].contains(intent) {
            return Err(NativeRestoreContractReviewError::DuplicateRestoreIntent);
        }
    }

    for required in PHASE7A_REQUIRED_RESTORE_INTENTS {
        if !intents.contains(required) {
            return Err(NativeRestoreContractReviewError::MissingRequiredRestoreIntent);
        }
    }

    Ok(())
}

fn validate_restore_sources(
    sources: &[NativeRestoreSource],
) -> Result<(), NativeRestoreContractReviewError> {
    if sources.is_empty() {
        return Err(NativeRestoreContractReviewError::MissingRestoreSources);
    }

    for (index, source) in sources.iter().enumerate() {
        if sources[..index].contains(source) {
            return Err(NativeRestoreContractReviewError::DuplicateRestoreSource);
        }

        if !PHASE7A_ALLOWED_RESTORE_SOURCES.contains(source) {
            return Err(NativeRestoreContractReviewError::UnsupportedRestoreSource);
        }
    }

    Ok(())
}
