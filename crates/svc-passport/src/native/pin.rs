//! RO:WHAT — Native Passport PIN unlock contract DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines public PIN policy metadata bound to the vault header before PIN validation, derivation, unlock, storage, or encryption runtime exists.
//! RO:INTERACTS — Phase 6A vault header DTOs, Phase 5B secure-surface contract, Phase 5A platform sealer contract.
//! RO:INVARIANTS — PIN unlock contract DTOs carry only public policy metadata, domains, version, KDF label, and boundary flags; no PIN value, PIN hash, derived key, ciphertext, plaintext, sealed bytes, vault runtime, storage, encryption/decryption, routes, signing, capabilities, wallet, or ledger authority are added.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects domain/version drift, platform mismatch, vault header mismatch, KDF label drift, invalid public policy bounds, and flags implying PIN runtime, derivation runtime, vault unlock, crypto runtime, storage, export, I/O, or wallet/ledger mutation.
//! RO:TEST — tests/native_passport_phase6b_pin_unlock_contract_dto.rs.

use super::{
    NativePlatformFamily, NativeTwoCompartmentVaultHeaderDescriptorV1, PHASE6A_KDF_ALGORITHM_LABEL,
    PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN, PHASE6A_VAULT_HEADER_VERSION,
};

/// Phase label for native PIN unlock contract DTO foundations.
pub const NATIVE_PASSPORT_PHASE6B_LABEL: &str = "NATIVE_PASSPORT_PHASE6B_PIN_UNLOCK_CONTRACT_DTO";

/// Canonical PIN unlock contract DTO domain.
pub const PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN: &str = "native-passport/pin-unlock-contract/v1";

/// PIN contract version.
pub const PHASE6B_PIN_UNLOCK_CONTRACT_VERSION: u16 = 1;

/// Minimum PIN length accepted by the public contract.
pub const PHASE6B_MIN_PIN_LENGTH: u8 = 6;

/// Maximum PIN length accepted by the public contract.
pub const PHASE6B_MAX_PIN_LENGTH: u8 = 64;

/// Maximum attempts named by the public contract before platform/client cooldown policy applies.
pub const PHASE6B_MAX_UNLOCK_ATTEMPTS: u8 = 10;

/// Cooldown seconds named by the public contract after maximum attempts.
pub const PHASE6B_COOLDOWN_SECONDS: u16 = 300;

/// Authority meanings that Phase 6B DTOs must not grant.
pub const PHASE6B_FORBIDDEN_PIN_UNLOCK_AUTHORITY_FLAGS: &[&str] = &[
    "pin_validation_runtime",
    "pin_derivation_runtime",
    "pin_secret_storage",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_implementation",
    "secret_storage",
    "material_export",
    "encryption_runtime",
    "decryption_runtime",
    "runtime_io",
    "signing_runtime",
    "signature_verification_runtime",
    "capability_issuance",
    "wallet_spend",
    "ledger_mutation",
];

/// Public PIN policy metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativePinPolicyV1 {
    /// Minimum PIN length.
    pub min_pin_length: u8,
    /// Maximum PIN length.
    pub max_pin_length: u8,
    /// Maximum attempts before cooldown.
    pub max_unlock_attempts: u8,
    /// Cooldown seconds after maximum attempts.
    pub cooldown_seconds: u16,
    /// KDF label this public policy expects.
    pub kdf_algorithm_label: &'static str,
}

/// Required public PIN policy.
pub const PHASE6B_REQUIRED_PIN_POLICY: NativePinPolicyV1 = NativePinPolicyV1 {
    min_pin_length: PHASE6B_MIN_PIN_LENGTH,
    max_pin_length: PHASE6B_MAX_PIN_LENGTH,
    max_unlock_attempts: PHASE6B_MAX_UNLOCK_ATTEMPTS,
    cooldown_seconds: PHASE6B_COOLDOWN_SECONDS,
    kdf_algorithm_label: PHASE6A_KDF_ALGORITHM_LABEL,
};

/// Public PIN unlock contract descriptor without PIN runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePinUnlockContractDescriptorV1 {
    /// PIN unlock contract domain.
    pub contract_domain: &'static str,
    /// PIN unlock contract version.
    pub contract_version: u16,
    /// Platform family inherited from the vault header.
    pub platform_family: NativePlatformFamily,
    /// Vault header domain this contract binds to.
    pub vault_header_domain: &'static str,
    /// Vault header version this contract binds to.
    pub vault_header_version: u16,
    /// Public PIN policy metadata.
    pub pin_policy: NativePinPolicyV1,
    /// Whether this descriptor is contract-only.
    pub contract_only: bool,
}

/// Public PIN unlock contract draft before PIN checking, derivation, unlock, or storage exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePinUnlockContractDraftV1 {
    /// PIN unlock contract domain.
    pub contract_domain: &'static str,
    /// PIN unlock contract version.
    pub contract_version: u16,
    /// Platform family expected from the vault header.
    pub platform_family: NativePlatformFamily,
    /// Vault header domain this contract binds to.
    pub vault_header_domain: &'static str,
    /// Vault header version this contract binds to.
    pub vault_header_version: u16,
    /// Requested public PIN policy metadata.
    pub requested_policy: NativePinPolicyV1,
    /// Boundary flag: this DTO must not validate PIN values.
    pub requests_pin_validation_runtime: bool,
    /// Boundary flag: this DTO must not derive PIN keys.
    pub requests_pin_derivation_runtime: bool,
    /// Boundary flag: this DTO must not contain or store PIN secrets.
    pub stores_pin_secret_material: bool,
    /// Boundary flag: this DTO must not unlock a vault.
    pub requests_vault_unlock: bool,
    /// Boundary flag: this DTO must not include vault runtime.
    pub includes_vault_runtime: bool,
    /// Boundary flag: this DTO must not include platform sealer implementation.
    pub includes_platform_sealer_implementation: bool,
    /// Boundary flag: this DTO must not store secret material.
    pub stores_secret_material: bool,
    /// Boundary flag: this DTO must not export material.
    pub exports_material: bool,
    /// Boundary flag: this DTO must not encrypt or decrypt data.
    pub requests_encryption_or_decryption: bool,
    /// Boundary flag: this DTO must not perform runtime I/O.
    pub requests_runtime_io: bool,
    /// Boundary flag: this DTO must not mutate wallet or ledger state.
    pub requests_wallet_or_ledger_mutation: bool,
}

/// Phase 6B PIN unlock contract review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePinUnlockContractReviewError {
    /// PIN unlock contract domain must match.
    ContractDomainMismatch,
    /// PIN unlock contract version must match.
    ContractVersionMismatch,
    /// Platform family must match the vault header.
    PlatformFamilyMismatch,
    /// Vault header domain must match.
    VaultHeaderDomainMismatch,
    /// Vault header version must match.
    VaultHeaderVersionMismatch,
    /// PIN policy bounds must match the locked public policy.
    PinPolicyMismatch,
    /// DTO flags attempted to carry or exercise unsafe PIN unlock authority.
    UnsafePinUnlockAuthorityFlag,
}

/// Phase 6B posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportPinUnlockPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds PIN unlock contract DTOs.
    pub pin_unlock_contract_dtos_added: bool,
    /// Whether this phase adds PIN validation runtime.
    pub pin_validation_runtime_added: bool,
    /// Whether this phase adds PIN derivation runtime.
    pub pin_derivation_runtime_added: bool,
    /// Whether this phase stores PIN secrets.
    pub pin_secret_storage_added: bool,
    /// Whether this phase adds vault unlock.
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
    /// Whether this phase adds runtime I/O.
    pub runtime_io_added: bool,
    /// Whether this phase adds signing runtime.
    pub signing_runtime_added: bool,
    /// Whether this phase adds signature/proof verification runtime.
    pub signature_verification_runtime_added: bool,
    /// Whether this phase issues capabilities.
    pub capability_issuance_added: bool,
    /// Whether this phase changes runtime authority.
    pub runtime_authority_changed: bool,
    /// Whether this phase adds native secret custody.
    pub native_secret_implementation_added: bool,
    /// Forbidden authority meanings.
    pub forbidden_authority_flags: &'static [&'static str],
}

/// Return Phase 6B PIN unlock contract posture.
pub fn native_passport_pin_unlock_posture() -> NativePassportPinUnlockPosture {
    NativePassportPinUnlockPosture {
        phase_label: NATIVE_PASSPORT_PHASE6B_LABEL,
        pin_unlock_contract_dtos_added: true,
        pin_validation_runtime_added: false,
        pin_derivation_runtime_added: false,
        pin_secret_storage_added: false,
        vault_unlock_added: false,
        vault_runtime_added: false,
        platform_sealer_implementation_added: false,
        secret_storage_added: false,
        material_export_added: false,
        encryption_runtime_added: false,
        decryption_runtime_added: false,
        runtime_io_added: false,
        signing_runtime_added: false,
        signature_verification_runtime_added: false,
        capability_issuance_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE6B_FORBIDDEN_PIN_UNLOCK_AUTHORITY_FLAGS,
    }
}

/// Validate a PIN unlock contract descriptor without validating PINs or unlocking vaults.
pub fn validate_native_pin_unlock_contract_descriptor(
    vault_header: &NativeTwoCompartmentVaultHeaderDescriptorV1,
    descriptor: &NativePinUnlockContractDescriptorV1,
) -> Result<(), NativePinUnlockContractReviewError> {
    if descriptor.contract_domain != PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN {
        return Err(NativePinUnlockContractReviewError::ContractDomainMismatch);
    }

    if descriptor.contract_version != PHASE6B_PIN_UNLOCK_CONTRACT_VERSION {
        return Err(NativePinUnlockContractReviewError::ContractVersionMismatch);
    }

    if descriptor.platform_family != vault_header.platform_family {
        return Err(NativePinUnlockContractReviewError::PlatformFamilyMismatch);
    }

    if descriptor.vault_header_domain != vault_header.header_domain
        || descriptor.vault_header_domain != PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN
    {
        return Err(NativePinUnlockContractReviewError::VaultHeaderDomainMismatch);
    }

    if descriptor.vault_header_version != vault_header.header_version
        || descriptor.vault_header_version != PHASE6A_VAULT_HEADER_VERSION
    {
        return Err(NativePinUnlockContractReviewError::VaultHeaderVersionMismatch);
    }

    validate_pin_policy(descriptor.pin_policy)
}

/// Review a PIN unlock contract draft into a descriptor.
///
/// This is contract DTO review only. It does not inspect PIN values, derive keys,
/// unlock vaults, call OS secure storage, store secrets, export material, encrypt,
/// decrypt, perform I/O, sign, verify, issue capabilities, mutate wallets, or mutate ledgers.
pub fn review_native_pin_unlock_contract_draft(
    vault_header: &NativeTwoCompartmentVaultHeaderDescriptorV1,
    draft: NativePinUnlockContractDraftV1,
) -> Result<NativePinUnlockContractDescriptorV1, NativePinUnlockContractReviewError> {
    if draft.contract_domain != PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN {
        return Err(NativePinUnlockContractReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE6B_PIN_UNLOCK_CONTRACT_VERSION {
        return Err(NativePinUnlockContractReviewError::ContractVersionMismatch);
    }

    if draft.platform_family != vault_header.platform_family {
        return Err(NativePinUnlockContractReviewError::PlatformFamilyMismatch);
    }

    if draft.vault_header_domain != vault_header.header_domain
        || draft.vault_header_domain != PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN
    {
        return Err(NativePinUnlockContractReviewError::VaultHeaderDomainMismatch);
    }

    if draft.vault_header_version != vault_header.header_version
        || draft.vault_header_version != PHASE6A_VAULT_HEADER_VERSION
    {
        return Err(NativePinUnlockContractReviewError::VaultHeaderVersionMismatch);
    }

    validate_pin_policy(draft.requested_policy)?;

    if draft.requests_pin_validation_runtime
        || draft.requests_pin_derivation_runtime
        || draft.stores_pin_secret_material
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.includes_platform_sealer_implementation
        || draft.stores_secret_material
        || draft.exports_material
        || draft.requests_encryption_or_decryption
        || draft.requests_runtime_io
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativePinUnlockContractReviewError::UnsafePinUnlockAuthorityFlag);
    }

    Ok(NativePinUnlockContractDescriptorV1 {
        contract_domain: PHASE6B_PIN_UNLOCK_CONTRACT_DOMAIN,
        contract_version: PHASE6B_PIN_UNLOCK_CONTRACT_VERSION,
        platform_family: draft.platform_family,
        vault_header_domain: PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN,
        vault_header_version: PHASE6A_VAULT_HEADER_VERSION,
        pin_policy: draft.requested_policy,
        contract_only: true,
    })
}

fn validate_pin_policy(
    policy: NativePinPolicyV1,
) -> Result<(), NativePinUnlockContractReviewError> {
    if policy != PHASE6B_REQUIRED_PIN_POLICY {
        return Err(NativePinUnlockContractReviewError::PinPolicyMismatch);
    }

    Ok(())
}
