//! RO:WHAT — Native Passport recovery-root DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines the public, local-custody recovery-root shape before generation, sealing, storage, or proof review exists.
//! RO:INTERACTS — Native Passport canonical IDs, optional ron-naming handle DTOs, Phase 3 authorization surfaces.
//! RO:INVARIANTS — recovery-root DTOs carry only public identifiers, public keys, public handle metadata, domain labels, and boundary flags; no seed material, root secret, vault unlock, platform sealer, signing, route, wallet, ledger, or capability authority is added.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects empty/duplicate recovery scopes, domain mismatches, and DTO flags that would imply secret custody, material export, vault unlock, or runtime authority.
//! RO:TEST — tests/native_passport_phase4a_native_recovery_root_dto.rs.

use super::{Ed25519PublicKeyHex, HandleV1, PassportIdV1};

/// Phase label for native recovery-root DTO foundations.
pub const NATIVE_PASSPORT_PHASE4A_LABEL: &str = "NATIVE_PASSPORT_PHASE4A_NATIVE_RECOVERY_ROOT_DTO";

/// Canonical local recovery-root DTO domain.
pub const PHASE4A_RECOVERY_ROOT_DOMAIN: &str = "native-passport/recovery-root/v1";

/// Recovery-root scopes allowed in Phase 4A DTOs.
pub const PHASE4A_ALLOWED_RECOVERY_ROOT_SCOPES: &[NativeRecoveryRootScope] = &[
    NativeRecoveryRootScope::RecoverPassport,
    NativeRecoveryRootScope::EnrollDevice,
    NativeRecoveryRootScope::RevokeDevice,
    NativeRecoveryRootScope::RotateRoot,
];

/// Authority meanings that Phase 4A DTOs must not grant.
pub const PHASE4A_FORBIDDEN_RECOVERY_ROOT_AUTHORITY_FLAGS: &[&str] = &[
    "generate_recovery_root",
    "store_recovery_secret",
    "export_recovery_material",
    "unlock_vault",
    "platform_sealer",
    "signing_runtime",
    "signature_verification_runtime",
    "capability_issuance",
    "wallet_spend",
    "ledger_mutation",
];

/// Public recovery-root scope labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeRecoveryRootScope {
    /// Recover the Passport identity locally.
    RecoverPassport,
    /// Enroll delegated device keys later.
    EnrollDevice,
    /// Revoke delegated device keys later.
    RevokeDevice,
    /// Rotate root public identity later.
    RotateRoot,
}

/// Public recovery-root descriptor without secret material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRecoveryRootDescriptorV1 {
    /// Canonical Passport ID.
    pub passport_id: PassportIdV1,
    /// Public recovery-root key.
    pub recovery_root_public_key: Ed25519PublicKeyHex,
    /// Optional public handle.
    pub optional_handle: Option<HandleV1>,
    /// Recovery-root DTO domain.
    pub recovery_domain: &'static str,
    /// Allowed recovery scopes.
    pub allowed_scopes: Vec<NativeRecoveryRootScope>,
}

/// Public recovery-root draft before any generation, sealing, storage, or proof review exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRecoveryRootDraftV1 {
    /// Canonical Passport ID.
    pub passport_id: PassportIdV1,
    /// Public recovery-root key.
    pub recovery_root_public_key: Ed25519PublicKeyHex,
    /// Optional public handle.
    pub optional_handle: Option<HandleV1>,
    /// Recovery-root DTO domain.
    pub recovery_domain: &'static str,
    /// Requested recovery scopes.
    pub requested_scopes: Vec<NativeRecoveryRootScope>,
    /// Boundary flag: this DTO must not contain local recovery material.
    pub contains_recovery_material: bool,
    /// Boundary flag: this DTO must not export recovery material.
    pub exports_recovery_material: bool,
    /// Boundary flag: this DTO must not unlock a vault.
    pub requests_vault_unlock: bool,
    /// Boundary flag: this DTO must not mutate wallet or ledger state.
    pub requests_wallet_or_ledger_mutation: bool,
}

/// Phase 4A recovery-root review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeRecoveryRootReviewError {
    /// The recovery domain must match the native recovery-root domain.
    RecoveryDomainMismatch,
    /// At least one recovery scope is required.
    MissingScopes,
    /// Duplicate recovery scopes are rejected for deterministic review.
    DuplicateScope,
    /// DTO flags attempted to carry or exercise unsafe recovery authority.
    UnsafeRecoveryAuthorityFlag,
}

/// Phase 4A posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportRecoveryRootPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds recovery-root DTOs.
    pub recovery_root_dtos_added: bool,
    /// Whether this phase generates recovery roots.
    pub recovery_root_generation_added: bool,
    /// Whether this phase stores recovery secrets.
    pub recovery_secret_storage_added: bool,
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

/// Return Phase 4A recovery-root posture.
pub fn native_passport_recovery_root_posture() -> NativePassportRecoveryRootPosture {
    NativePassportRecoveryRootPosture {
        phase_label: NATIVE_PASSPORT_PHASE4A_LABEL,
        recovery_root_dtos_added: true,
        recovery_root_generation_added: false,
        recovery_secret_storage_added: false,
        platform_sealer_added: false,
        signing_runtime_added: false,
        signature_verification_runtime_added: false,
        vault_runtime_added: false,
        capability_issuance_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE4A_FORBIDDEN_RECOVERY_ROOT_AUTHORITY_FLAGS,
    }
}

/// Validate a recovery-root descriptor without generation, storage, sealing, or proof review.
pub fn validate_native_recovery_root_descriptor(
    descriptor: &NativeRecoveryRootDescriptorV1,
) -> Result<(), NativeRecoveryRootReviewError> {
    if descriptor.recovery_domain != PHASE4A_RECOVERY_ROOT_DOMAIN {
        return Err(NativeRecoveryRootReviewError::RecoveryDomainMismatch);
    }

    validate_recovery_scopes(&descriptor.allowed_scopes)
}

/// Review a recovery-root draft into a public descriptor.
///
/// This is DTO review only. It does not generate recovery roots, store secret material,
/// unlock vaults, call platform sealers, sign messages, verify signatures, issue capabilities,
/// mutate wallets, or mutate ledgers.
pub fn review_native_recovery_root_draft(
    draft: NativeRecoveryRootDraftV1,
) -> Result<NativeRecoveryRootDescriptorV1, NativeRecoveryRootReviewError> {
    if draft.recovery_domain != PHASE4A_RECOVERY_ROOT_DOMAIN {
        return Err(NativeRecoveryRootReviewError::RecoveryDomainMismatch);
    }

    validate_recovery_scopes(&draft.requested_scopes)?;

    if draft.contains_recovery_material
        || draft.exports_recovery_material
        || draft.requests_vault_unlock
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeRecoveryRootReviewError::UnsafeRecoveryAuthorityFlag);
    }

    Ok(NativeRecoveryRootDescriptorV1 {
        passport_id: draft.passport_id,
        recovery_root_public_key: draft.recovery_root_public_key,
        optional_handle: draft.optional_handle,
        recovery_domain: PHASE4A_RECOVERY_ROOT_DOMAIN,
        allowed_scopes: draft.requested_scopes,
    })
}

fn validate_recovery_scopes(
    scopes: &[NativeRecoveryRootScope],
) -> Result<(), NativeRecoveryRootReviewError> {
    if scopes.is_empty() {
        return Err(NativeRecoveryRootReviewError::MissingScopes);
    }

    for (index, scope) in scopes.iter().enumerate() {
        if scopes[..index].contains(scope) {
            return Err(NativeRecoveryRootReviewError::DuplicateScope);
        }
    }

    Ok(())
}
