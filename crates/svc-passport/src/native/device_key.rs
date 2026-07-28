//! RO:WHAT — Native Passport device-key DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines delegated device-key public metadata after recovery-root DTOs and before generation, sealing, signing, or proof review exists.
//! RO:INTERACTS — Native Passport canonical IDs, Phase 4A recovery-root descriptor, Phase 3 device authorization DTOs.
//! RO:INVARIANTS — device-key DTOs carry only public identifiers, public keys, class, domain, public purpose labels, and boundary flags; no private key, vault unlock, platform sealer, signing, verification, route, capability, wallet, or ledger authority is added.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects Passport mismatches, recovery-root/device-key collapse, empty/duplicate device purposes, domain mismatches, and flags implying secret custody, key generation, material export, signing runtime, vault unlock, or wallet/ledger mutation.
//! RO:TEST — tests/native_passport_phase4b_device_key_foundation_dto.rs.

use super::{
    DeviceClass, DeviceIdV1, Ed25519PublicKeyHex, NativeRecoveryRootDescriptorV1, PassportIdV1,
};

/// Phase label for native device-key DTO foundations.
pub const NATIVE_PASSPORT_PHASE4B_LABEL: &str = "NATIVE_PASSPORT_PHASE4B_DEVICE_KEY_FOUNDATION_DTO";

/// Canonical local device-key DTO domain.
pub const PHASE4B_DEVICE_KEY_DOMAIN: &str = "native-passport/device-key/v1";

/// Device-key purposes allowed in Phase 4B DTOs.
pub const PHASE4B_ALLOWED_DEVICE_KEY_PURPOSES: &[NativeDeviceKeyPurpose] = &[
    NativeDeviceKeyPurpose::AuthenticateDevice,
    NativeDeviceKeyPurpose::SignRequestProofs,
    NativeDeviceKeyPurpose::ReadPassportData,
    NativeDeviceKeyPurpose::RevokeSelf,
];

/// Authority meanings that Phase 4B DTOs must not grant.
pub const PHASE4B_FORBIDDEN_DEVICE_KEY_AUTHORITY_FLAGS: &[&str] = &[
    "generate_device_key",
    "store_device_secret",
    "export_device_secret",
    "platform_sealer",
    "unlock_vault",
    "signing_runtime",
    "signature_verification_runtime",
    "capability_issuance",
    "wallet_spend",
    "ledger_mutation",
];

/// Public device-key purpose labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeDeviceKeyPurpose {
    /// Public purpose label for future device authentication.
    AuthenticateDevice,
    /// Public purpose label for future request-proof signing.
    SignRequestProofs,
    /// Public purpose label for read-only Passport data access.
    ReadPassportData,
    /// Public purpose label for future self-revocation.
    RevokeSelf,
}

/// Public device-key descriptor without secret material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceKeyDescriptorV1 {
    /// Canonical Passport ID.
    pub passport_id: PassportIdV1,
    /// Canonical Device ID.
    pub device_id: DeviceIdV1,
    /// Public device key.
    pub device_public_key: Ed25519PublicKeyHex,
    /// Device class.
    pub device_class: DeviceClass,
    /// Device-key DTO domain.
    pub device_key_domain: &'static str,
    /// Allowed public purpose labels.
    pub allowed_purposes: Vec<NativeDeviceKeyPurpose>,
}

/// Public device-key draft before generation, sealing, storage, signing, or proof review exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDeviceKeyDraftV1 {
    /// Canonical Passport ID.
    pub passport_id: PassportIdV1,
    /// Canonical Device ID.
    pub device_id: DeviceIdV1,
    /// Public device key.
    pub device_public_key: Ed25519PublicKeyHex,
    /// Device class.
    pub device_class: DeviceClass,
    /// Device-key DTO domain.
    pub device_key_domain: &'static str,
    /// Requested public purpose labels.
    pub requested_purposes: Vec<NativeDeviceKeyPurpose>,
    /// Boundary flag: this DTO must not generate device keys.
    pub requests_device_key_generation: bool,
    /// Boundary flag: this DTO must not contain device secret material.
    pub contains_device_secret_material: bool,
    /// Boundary flag: this DTO must not export device secret material.
    pub exports_device_secret_material: bool,
    /// Boundary flag: this DTO must not request signing runtime.
    pub requests_signing_runtime: bool,
    /// Boundary flag: this DTO must not unlock a vault.
    pub requests_vault_unlock: bool,
    /// Boundary flag: this DTO must not mutate wallet or ledger state.
    pub requests_wallet_or_ledger_mutation: bool,
}

/// Phase 4B device-key review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDeviceKeyReviewError {
    /// Draft Passport ID must match the recovery-root descriptor Passport ID.
    PassportMismatch,
    /// Device-key domain must match the native device-key domain.
    DeviceKeyDomainMismatch,
    /// At least one device-key purpose is required.
    MissingPurposes,
    /// Duplicate device-key purposes are rejected for deterministic review.
    DuplicatePurpose,
    /// Device public key must not equal the recovery-root public key.
    RecoveryRootDeviceKeyCollision,
    /// DTO flags attempted to carry or exercise unsafe device-key authority.
    UnsafeDeviceKeyAuthorityFlag,
}

/// Phase 4B posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportDeviceKeyPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds device-key DTOs.
    pub device_key_dtos_added: bool,
    /// Whether this phase generates device keys.
    pub device_key_generation_added: bool,
    /// Whether this phase stores device secrets.
    pub device_secret_storage_added: bool,
    /// Whether this phase adds platform sealer integration.
    pub platform_sealer_added: bool,
    /// Whether this phase adds signing runtime.
    pub signing_runtime_added: bool,
    /// Whether this phase adds signature/proof verification runtime.
    pub signature_verification_runtime_added: bool,
    /// Whether this phase adds vault runtime.
    pub vault_runtime_added: bool,
    /// Whether this phase issues capabilities.
    pub capability_issuance_added: bool,
    /// Whether this phase changes runtime authority.
    pub runtime_authority_changed: bool,
    /// Whether this phase adds native secret custody.
    pub native_secret_implementation_added: bool,
    /// Forbidden authority meanings.
    pub forbidden_authority_flags: &'static [&'static str],
}

/// Return Phase 4B device-key posture.
pub fn native_passport_device_key_posture() -> NativePassportDeviceKeyPosture {
    NativePassportDeviceKeyPosture {
        phase_label: NATIVE_PASSPORT_PHASE4B_LABEL,
        device_key_dtos_added: true,
        device_key_generation_added: false,
        device_secret_storage_added: false,
        platform_sealer_added: false,
        signing_runtime_added: false,
        signature_verification_runtime_added: false,
        vault_runtime_added: false,
        capability_issuance_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE4B_FORBIDDEN_DEVICE_KEY_AUTHORITY_FLAGS,
    }
}

/// Validate a device-key descriptor without generation, storage, sealing, signing, or proof review.
pub fn validate_native_device_key_descriptor(
    recovery_root: &NativeRecoveryRootDescriptorV1,
    descriptor: &NativeDeviceKeyDescriptorV1,
) -> Result<(), NativeDeviceKeyReviewError> {
    if descriptor.passport_id.as_str() != recovery_root.passport_id.as_str() {
        return Err(NativeDeviceKeyReviewError::PassportMismatch);
    }

    if descriptor.device_key_domain != PHASE4B_DEVICE_KEY_DOMAIN {
        return Err(NativeDeviceKeyReviewError::DeviceKeyDomainMismatch);
    }

    if descriptor.device_public_key.as_str() == recovery_root.recovery_root_public_key.as_str() {
        return Err(NativeDeviceKeyReviewError::RecoveryRootDeviceKeyCollision);
    }

    validate_device_key_purposes(&descriptor.allowed_purposes)
}

/// Review a device-key draft into a public descriptor.
///
/// This is DTO review only. It does not generate device keys, store secret material,
/// unlock vaults, call platform sealers, sign messages, verify signatures, issue capabilities,
/// mutate wallets, or mutate ledgers.
pub fn review_native_device_key_draft(
    recovery_root: &NativeRecoveryRootDescriptorV1,
    draft: NativeDeviceKeyDraftV1,
) -> Result<NativeDeviceKeyDescriptorV1, NativeDeviceKeyReviewError> {
    if draft.passport_id.as_str() != recovery_root.passport_id.as_str() {
        return Err(NativeDeviceKeyReviewError::PassportMismatch);
    }

    if draft.device_key_domain != PHASE4B_DEVICE_KEY_DOMAIN {
        return Err(NativeDeviceKeyReviewError::DeviceKeyDomainMismatch);
    }

    validate_device_key_purposes(&draft.requested_purposes)?;

    if draft.device_public_key.as_str() == recovery_root.recovery_root_public_key.as_str() {
        return Err(NativeDeviceKeyReviewError::RecoveryRootDeviceKeyCollision);
    }

    if draft.requests_device_key_generation
        || draft.contains_device_secret_material
        || draft.exports_device_secret_material
        || draft.requests_signing_runtime
        || draft.requests_vault_unlock
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeDeviceKeyReviewError::UnsafeDeviceKeyAuthorityFlag);
    }

    Ok(NativeDeviceKeyDescriptorV1 {
        passport_id: draft.passport_id,
        device_id: draft.device_id,
        device_public_key: draft.device_public_key,
        device_class: draft.device_class,
        device_key_domain: PHASE4B_DEVICE_KEY_DOMAIN,
        allowed_purposes: draft.requested_purposes,
    })
}

fn validate_device_key_purposes(
    purposes: &[NativeDeviceKeyPurpose],
) -> Result<(), NativeDeviceKeyReviewError> {
    if purposes.is_empty() {
        return Err(NativeDeviceKeyReviewError::MissingPurposes);
    }

    for (index, purpose) in purposes.iter().enumerate() {
        if purposes[..index].contains(purpose) {
            return Err(NativeDeviceKeyReviewError::DuplicatePurpose);
        }
    }

    Ok(())
}
