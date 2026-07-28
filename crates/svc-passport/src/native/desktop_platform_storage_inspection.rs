//! RO:WHAT — Inspects and locks ownership boundaries for desktop PlatformSealer and atomic VaultStore implementation.
//! RO:WHY — Phase 15H must decide where platform-neutral contracts, OS adapters, vault persistence, and frontend DTOs live before create/restore/unlock runtime begins.
//! RO:INTERACTS — Phase 5A PlatformSealer contract, Phase 6A vault-header contract, CrabLink Tauri state/commands, and the redacted React boundary.
//! RO:INVARIANTS — svc-passport owns platform-neutral traits and errors; CrabLink Tauri owns OS and app-data adapters; frontend code owns only redacted DTO consumption.
//! RO:SECURITY — inspection only: no keychain/DPAPI/Secret Service call, vault read/write, encryption, decryption, unlock, secret storage, capability issuance, or wallet/ledger mutation.
//! RO:TEST — tests/native_passport_phase15h_platform_sealer_vault_store_inspection.rs.

use super::{
    native_passport_platform_sealer_posture, native_passport_vault_header_posture,
    NativePlatformFamily, NATIVE_PASSPORT_PHASE5A_LABEL, NATIVE_PASSPORT_PHASE6A_LABEL,
};

pub const NATIVE_PASSPORT_PHASE15H_LABEL: &str =
    "NATIVE_PASSPORT_PHASE15H_PLATFORM_SEALER_VAULT_STORE_ADAPTER_INSPECTION";

pub const PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_DOMAIN: &str =
    "native-passport/desktop-platform-storage-inspection/v1";

pub const PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_VERSION: u16 = 1;

pub const PHASE15H_PLATFORM_NEUTRAL_OWNER: &str = "crates/svc-passport";

pub const PHASE15H_PLATFORM_ADAPTER_OWNER: &str = "apps/crablink-tauri/src-tauri";

pub const PHASE15H_FRONTEND_OWNER: &str = "apps/crablink-tauri/src";

pub const PHASE15H_TAURI_CARGO_PATH: &str = "apps/crablink-tauri/src-tauri/Cargo.toml";

pub const PHASE15H_TAURI_STATE_PATH: &str = "apps/crablink-tauri/src-tauri/src/state.rs";

pub const PHASE15H_TAURI_PASSPORT_COMMAND_PATH: &str =
    "apps/crablink-tauri/src-tauri/src/commands/passport.rs";

pub const PHASE15H_REQUIRED_DESKTOP_PLATFORM_FAMILIES: &[NativePlatformFamily] = &[
    NativePlatformFamily::MacosKeychain,
    NativePlatformFamily::WindowsDpapi,
    NativePlatformFamily::LinuxSecretService,
];

pub const PHASE15H_REQUIRED_ATOMIC_WRITE_STEPS: &[&str] = &[
    "write_encrypted_temporary_file",
    "sync_temporary_file",
    "sync_parent_directory_where_supported",
    "atomic_rename",
    "sync_parent_directory_after_rename_where_supported",
    "remove_stale_temporary_file",
];

pub const PHASE15H_FORBIDDEN_PLATFORM_STORAGE_FLAGS: &[&str] = &[
    "plaintext_temporary_file",
    "frontend_secret_custody",
    "webview_pin_custody",
    "webview_recovery_custody",
    "raw_platform_secret_export",
    "dev_kms_native_authority",
    "vault_unlock",
    "encryption_runtime",
    "decryption_runtime",
    "runtime_io",
    "secret_storage",
    "capability_issuance",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDesktopPlatformStorageInspectionDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub requester_label: &'static str,

    pub platform_neutral_owner: &'static str,
    pub platform_adapter_owner: &'static str,
    pub frontend_owner: &'static str,

    pub phase5a_sealer_contract_reviewed: bool,
    pub phase6a_vault_header_reviewed: bool,

    pub direct_svc_passport_dependency_present: bool,
    pub native_passport_feature_enabled: bool,
    pub dev_kms_disabled: bool,
    pub passport_status_registered: bool,

    pub platform_sealer_backend_dependency_present: bool,
    pub platform_sealer_adapter_present: bool,
    pub vault_store_adapter_present: bool,
    pub passport_runtime_state_owner_present: bool,

    pub frontend_redacted_only: bool,
    pub frontend_secret_custody_present: bool,
    pub plaintext_temporary_file_requested: bool,
    pub raw_platform_material_export_requested: bool,

    pub platform_sealer_runtime_requested: bool,
    pub vault_store_runtime_requested: bool,
    pub vault_unlock_requested: bool,
    pub encryption_or_decryption_requested: bool,
    pub runtime_io_requested: bool,
    pub secret_storage_requested: bool,
    pub capability_issuance_requested: bool,
    pub wallet_or_ledger_mutation_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDesktopPlatformStorageInspectionDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub requester_label: &'static str,

    pub platform_neutral_owner: &'static str,
    pub platform_adapter_owner: &'static str,
    pub frontend_owner: &'static str,

    pub desktop_platform_families: &'static [NativePlatformFamily],
    pub atomic_write_steps: &'static [&'static str],

    pub sealer_backend_pin_required: bool,
    pub platform_sealer_trait_required: bool,
    pub vault_store_trait_required: bool,
    pub tauri_os_adapters_required: bool,
    pub tauri_app_data_adapter_required: bool,

    pub frontend_redacted_only: bool,
    pub plaintext_temporary_files_forbidden: bool,
    pub raw_platform_material_export_forbidden: bool,

    pub platform_sealer_runtime_added: bool,
    pub vault_store_runtime_added: bool,
    pub vault_unlock_added: bool,
    pub encryption_or_decryption_added: bool,
    pub runtime_io_added: bool,
    pub secret_storage_added: bool,
    pub capability_issuance_added: bool,
    pub wallet_or_ledger_mutation_added: bool,

    pub ready_for_platform_trait_foundation: bool,
    pub inspection_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDesktopPlatformStorageInspectionReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    MissingRequesterLabel,
    OwnershipBoundaryMismatch,
    PriorContractNotReviewed,
    DesktopDependencyPostureMismatch,
    ExistingRuntimePostureChanged,
    FrontendBoundaryUnsafe,
    UnsafePlatformStorageAuthorityFlag,
    SealerContractPostureUnsafe,
    VaultHeaderPostureUnsafe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDesktopPlatformStorageInspectionPosture {
    pub phase_label: &'static str,
    pub ownership_boundary_locked: bool,
    pub desktop_platform_families_locked: bool,
    pub atomic_write_sequence_locked: bool,
    pub platform_sealer_trait_added: bool,
    pub vault_store_trait_added: bool,
    pub platform_sealer_adapter_added: bool,
    pub vault_store_adapter_added: bool,
    pub runtime_state_owner_added: bool,
    pub frontend_secret_custody_added: bool,
    pub runtime_io_added: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_flags: &'static [&'static str],
}

pub fn native_desktop_platform_storage_inspection_posture(
) -> NativeDesktopPlatformStorageInspectionPosture {
    NativeDesktopPlatformStorageInspectionPosture {
        phase_label: NATIVE_PASSPORT_PHASE15H_LABEL,
        ownership_boundary_locked: true,
        desktop_platform_families_locked: true,
        atomic_write_sequence_locked: true,
        platform_sealer_trait_added: false,
        vault_store_trait_added: false,
        platform_sealer_adapter_added: false,
        vault_store_adapter_added: false,
        runtime_state_owner_added: false,
        frontend_secret_custody_added: false,
        runtime_io_added: false,
        native_secret_implementation_added: false,
        forbidden_flags: PHASE15H_FORBIDDEN_PLATFORM_STORAGE_FLAGS,
    }
}

pub fn review_native_desktop_platform_storage_inspection(
    draft: NativeDesktopPlatformStorageInspectionDraftV1,
) -> Result<
    NativeDesktopPlatformStorageInspectionDecisionV1,
    NativeDesktopPlatformStorageInspectionReviewError,
> {
    if draft.contract_domain != PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_DOMAIN {
        return Err(NativeDesktopPlatformStorageInspectionReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_VERSION {
        return Err(NativeDesktopPlatformStorageInspectionReviewError::ContractVersionMismatch);
    }

    if draft.requester_label.is_empty() {
        return Err(NativeDesktopPlatformStorageInspectionReviewError::MissingRequesterLabel);
    }

    if draft.platform_neutral_owner != PHASE15H_PLATFORM_NEUTRAL_OWNER
        || draft.platform_adapter_owner != PHASE15H_PLATFORM_ADAPTER_OWNER
        || draft.frontend_owner != PHASE15H_FRONTEND_OWNER
    {
        return Err(NativeDesktopPlatformStorageInspectionReviewError::OwnershipBoundaryMismatch);
    }

    if !draft.phase5a_sealer_contract_reviewed || !draft.phase6a_vault_header_reviewed {
        return Err(NativeDesktopPlatformStorageInspectionReviewError::PriorContractNotReviewed);
    }

    if !draft.direct_svc_passport_dependency_present
        || !draft.native_passport_feature_enabled
        || !draft.dev_kms_disabled
        || !draft.passport_status_registered
    {
        return Err(
            NativeDesktopPlatformStorageInspectionReviewError::DesktopDependencyPostureMismatch,
        );
    }

    if draft.platform_sealer_backend_dependency_present
        || draft.platform_sealer_adapter_present
        || draft.vault_store_adapter_present
        || draft.passport_runtime_state_owner_present
    {
        return Err(
            NativeDesktopPlatformStorageInspectionReviewError::ExistingRuntimePostureChanged,
        );
    }

    if !draft.frontend_redacted_only
        || draft.frontend_secret_custody_present
        || draft.plaintext_temporary_file_requested
        || draft.raw_platform_material_export_requested
    {
        return Err(NativeDesktopPlatformStorageInspectionReviewError::FrontendBoundaryUnsafe);
    }

    if draft.platform_sealer_runtime_requested
        || draft.vault_store_runtime_requested
        || draft.vault_unlock_requested
        || draft.encryption_or_decryption_requested
        || draft.runtime_io_requested
        || draft.secret_storage_requested
        || draft.capability_issuance_requested
        || draft.wallet_or_ledger_mutation_requested
    {
        return Err(
            NativeDesktopPlatformStorageInspectionReviewError::UnsafePlatformStorageAuthorityFlag,
        );
    }

    validate_prior_postures()?;

    Ok(NativeDesktopPlatformStorageInspectionDecisionV1 {
        contract_domain: PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_DOMAIN,
        contract_version: PHASE15H_DESKTOP_PLATFORM_STORAGE_INSPECTION_VERSION,
        requester_label: draft.requester_label,

        platform_neutral_owner: PHASE15H_PLATFORM_NEUTRAL_OWNER,
        platform_adapter_owner: PHASE15H_PLATFORM_ADAPTER_OWNER,
        frontend_owner: PHASE15H_FRONTEND_OWNER,

        desktop_platform_families: PHASE15H_REQUIRED_DESKTOP_PLATFORM_FAMILIES,
        atomic_write_steps: PHASE15H_REQUIRED_ATOMIC_WRITE_STEPS,

        sealer_backend_pin_required: true,
        platform_sealer_trait_required: true,
        vault_store_trait_required: true,
        tauri_os_adapters_required: true,
        tauri_app_data_adapter_required: true,

        frontend_redacted_only: true,
        plaintext_temporary_files_forbidden: true,
        raw_platform_material_export_forbidden: true,

        platform_sealer_runtime_added: false,
        vault_store_runtime_added: false,
        vault_unlock_added: false,
        encryption_or_decryption_added: false,
        runtime_io_added: false,
        secret_storage_added: false,
        capability_issuance_added: false,
        wallet_or_ledger_mutation_added: false,

        ready_for_platform_trait_foundation: true,
        inspection_only: true,
    })
}

fn validate_prior_postures() -> Result<(), NativeDesktopPlatformStorageInspectionReviewError> {
    let sealer = native_passport_platform_sealer_posture();

    if sealer.phase_label != NATIVE_PASSPORT_PHASE5A_LABEL
        || !sealer.platform_sealer_contract_dtos_added
        || sealer.platform_sealer_implementation_added
        || sealer.secret_storage_added
        || sealer.material_export_added
        || sealer.encryption_runtime_added
        || sealer.decryption_runtime_added
        || sealer.vault_runtime_added
        || sealer.runtime_authority_changed
        || sealer.native_secret_implementation_added
    {
        return Err(NativeDesktopPlatformStorageInspectionReviewError::SealerContractPostureUnsafe);
    }

    let vault = native_passport_vault_header_posture();

    if vault.phase_label != NATIVE_PASSPORT_PHASE6A_LABEL
        || !vault.vault_header_dtos_added
        || vault.pin_unlock_added
        || vault.pin_derivation_runtime_added
        || vault.vault_runtime_added
        || vault.platform_sealer_implementation_added
        || vault.secret_storage_added
        || vault.material_export_added
        || vault.encryption_runtime_added
        || vault.decryption_runtime_added
        || vault.runtime_io_added
        || vault.runtime_authority_changed
        || vault.native_secret_implementation_added
    {
        return Err(NativeDesktopPlatformStorageInspectionReviewError::VaultHeaderPostureUnsafe);
    }

    Ok(())
}
