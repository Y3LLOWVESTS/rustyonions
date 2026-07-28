//! RO:WHAT — Native Passport two-compartment vault header DTO foundation.
//! RO:WHY — P3 Identity & Keys. Defines authenticated vault header metadata before PIN unlock, encryption, decryption, platform sealing, storage, or vault runtime exists.
//! RO:INTERACTS — Phase 5B secure-surface contract, Phase 5A platform sealer contract, native recovery-root and device-key compartment labels.
//! RO:INVARIANTS — vault header DTOs carry only public header metadata, domains, labels, algorithm names, and length constants; no PINs, keys, ciphertext, plaintext, sealed bytes, storage handles, encryption/decryption runtime, routes, signing, capabilities, wallet, or ledger authority are added.
//! RO:METRICS — none.
//! RO:CONFIG — compiled only through the `native-passport` feature-gated native module.
//! RO:SECURITY — rejects domain/version drift, platform mismatch, secure-surface mismatch, missing/duplicate compartments, label drift, algorithm drift, length drift, and flags implying PIN unlock, crypto runtime, sealer runtime, storage, export, I/O, or wallet/ledger mutation.
//! RO:TEST — tests/native_passport_phase6a_two_compartment_vault_header_dto.rs.

use super::{
    NativePlatformFamily, NativeSecureCompartment, NativeSecureSurfaceContractDescriptorV1,
    PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN, PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL,
    PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL, PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
};

/// Phase label for native two-compartment vault header DTO foundations.
pub const NATIVE_PASSPORT_PHASE6A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DTO";

/// Canonical two-compartment vault header DTO domain.
pub const PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN: &str =
    "native-passport/two-compartment-vault-header/v1";

/// Vault header version locked for the first native Passport vault header.
pub const PHASE6A_VAULT_HEADER_VERSION: u16 = 1;

/// KDF label locked by the header DTO. No KDF runtime is added in Phase 6A.
pub const PHASE6A_KDF_ALGORITHM_LABEL: &str = "argon2id";

/// AEAD label locked by the header DTO. No encryption runtime is added in Phase 6A.
pub const PHASE6A_AEAD_ALGORITHM_LABEL: &str = "xchacha20poly1305";

/// Authenticated-header posture label.
pub const PHASE6A_AUTHENTICATED_HEADER_LABEL: &str = "vault-header-v1-authenticated";

/// Argon2id salt length locked as public header metadata.
pub const PHASE6A_KDF_SALT_LEN: usize = 16;

/// XChaCha20-Poly1305 nonce length locked as public header metadata.
pub const PHASE6A_AEAD_NONCE_LEN: usize = 24;

/// AEAD tag length locked as public header metadata.
pub const PHASE6A_AEAD_TAG_LEN: usize = 16;

/// Required vault compartment headers.
pub const PHASE6A_REQUIRED_VAULT_COMPARTMENT_HEADERS: &[NativeVaultCompartmentHeaderV1] = &[
    NativeVaultCompartmentHeaderV1 {
        compartment: NativeSecureCompartment::RecoveryRoot,
        compartment_label: PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL,
        secure_surface_contract_domain: PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
        platform_contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
        kdf_algorithm_label: PHASE6A_KDF_ALGORITHM_LABEL,
        aead_algorithm_label: PHASE6A_AEAD_ALGORITHM_LABEL,
        kdf_salt_len: PHASE6A_KDF_SALT_LEN,
        aead_nonce_len: PHASE6A_AEAD_NONCE_LEN,
        aead_tag_len: PHASE6A_AEAD_TAG_LEN,
        authenticated_header_label: PHASE6A_AUTHENTICATED_HEADER_LABEL,
    },
    NativeVaultCompartmentHeaderV1 {
        compartment: NativeSecureCompartment::DeviceKey,
        compartment_label: PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL,
        secure_surface_contract_domain: PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
        platform_contract_domain: PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN,
        kdf_algorithm_label: PHASE6A_KDF_ALGORITHM_LABEL,
        aead_algorithm_label: PHASE6A_AEAD_ALGORITHM_LABEL,
        kdf_salt_len: PHASE6A_KDF_SALT_LEN,
        aead_nonce_len: PHASE6A_AEAD_NONCE_LEN,
        aead_tag_len: PHASE6A_AEAD_TAG_LEN,
        authenticated_header_label: PHASE6A_AUTHENTICATED_HEADER_LABEL,
    },
];

/// Authority meanings that Phase 6A DTOs must not grant.
pub const PHASE6A_FORBIDDEN_VAULT_HEADER_AUTHORITY_FLAGS: &[&str] = &[
    "pin_unlock",
    "pin_derivation_runtime",
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

/// Public per-compartment vault header metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeVaultCompartmentHeaderV1 {
    /// Secure compartment named by the header.
    pub compartment: NativeSecureCompartment,
    /// Stable compartment label.
    pub compartment_label: &'static str,
    /// Secure-surface contract domain.
    pub secure_surface_contract_domain: &'static str,
    /// Platform sealer contract domain.
    pub platform_contract_domain: &'static str,
    /// KDF algorithm label.
    pub kdf_algorithm_label: &'static str,
    /// AEAD algorithm label.
    pub aead_algorithm_label: &'static str,
    /// KDF salt length.
    pub kdf_salt_len: usize,
    /// AEAD nonce length.
    pub aead_nonce_len: usize,
    /// AEAD tag length.
    pub aead_tag_len: usize,
    /// Authenticated-header label.
    pub authenticated_header_label: &'static str,
}

/// Public vault header descriptor without vault runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTwoCompartmentVaultHeaderDescriptorV1 {
    /// Vault header domain.
    pub header_domain: &'static str,
    /// Header version.
    pub header_version: u16,
    /// Platform family inherited from secure-surface contract.
    pub platform_family: NativePlatformFamily,
    /// Secure-surface contract domain this header binds to.
    pub secure_surface_contract_domain: &'static str,
    /// Declared compartment headers.
    pub compartment_headers: Vec<NativeVaultCompartmentHeaderV1>,
    /// Whether this descriptor is header-only.
    pub header_only: bool,
}

/// Public vault header draft before PIN unlock, crypto, storage, or vault runtime exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTwoCompartmentVaultHeaderDraftV1 {
    /// Vault header domain.
    pub header_domain: &'static str,
    /// Header version.
    pub header_version: u16,
    /// Platform family expected from secure-surface contract.
    pub platform_family: NativePlatformFamily,
    /// Secure-surface contract domain this header binds to.
    pub secure_surface_contract_domain: &'static str,
    /// Requested compartment headers.
    pub requested_compartment_headers: Vec<NativeVaultCompartmentHeaderV1>,
    /// Boundary flag: this DTO must not unlock a PIN.
    pub requests_pin_unlock: bool,
    /// Boundary flag: this DTO must not run PIN derivation.
    pub requests_pin_derivation_runtime: bool,
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

/// Phase 6A vault header review errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeVaultHeaderReviewError {
    /// Vault header domain must match.
    HeaderDomainMismatch,
    /// Vault header version must match.
    HeaderVersionMismatch,
    /// Platform family must match the secure-surface contract.
    PlatformFamilyMismatch,
    /// Secure-surface contract domain must match.
    SecureSurfaceContractDomainMismatch,
    /// At least one compartment header is required.
    MissingCompartmentHeaders,
    /// Duplicate compartment headers are rejected for deterministic review.
    DuplicateCompartmentHeader,
    /// Recovery-root and device-key headers are both required.
    MissingRequiredCompartmentHeader,
    /// Compartment label must match the expected label.
    CompartmentLabelMismatch,
    /// KDF or AEAD algorithm label must match the locked header labels.
    AlgorithmLabelMismatch,
    /// Salt, nonce, or tag length must match the locked header lengths.
    HeaderLengthMismatch,
    /// Authenticated-header label must match.
    AuthenticatedHeaderLabelMismatch,
    /// DTO flags attempted to carry or exercise unsafe vault authority.
    UnsafeVaultHeaderAuthorityFlag,
}

/// Phase 6A posture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportVaultHeaderPosture {
    /// Active phase label.
    pub phase_label: &'static str,
    /// Whether this phase adds vault header DTOs.
    pub vault_header_dtos_added: bool,
    /// Whether this phase adds PIN unlock.
    pub pin_unlock_added: bool,
    /// Whether this phase adds PIN derivation runtime.
    pub pin_derivation_runtime_added: bool,
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

/// Return Phase 6A two-compartment vault header posture.
pub fn native_passport_vault_header_posture() -> NativePassportVaultHeaderPosture {
    NativePassportVaultHeaderPosture {
        phase_label: NATIVE_PASSPORT_PHASE6A_LABEL,
        vault_header_dtos_added: true,
        pin_unlock_added: false,
        pin_derivation_runtime_added: false,
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
        forbidden_authority_flags: PHASE6A_FORBIDDEN_VAULT_HEADER_AUTHORITY_FLAGS,
    }
}

/// Validate a two-compartment vault header descriptor without vault runtime.
pub fn validate_native_two_compartment_vault_header_descriptor(
    secure_surface: &NativeSecureSurfaceContractDescriptorV1,
    descriptor: &NativeTwoCompartmentVaultHeaderDescriptorV1,
) -> Result<(), NativeVaultHeaderReviewError> {
    if descriptor.header_domain != PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN {
        return Err(NativeVaultHeaderReviewError::HeaderDomainMismatch);
    }

    if descriptor.header_version != PHASE6A_VAULT_HEADER_VERSION {
        return Err(NativeVaultHeaderReviewError::HeaderVersionMismatch);
    }

    if descriptor.platform_family != secure_surface.platform_family {
        return Err(NativeVaultHeaderReviewError::PlatformFamilyMismatch);
    }

    if descriptor.secure_surface_contract_domain != secure_surface.contract_domain
        || descriptor.secure_surface_contract_domain != PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN
    {
        return Err(NativeVaultHeaderReviewError::SecureSurfaceContractDomainMismatch);
    }

    validate_vault_compartment_headers(&descriptor.compartment_headers)
}

/// Review a vault header draft into a header descriptor.
///
/// This is header DTO review only. It does not derive PIN keys, unlock vaults,
/// call OS secure storage, store secrets, export material, encrypt, decrypt,
/// perform I/O, sign, verify, issue capabilities, mutate wallets, or mutate ledgers.
pub fn review_native_two_compartment_vault_header_draft(
    secure_surface: &NativeSecureSurfaceContractDescriptorV1,
    draft: NativeTwoCompartmentVaultHeaderDraftV1,
) -> Result<NativeTwoCompartmentVaultHeaderDescriptorV1, NativeVaultHeaderReviewError> {
    if draft.header_domain != PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN {
        return Err(NativeVaultHeaderReviewError::HeaderDomainMismatch);
    }

    if draft.header_version != PHASE6A_VAULT_HEADER_VERSION {
        return Err(NativeVaultHeaderReviewError::HeaderVersionMismatch);
    }

    if draft.platform_family != secure_surface.platform_family {
        return Err(NativeVaultHeaderReviewError::PlatformFamilyMismatch);
    }

    if draft.secure_surface_contract_domain != secure_surface.contract_domain
        || draft.secure_surface_contract_domain != PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN
    {
        return Err(NativeVaultHeaderReviewError::SecureSurfaceContractDomainMismatch);
    }

    validate_vault_compartment_headers(&draft.requested_compartment_headers)?;

    if draft.requests_pin_unlock
        || draft.requests_pin_derivation_runtime
        || draft.includes_vault_runtime
        || draft.includes_platform_sealer_implementation
        || draft.stores_secret_material
        || draft.exports_material
        || draft.requests_encryption_or_decryption
        || draft.requests_runtime_io
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeVaultHeaderReviewError::UnsafeVaultHeaderAuthorityFlag);
    }

    Ok(NativeTwoCompartmentVaultHeaderDescriptorV1 {
        header_domain: PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN,
        header_version: PHASE6A_VAULT_HEADER_VERSION,
        platform_family: draft.platform_family,
        secure_surface_contract_domain: PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN,
        compartment_headers: draft.requested_compartment_headers,
        header_only: true,
    })
}

fn validate_vault_compartment_headers(
    headers: &[NativeVaultCompartmentHeaderV1],
) -> Result<(), NativeVaultHeaderReviewError> {
    if headers.is_empty() {
        return Err(NativeVaultHeaderReviewError::MissingCompartmentHeaders);
    }

    let mut has_recovery_root = false;
    let mut has_device_key = false;

    for (index, header) in headers.iter().enumerate() {
        if headers[..index]
            .iter()
            .any(|previous| previous.compartment == header.compartment)
        {
            return Err(NativeVaultHeaderReviewError::DuplicateCompartmentHeader);
        }

        if header.compartment_label != expected_compartment_label(header.compartment) {
            return Err(NativeVaultHeaderReviewError::CompartmentLabelMismatch);
        }

        if header.secure_surface_contract_domain != PHASE5B_SECURE_SURFACE_CONTRACT_DOMAIN
            || header.platform_contract_domain != PHASE5A_PLATFORM_SEALER_CONTRACT_DOMAIN
        {
            return Err(NativeVaultHeaderReviewError::SecureSurfaceContractDomainMismatch);
        }

        if header.kdf_algorithm_label != PHASE6A_KDF_ALGORITHM_LABEL
            || header.aead_algorithm_label != PHASE6A_AEAD_ALGORITHM_LABEL
        {
            return Err(NativeVaultHeaderReviewError::AlgorithmLabelMismatch);
        }

        if header.kdf_salt_len != PHASE6A_KDF_SALT_LEN
            || header.aead_nonce_len != PHASE6A_AEAD_NONCE_LEN
            || header.aead_tag_len != PHASE6A_AEAD_TAG_LEN
        {
            return Err(NativeVaultHeaderReviewError::HeaderLengthMismatch);
        }

        if header.authenticated_header_label != PHASE6A_AUTHENTICATED_HEADER_LABEL {
            return Err(NativeVaultHeaderReviewError::AuthenticatedHeaderLabelMismatch);
        }

        match header.compartment {
            NativeSecureCompartment::RecoveryRoot => has_recovery_root = true,
            NativeSecureCompartment::DeviceKey => has_device_key = true,
        }
    }

    if !has_recovery_root || !has_device_key {
        return Err(NativeVaultHeaderReviewError::MissingRequiredCompartmentHeader);
    }

    Ok(())
}

fn expected_compartment_label(compartment: NativeSecureCompartment) -> &'static str {
    match compartment {
        NativeSecureCompartment::RecoveryRoot => PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL,
        NativeSecureCompartment::DeviceKey => PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL,
    }
}
