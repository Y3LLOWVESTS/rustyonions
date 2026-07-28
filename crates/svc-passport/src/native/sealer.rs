//! RO:WHAT — Native Passport platform sealer contract DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines the secure-surface contract names and platform compartment intent before any OS sealer implementation exists.
//! RO:INTERACTS — Native recovery-root DTOs, native device-key DTOs, future two-compartment vault contract.
//! RO:INVARIANTS — this module describes public contract metadata only; it does not call platform keychains/keystores, seal/unseal bytes, encrypt/decrypt, store secrets, unlock vaults, sign, verify, route, issue capabilities, or mutate wallet/ledger state.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects domain mismatches, empty/duplicate compartment declarations, and flags implying sealer implementation, secret storage, material export, vault unlock, or wallet/ledger mutation.
//! RO:TEST — tests/native_passport_phase5a_platform_sealer_contract_dto.rs.

/// Phase label for native platform sealer contract DTO foundations.
pub const NATIVE_PASSPORT_PHASE5A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE5A_PLATFORM_SEALER_CONTRACT_DTO";

/// Canonical platform sealer contract DTO domain.
pub const PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN: &str =
    "native-passport/platform-sealer-contract/v1";

/// Secure compartments declared by the Phase 5A platform sealer contract.
pub const PHASE5A_REQUIRED_SECURE_COMPARTMENTS: &[NativeSecureCompartment] = &[
    NativeSecureCompartment::RecoveryRoot,
    NativeSecureCompartment::DeviceKey,
];

/// Authority meanings that Phase 5A DTOs must not grant.
pub const PHASE5A_FORBIDDEN_PLATFORM_SEALER_AUTHORITY_FLAGS: &[&str] = &[
    "platform_sealer_implementation",
    "secret_storage",
    "material_export",
    "vault_unlock",
    "encryption_runtime",
    "decryption_runtime",
    "signing_runtime",
    "signature_verification_runtime",
    "capability_issuance",
    "wallet_spend",
    "ledger_mutation",
];

/// Platform families named by the contract without implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativePlatformFamily {
    /// Apple macOS secure storage family.
    MacosKeychain,
    /// Apple iOS secure storage family.
    IosKeychain,
    /// Android secure storage family.
    AndroidKeystore,
    /// Windows secure storage family.
    WindowsDpapi,
    /// Linux desktop secure storage family.
    LinuxSecretService,
    /// Local development or unsupported platform placeholder.
    UnknownLocal,
}

/// Secure compartments declared before the two-compartment vault runtime exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeSecureCompartment {
    /// Recovery-root compartment intent.
    RecoveryRoot,
    /// Device-key compartment intent.
    DeviceKey,
}

/// Public platform sealer contract descriptor without implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePlatformSealerContractDescriptorV1 {
    /// Sealer contract domain.
    pub contract_domain: &'static str,
    /// Platform family.
    pub platform_family: NativePlatformFamily,
    /// Declared secure compartments.
    pub secure_compartments: Vec<NativeSecureCompartment>,
    /// Whether the descriptor is contract-only.
    pub contract_only: bool,
}

/// Public platform sealer contract draft before platform integration exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePlatformSealerContractDraftV1 {
    /// Sealer contract domain.
    pub contract_domain: &'static str,
    /// Platform family.
    pub platform_family: NativePlatformFamily,
    /// Requested secure compartments.
    pub requested_compartments: Vec<NativeSecureCompartment>,
    /// Boundary flag: this DTO must not include a sealer implementation.
    pub includes_platform_sealer_implementation: bool,
    /// Boundary flag: this DTO must not store secret material.
    pub stores_secret_material: bool,
    /// Boundary flag: this DTO must not export sealed or unsealed material.
    pub exports_material: bool,
    /// Boundary flag: this DTO must not unlock a vault.
    pub requests_vault_unlock: bool,
    /// Boundary flag: this DTO must not encrypt or decrypt data.
    pub requests_encryption_or_decryption: bool,
    /// Boundary flag: this DTO must not mutate wallet or ledger state.
    pub requests_wallet_or_ledger_mutation: bool,
}

/// Phase 5A sealer contract review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePlatformSealerContractReviewError {
    /// Contract domain must match the native platform sealer contract domain.
    ContractDomainMismatch,
    /// At least one secure compartment is required.
    MissingCompartments,
    /// Duplicate secure compartments are rejected for deterministic review.
    DuplicateCompartment,
    /// DTO flags attempted to carry or exercise unsafe platform sealer authority.
    UnsafePlatformSealerAuthorityFlag,
}

/// Phase 5A posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportPlatformSealerPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds platform sealer contract DTOs.
    pub platform_sealer_contract_dtos_added: bool,
    /// Whether this phase adds a platform sealer implementation.
    pub platform_sealer_implementation_added: bool,
    /// Whether this phase stores secrets.
    pub secret_storage_added: bool,
    /// Whether this phase exports secret or sealed material.
    pub material_export_added: bool,
    /// Whether this phase adds encryption runtime.
    pub encryption_runtime_added: bool,
    /// Whether this phase adds decryption runtime.
    pub decryption_runtime_added: bool,
    /// Whether this phase adds vault runtime.
    pub vault_runtime_added: bool,
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

/// Return Phase 5A platform sealer contract posture.
pub fn native_passport_platform_sealer_posture() -> NativePassportPlatformSealerPosture {
    NativePassportPlatformSealerPosture {
        phase_label: NATIVE_PASSPORT_PHASE5A_LABEL,
        platform_sealer_contract_dtos_added: true,
        platform_sealer_implementation_added: false,
        secret_storage_added: false,
        material_export_added: false,
        encryption_runtime_added: false,
        decryption_runtime_added: false,
        vault_runtime_added: false,
        signing_runtime_added: false,
        signature_verification_runtime_added: false,
        capability_issuance_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE5A_FORBIDDEN_PLATFORM_SEALER_AUTHORITY_FLAGS,
    }
}

/// Validate a platform sealer contract descriptor without using platform secure storage.
pub fn validate_native_platform_sealer_contract_descriptor(
    descriptor: &NativePlatformSealerContractDescriptorV1,
) -> Result<(), NativePlatformSealerContractReviewError> {
    if descriptor.contract_domain != PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN {
        return Err(NativePlatformSealerContractReviewError::ContractDomainMismatch);
    }

    validate_secure_compartments(&descriptor.secure_compartments)
}

/// Review a platform sealer contract draft into a descriptor.
///
/// This is contract DTO review only. It does not call OS secure storage,
/// store secrets, export material, unlock vaults, encrypt, decrypt, sign,
/// verify, issue capabilities, mutate wallets, or mutate ledgers.
pub fn review_native_platform_sealer_contract_draft(
    draft: NativePlatformSealerContractDraftV1,
) -> Result<NativePlatformSealerContractDescriptorV1, NativePlatformSealerContractReviewError> {
    if draft.contract_domain != PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN {
        return Err(NativePlatformSealerContractReviewError::ContractDomainMismatch);
    }

    validate_secure_compartments(&draft.requested_compartments)?;

    if draft.includes_platform_sealer_implementation
        || draft.stores_secret_material
        || draft.exports_material
        || draft.requests_vault_unlock
        || draft.requests_encryption_or_decryption
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativePlatformSealerContractReviewError::UnsafePlatformSealerAuthorityFlag);
    }

    Ok(NativePlatformSealerContractDescriptorV1 {
        contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
        platform_family: draft.platform_family,
        secure_compartments: draft.requested_compartments,
        contract_only: true,
    })
}

fn validate_secure_compartments(
    compartments: &[NativeSecureCompartment],
) -> Result<(), NativePlatformSealerContractReviewError> {
    if compartments.is_empty() {
        return Err(NativePlatformSealerContractReviewError::MissingCompartments);
    }

    for (index, compartment) in compartments.iter().enumerate() {
        if compartments[..index].contains(compartment) {
            return Err(NativePlatformSealerContractReviewError::DuplicateCompartment);
        }
    }

    Ok(())
}
