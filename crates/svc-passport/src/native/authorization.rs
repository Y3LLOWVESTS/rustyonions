//! RO:WHAT — Native Passport root/device authorization DTO foundation.
//! RO:WHY — P3 Identity & Keys. Separates root Passport identity from delegated device keys before proof verification or vault runtime exists.
//! RO:INTERACTS — Native DTO IDs, ron-proto canonical IDs, ron-naming optional handles, future pure authorization verification.
//! RO:INVARIANTS — root keys identify Passport authority; device keys are delegated read-only actors; this module does not sign, verify signatures, unlock vaults, issue capabilities, mutate wallets, or mutate ledgers.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects empty/duplicate scope drafts, root/device key collapse, and any DTO flag that would imply root unlock, device issuance, wallet/ledger mutation, or capability issuance.
//! RO:TEST — tests/native_passport_phase3a_root_device_authorization_dto_foundation.rs.

use super::{
    DeviceClass, DeviceIdV1, Ed25519PublicKeyHex, HandleV1, NativePassportScope, PassportIdV1,
};

/// Phase label for root/device authorization DTO foundations.
pub const NATIVE_PASSPORT_PHASE3A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE3A_ROOT_DEVICE_AUTHORIZATION_DTO_FOUNDATION";

/// Phase label for pure device authorization review.
pub const NATIVE_PASSPORT_PHASE3B_LABEL: &str = "NATIVE_PASSPORT_PHASE3B_PURE_AUTHORIZATION_REVIEW";

/// Device scopes allowed in Phase 3A DTOs.
pub const PHASE3A_ALLOWED_DEVICE_SCOPES: &[NativePassportScope] = &[
    NativePassportScope::IdentityRead,
    NativePassportScope::CatalogRead,
    NativePassportScope::ContentRead,
    NativePassportScope::EntitlementRead,
    NativePassportScope::ReceiptsRead,
    NativePassportScope::ConfirmedRocRead,
    NativePassportScope::CapabilityRevokeSelf,
];

/// Authority meanings that Phase 3A DTOs must not grant.
pub const PHASE3A_FORBIDDEN_AUTHORITY_FLAGS: &[&str] = &[
    "root_unlock",
    "device_issuance",
    "capability_issuance",
    "wallet_spend",
    "ledger_mutation",
    "vault_decrypt",
    "native_secret_custody",
    "signing_runtime",
    "signature_verification_runtime",
];

/// Root Passport descriptor without private material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootPassportDescriptorV1 {
    /// Canonical Passport ID.
    pub passport_id: PassportIdV1,
    /// Root public key only.
    pub root_public_key: Ed25519PublicKeyHex,
    /// Optional public handle.
    pub optional_handle: Option<HandleV1>,
}

/// Device authorization draft before any signature/proof verification exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthorizationDraftV1 {
    /// Canonical Passport ID this device asks to join.
    pub passport_id: PassportIdV1,
    /// Root public key expected to authorize the device later.
    pub root_public_key: Ed25519PublicKeyHex,
    /// Canonical Device ID.
    pub device_id: DeviceIdV1,
    /// Device public key only.
    pub device_public_key: Ed25519PublicKeyHex,
    /// Device class.
    pub device_class: DeviceClass,
    /// Requested read-only scopes.
    pub requested_scopes: Vec<NativePassportScope>,
}

/// Device authorization grant DTO before proof/signature review is implemented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthorizationGrantV1 {
    /// Root descriptor.
    pub root: RootPassportDescriptorV1,
    /// Canonical Device ID.
    pub device_id: DeviceIdV1,
    /// Device public key only.
    pub device_public_key: Ed25519PublicKeyHex,
    /// Device class.
    pub device_class: DeviceClass,
    /// Allowed read-only scopes.
    pub allowed_scopes: Vec<NativePassportScope>,
    /// Whether the device can sign request proofs later.
    pub can_sign_request_proofs: bool,
    /// Whether the device can unlock root material.
    pub can_unlock_root: bool,
    /// Whether the device can authorize additional devices.
    pub can_authorize_devices: bool,
    /// Whether the device can issue capabilities.
    pub can_issue_capabilities: bool,
    /// Whether the device can mutate wallet or ledger state.
    pub can_mutate_wallet_or_ledger: bool,
}

/// Phase 3A DTO review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceAuthorizationReviewError {
    /// Draft Passport ID must match the reviewed root descriptor.
    PassportMismatch,
    /// Draft root public key must match the reviewed root descriptor.
    RootPublicKeyMismatch,
    /// At least one device scope is required.
    MissingScopes,
    /// Duplicate scopes are rejected for deterministic review.
    DuplicateScope,
    /// A scope is outside the Phase 3A read-only allow-list.
    UnsupportedScope,
    /// Root and device public keys must remain separate.
    RootDeviceKeyCollision,
    /// DTO flags attempted to grant root/device/capability authority.
    UnsafeAuthorityFlag,
}

/// Phase 3A posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportAuthorizationPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds root/device authorization DTOs.
    pub root_device_authorization_dtos_added: bool,
    /// Whether this phase verifies signatures.
    pub signature_verification_runtime_added: bool,
    /// Whether this phase signs request proofs.
    pub signing_runtime_added: bool,
    /// Whether this phase adds vault encryption/decryption runtime.
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

/// Return Phase 3A posture.
pub fn native_passport_authorization_posture() -> NativePassportAuthorizationPosture {
    NativePassportAuthorizationPosture {
        phase_label: NATIVE_PASSPORT_PHASE3A_LABEL,
        root_device_authorization_dtos_added: true,
        signature_verification_runtime_added: false,
        signing_runtime_added: false,
        vault_runtime_added: false,
        capability_issuance_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE3A_FORBIDDEN_AUTHORITY_FLAGS,
    }
}

/// Validate a Phase 3A device authorization grant DTO without signature/proof review.
pub fn validate_device_authorization_grant(
    grant: &DeviceAuthorizationGrantV1,
) -> Result<(), DeviceAuthorizationReviewError> {
    if grant.allowed_scopes.is_empty() {
        return Err(DeviceAuthorizationReviewError::MissingScopes);
    }

    if grant.root.root_public_key.as_str() == grant.device_public_key.as_str() {
        return Err(DeviceAuthorizationReviewError::RootDeviceKeyCollision);
    }

    for (index, scope) in grant.allowed_scopes.iter().enumerate() {
        if !PHASE3A_ALLOWED_DEVICE_SCOPES.contains(scope) {
            return Err(DeviceAuthorizationReviewError::UnsupportedScope);
        }

        if grant.allowed_scopes[..index].contains(scope) {
            return Err(DeviceAuthorizationReviewError::DuplicateScope);
        }
    }

    if grant.can_unlock_root
        || grant.can_authorize_devices
        || grant.can_issue_capabilities
        || grant.can_mutate_wallet_or_ledger
    {
        return Err(DeviceAuthorizationReviewError::UnsafeAuthorityFlag);
    }

    Ok(())
}

/// Review a Phase 3A device authorization draft against a root descriptor.
///
/// This is a pure DTO review. It does not verify signatures, sign request proofs,
/// unlock root material, decrypt vaults, issue capabilities, mutate wallets, or mutate ledgers.
pub fn review_device_authorization_draft(
    root: &RootPassportDescriptorV1,
    draft: DeviceAuthorizationDraftV1,
) -> Result<DeviceAuthorizationGrantV1, DeviceAuthorizationReviewError> {
    if draft.passport_id.as_str() != root.passport_id.as_str() {
        return Err(DeviceAuthorizationReviewError::PassportMismatch);
    }

    if draft.root_public_key.as_str() != root.root_public_key.as_str() {
        return Err(DeviceAuthorizationReviewError::RootPublicKeyMismatch);
    }

    let grant = DeviceAuthorizationGrantV1 {
        root: root.clone(),
        device_id: draft.device_id,
        device_public_key: draft.device_public_key,
        device_class: draft.device_class,
        allowed_scopes: draft.requested_scopes,
        can_sign_request_proofs: true,
        can_unlock_root: false,
        can_authorize_devices: false,
        can_issue_capabilities: false,
        can_mutate_wallet_or_ledger: false,
    };

    validate_device_authorization_grant(&grant)?;

    Ok(grant)
}
