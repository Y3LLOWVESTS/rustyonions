//! RO:WHAT — Native Passport secure-surface compartment contract DTO foundation.
//! RO:WHY — P3 Identity & Keys. Binds recovery-root and device-key secure compartments to explicit contract labels before vault, sealer, storage, or encryption runtime exists.
//! RO:INTERACTS — Phase 5A platform sealer contract DTOs, Phase 4A recovery-root DTOs, Phase 4B device-key DTOs.
//! RO:INVARIANTS — this module describes compartment binding metadata only; it does not call platform keychains/keystores, seal/unseal bytes, encrypt/decrypt, store secrets, export material, unlock vaults, sign, verify, route, issue capabilities, or mutate wallet/ledger state.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects domain mismatches, platform contract mismatches, platform family mismatches, empty/duplicate/missing compartment bindings, label mismatches, and flags implying implementation, storage, material export, vault unlock, crypto runtime, I/O runtime, or wallet/ledger mutation.
//! RO:TEST — tests/native_passport_phase5b_secure_surface_compartment_contract.rs.

use super::{
    NativePlatformFamily, NativePlatformSealerContractDescriptorV1, NativeSecureCompartment,
    PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
};

/// Phase label for native secure-surface compartment contract DTO foundations.
pub const NATIVE_PASSPORT_PHASE5B_LABEL: &str =
    "NATIVE_PASSPORT_PHASE5B_SECURE_SURFACE_COMPARTMENT_CONTRACT";

/// Canonical secure-surface compartment contract DTO domain.
pub const PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN: &str =
    "native-passport/secure-surface-compartment/v1";

/// Recovery-root compartment label used by native vault/sealer contracts later.
pub const PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL: &str = "passport_root_compartment";

/// Device-key compartment label used by native vault/sealer contracts later.
pub const PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL: &str = "device_compartment";

/// Required secure-surface bindings.
pub const PHASE5B_REQUIRED_SECURE_SURFACE_BINDINGS: &[NativeSecureSurfaceCompartmentBindingV1] = &[
    NativeSecureSurfaceCompartmentBindingV1 {
        compartment: NativeSecureCompartment::RecoveryRoot,
        compartment_label: PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL,
        binding_domain: PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
        platform_contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
    },
    NativeSecureSurfaceCompartmentBindingV1 {
        compartment: NativeSecureCompartment::DeviceKey,
        compartment_label: PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL,
        binding_domain: PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
        platform_contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
    },
];

/// Authority meanings that Phase 5B DTOs must not grant.
pub const PHASE5B_FORBIDDEN_SECURE_SURFACE_AUTHORITY_FLAGS: &[&str] = &[
    "platform_sealer_implementation",
    "secret_storage",
    "material_export",
    "vault_unlock",
    "encryption_runtime",
    "decryption_runtime",
    "runtime_io",
    "signing_runtime",
    "signature_verification_runtime",
    "capability_issuance",
    "wallet_spend",
    "ledger_mutation",
];

/// Public secure-surface compartment binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeSecureSurfaceCompartmentBindingV1 {
    /// Secure compartment being bound.
    pub compartment: NativeSecureCompartment,
    /// Stable compartment label.
    pub compartment_label: &'static str,
    /// Secure-surface binding domain.
    pub binding_domain: &'static str,
    /// Platform sealer contract domain this binding expects.
    pub platform_contract_domain: &'static str,
}

/// Public secure-surface contract descriptor without implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSecureSurfaceContractDescriptorV1 {
    /// Secure-surface contract domain.
    pub contract_domain: &'static str,
    /// Platform family inherited from the platform sealer contract.
    pub platform_family: NativePlatformFamily,
    /// Platform sealer contract domain this secure surface binds to.
    pub platform_contract_domain: &'static str,
    /// Declared compartment bindings.
    pub compartment_bindings: Vec<NativeSecureSurfaceCompartmentBindingV1>,
    /// Whether this descriptor is contract-only.
    pub contract_only: bool,
}

/// Public secure-surface contract draft before any implementation exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSecureSurfaceContractDraftV1 {
    /// Secure-surface contract domain.
    pub contract_domain: &'static str,
    /// Platform family expected from the platform sealer contract.
    pub platform_family: NativePlatformFamily,
    /// Platform sealer contract domain this secure surface binds to.
    pub platform_contract_domain: &'static str,
    /// Requested compartment bindings.
    pub requested_bindings: Vec<NativeSecureSurfaceCompartmentBindingV1>,
    /// Boundary flag: this DTO must not include a sealer implementation.
    pub includes_platform_sealer_implementation: bool,
    /// Boundary flag: this DTO must not store secret material.
    pub stores_secret_material: bool,
    /// Boundary flag: this DTO must not export material.
    pub exports_material: bool,
    /// Boundary flag: this DTO must not unlock a vault.
    pub requests_vault_unlock: bool,
    /// Boundary flag: this DTO must not encrypt or decrypt data.
    pub requests_encryption_or_decryption: bool,
    /// Boundary flag: this DTO must not perform runtime I/O.
    pub requests_runtime_io: bool,
    /// Boundary flag: this DTO must not mutate wallet or ledger state.
    pub requests_wallet_or_ledger_mutation: bool,
}

/// Phase 5B secure-surface contract review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeSecureSurfaceContractReviewError {
    /// Secure-surface contract domain must match.
    ContractDomainMismatch,
    /// Platform sealer contract domain must match.
    PlatformContractDomainMismatch,
    /// Platform family must match the reviewed platform sealer contract.
    PlatformFamilyMismatch,
    /// At least one compartment binding is required.
    MissingBindings,
    /// Duplicate compartment bindings are rejected for deterministic review.
    DuplicateCompartmentBinding,
    /// Recovery-root and device-key bindings are both required.
    MissingRequiredCompartmentBinding,
    /// Binding label must match the expected compartment label.
    CompartmentLabelMismatch,
    /// DTO flags attempted to carry or exercise unsafe secure-surface authority.
    UnsafeSecureSurfaceAuthorityFlag,
}

/// Phase 5B posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportSecureSurfacePosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds secure-surface compartment contract DTOs.
    pub secure_surface_contract_dtos_added: bool,
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
    /// Whether this phase adds runtime I/O.
    pub runtime_io_added: bool,
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

/// Return Phase 5B secure-surface contract posture.
pub fn native_passport_secure_surface_posture() -> NativePassportSecureSurfacePosture {
    NativePassportSecureSurfacePosture {
        phase_label: NATIVE_PASSPORT_PHASE5B_LABEL,
        secure_surface_contract_dtos_added: true,
        platform_sealer_implementation_added: false,
        secret_storage_added: false,
        material_export_added: false,
        encryption_runtime_added: false,
        decryption_runtime_added: false,
        runtime_io_added: false,
        vault_runtime_added: false,
        signing_runtime_added: false,
        signature_verification_runtime_added: false,
        capability_issuance_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE5B_FORBIDDEN_SECURE_SURFACE_AUTHORITY_FLAGS,
    }
}

/// Validate a secure-surface contract descriptor without using platform secure storage.
pub fn validate_native_secure_surface_contract_descriptor(
    platform_contract: &NativePlatformSealerContractDescriptorV1,
    descriptor: &NativeSecureSurfaceContractDescriptorV1,
) -> Result<(), NativeSecureSurfaceContractReviewError> {
    if descriptor.contract_domain != PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN {
        return Err(NativeSecureSurfaceContractReviewError::ContractDomainMismatch);
    }

    if descriptor.platform_contract_domain != platform_contract.contract_domain
        || descriptor.platform_contract_domain != PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN
    {
        return Err(NativeSecureSurfaceContractReviewError::PlatformContractDomainMismatch);
    }

    if descriptor.platform_family != platform_contract.platform_family {
        return Err(NativeSecureSurfaceContractReviewError::PlatformFamilyMismatch);
    }

    validate_secure_surface_bindings(&descriptor.compartment_bindings)
}

/// Review a secure-surface contract draft into a descriptor.
///
/// This is contract DTO review only. It does not call OS secure storage,
/// store secrets, export material, unlock vaults, encrypt, decrypt, perform I/O,
/// sign, verify, issue capabilities, mutate wallets, or mutate ledgers.
pub fn review_native_secure_surface_contract_draft(
    platform_contract: &NativePlatformSealerContractDescriptorV1,
    draft: NativeSecureSurfaceContractDraftV1,
) -> Result<NativeSecureSurfaceContractDescriptorV1, NativeSecureSurfaceContractReviewError> {
    if draft.contract_domain != PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN {
        return Err(NativeSecureSurfaceContractReviewError::ContractDomainMismatch);
    }

    if draft.platform_contract_domain != platform_contract.contract_domain
        || draft.platform_contract_domain != PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN
    {
        return Err(NativeSecureSurfaceContractReviewError::PlatformContractDomainMismatch);
    }

    if draft.platform_family != platform_contract.platform_family {
        return Err(NativeSecureSurfaceContractReviewError::PlatformFamilyMismatch);
    }

    validate_secure_surface_bindings(&draft.requested_bindings)?;

    if draft.includes_platform_sealer_implementation
        || draft.stores_secret_material
        || draft.exports_material
        || draft.requests_vault_unlock
        || draft.requests_encryption_or_decryption
        || draft.requests_runtime_io
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeSecureSurfaceContractReviewError::UnsafeSecureSurfaceAuthorityFlag);
    }

    Ok(NativeSecureSurfaceContractDescriptorV1 {
        contract_domain: PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
        platform_family: draft.platform_family,
        platform_contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
        compartment_bindings: draft.requested_bindings,
        contract_only: true,
    })
}

fn validate_secure_surface_bindings(
    bindings: &[NativeSecureSurfaceCompartmentBindingV1],
) -> Result<(), NativeSecureSurfaceContractReviewError> {
    if bindings.is_empty() {
        return Err(NativeSecureSurfaceContractReviewError::MissingBindings);
    }

    let mut has_recovery_root = false;
    let mut has_device_key = false;

    for (index, binding) in bindings.iter().enumerate() {
        if binding.binding_domain != PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN
            || binding.platform_contract_domain != PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN
        {
            return Err(NativeSecureSurfaceContractReviewError::ContractDomainMismatch);
        }

        if bindings[..index]
            .iter()
            .any(|previous| previous.compartment == binding.compartment)
        {
            return Err(NativeSecureSurfaceContractReviewError::DuplicateCompartmentBinding);
        }

        if binding.compartment_label != expected_compartment_label(binding.compartment) {
            return Err(NativeSecureSurfaceContractReviewError::CompartmentLabelMismatch);
        }

        match binding.compartment {
            NativeSecureCompartment::RecoveryRoot => has_recovery_root = true,
            NativeSecureCompartment::DeviceKey => has_device_key = true,
        }
    }

    if !has_recovery_root || !has_device_key {
        return Err(NativeSecureSurfaceContractReviewError::MissingRequiredCompartmentBinding);
    }

    Ok(())
}

fn expected_compartment_label(compartment: NativeSecureCompartment) -> &'static str {
    match compartment {
        NativeSecureCompartment::RecoveryRoot => PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL,
        NativeSecureCompartment::DeviceKey => PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL,
    }
}
