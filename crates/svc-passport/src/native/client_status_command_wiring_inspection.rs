//! RO:WHAT — Records the exact CrabLink desktop anchors for the first live Native Passport `passport_status` command.
//! RO:WHY — Phase 15E converts the Phase 15A–15D contracts into a concrete cross-repository wiring map before Tauri runtime or vault behavior is added.
//! RO:INTERACTS — CrabLink Tauri Cargo.toml, commands/mod.rs, lib.rs, state.rs, identity.rs, and Phase 15D desktop command-surface acceptance.
//! RO:INVARIANTS — inspection only; `passport_status` is first; React remains redacted; dev passport labels are compatibility-only; no command wiring, runtime I/O, storage mutation, wallet/ledger mutation, or secret exposure.
//! RO:TEST — tests/native_passport_phase15e_desktop_status_command_wiring_inspection.rs.

use super::{
    native_client_command_surface_acceptance_posture,
    NativeClientCommandSurfaceAcceptanceDecisionV1, NATIVE_PASSPORT_PHASE15D_LABEL,
    PHASE15A_PASSPORT_STATUS_COMMAND, PHASE15C_REDACTED_DTO_TARGET,
    PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN,
    PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_VERSION,
};

pub const NATIVE_PASSPORT_PHASE15E_LABEL: &str =
    "NATIVE_PASSPORT_PHASE15E_DESKTOP_STATUS_COMMAND_WIRING_INSPECTION";
pub const PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_DOMAIN: &str =
    "native-passport/desktop-status-command-wiring-inspection/v1";
pub const PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_VERSION: u16 = 1;

pub const PHASE15E_TAURI_CRATE_ROOT: &str = "apps/crablink-tauri/src-tauri";
pub const PHASE15E_TAURI_CARGO_PATH: &str = "apps/crablink-tauri/src-tauri/Cargo.toml";
pub const PHASE15E_COMMAND_MODULE_REGISTRY_PATH: &str =
    "apps/crablink-tauri/src-tauri/src/commands/mod.rs";
pub const PHASE15E_COMMAND_HANDLER_REGISTRY_PATH: &str = "apps/crablink-tauri/src-tauri/src/lib.rs";
pub const PHASE15E_APP_STATE_PATH: &str = "apps/crablink-tauri/src-tauri/src/state.rs";
pub const PHASE15E_EXISTING_IDENTITY_COMMAND_PATH: &str =
    "apps/crablink-tauri/src-tauri/src/commands/identity.rs";
pub const PHASE15E_PROPOSED_PASSPORT_COMMAND_PATH: &str =
    "apps/crablink-tauri/src-tauri/src/commands/passport.rs";
pub const PHASE15E_EXISTING_IDENTITY_COMMAND_NAME: &str = "identity_me_gateway";

pub const PHASE15E_FORBIDDEN_WIRING_INSPECTION_FLAGS: &[&str] = &[
    "live_status_command",
    "status_command_registry_hook",
    "direct_dependency_pin",
    "dev_label_as_native_truth",
    "runtime_io",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
    "secret_material_to_react",
    "capability_material_to_react",
    "vault_material_to_react",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientStatusCommandWiringInspectionDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub requester_label: &'static str,
    pub tauri_crate_root: &'static str,
    pub tauri_cargo_path: &'static str,
    pub command_module_registry_path: &'static str,
    pub command_handler_registry_path: &'static str,
    pub app_state_path: &'static str,
    pub existing_identity_command_path: &'static str,
    pub proposed_passport_command_path: &'static str,
    pub status_command_name: &'static str,
    pub existing_identity_command_name: &'static str,
    pub native_feature_enabled: bool,
    pub desktop_surface: bool,
    pub phase15d_acceptance_reviewed: bool,
    pub direct_svc_passport_dependency_present: bool,
    pub crablink_native_core_dependency_present: bool,
    pub dev_passport_label_present: bool,
    pub dev_label_treated_as_native_truth: bool,
    pub live_status_command_present: bool,
    pub status_command_registered: bool,
    pub forbidden_command_target_present: bool,
    pub redacted_dto_target_confirmed: bool,
    pub exposes_secret_material: bool,
    pub exposes_capability_material: bool,
    pub exposes_vault_material: bool,
    pub requests_runtime_change: bool,
    pub requests_storage_mutation: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientStatusCommandWiringInspectionDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub requester_label: &'static str,
    pub status_command_name: &'static str,
    pub response_target: &'static str,
    pub tauri_crate_root: &'static str,
    pub tauri_cargo_path: &'static str,
    pub command_module_registry_path: &'static str,
    pub command_handler_registry_path: &'static str,
    pub app_state_path: &'static str,
    pub existing_identity_command_path: &'static str,
    pub proposed_passport_command_path: &'static str,
    pub existing_identity_command_name: &'static str,
    pub direct_svc_passport_dependency_present: bool,
    pub crablink_native_core_dependency_present: bool,
    pub svc_passport_dependency_pin_required_before_live_wiring: bool,
    pub existing_dev_identity_is_compatibility_only: bool,
    pub live_status_command_present: bool,
    pub status_command_registered: bool,
    pub ready_for_phase15f_status_wiring: bool,
    pub runtime_io_performed: bool,
    pub storage_mutated: bool,
    pub wallet_or_ledger_mutated: bool,
    pub material_exposed: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeClientStatusCommandWiringInspectionReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    MissingRequesterLabel,
    NativeFeatureNotEnabled,
    NotDesktopSurface,
    Phase15DAcceptanceNotReviewed,
    TauriAnchorMismatch,
    StatusCommandNameMismatch,
    ExistingIdentityCommandNameMismatch,
    DirectSvcPassportDependencyUnexpected,
    CrablinkNativeCoreDependencyNotObserved,
    DevPassportCompatibilityNotObserved,
    DevPassportLabelPromotedToNativeTruth,
    LiveStatusCommandUnexpected,
    StatusCommandRegistrationUnexpected,
    ForbiddenCommandTargetObserved,
    RedactedDtoTargetNotConfirmed,
    Phase15DPostureNotGreen,
    Phase15DAcceptanceUnsafe,
    MaterialExposureObserved,
    UnsafeInspectionAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientStatusCommandWiringInspectionPosture {
    pub phase_label: &'static str,
    pub exact_tauri_anchors_inspected: bool,
    pub status_command_selected_first: bool,
    pub phase15d_acceptance_reused: bool,
    pub redacted_react_target_reused: bool,
    pub dev_identity_compatibility_identified: bool,
    pub dependency_pin_required_before_live_wiring: bool,
    pub live_status_command_added: bool,
    pub command_registry_hook_added: bool,
    pub runtime_io_added: bool,
    pub storage_mutation_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_client_status_command_wiring_inspection_posture(
) -> NativeClientStatusCommandWiringInspectionPosture {
    NativeClientStatusCommandWiringInspectionPosture {
        phase_label: NATIVE_PASSPORT_PHASE15E_LABEL,
        exact_tauri_anchors_inspected: true,
        status_command_selected_first: true,
        phase15d_acceptance_reused: true,
        redacted_react_target_reused: true,
        dev_identity_compatibility_identified: true,
        dependency_pin_required_before_live_wiring: true,
        live_status_command_added: false,
        command_registry_hook_added: false,
        runtime_io_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE15E_FORBIDDEN_WIRING_INSPECTION_FLAGS,
    }
}

pub fn review_native_client_status_command_wiring_inspection(
    draft: NativeClientStatusCommandWiringInspectionDraftV1,
    acceptance: &NativeClientCommandSurfaceAcceptanceDecisionV1,
) -> Result<
    NativeClientStatusCommandWiringInspectionDecisionV1,
    NativeClientStatusCommandWiringInspectionReviewError,
> {
    validate_draft(&draft)?;
    validate_phase15d_acceptance(acceptance)?;

    Ok(NativeClientStatusCommandWiringInspectionDecisionV1 {
        contract_domain: PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_DOMAIN,
        contract_version: PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_VERSION,
        requester_label: draft.requester_label,
        status_command_name: PHASE15A_PASSPORT_STATUS_COMMAND,
        response_target: PHASE15C_REDACTED_DTO_TARGET,
        tauri_crate_root: PHASE15E_TAURI_CRATE_ROOT,
        tauri_cargo_path: PHASE15E_TAURI_CARGO_PATH,
        command_module_registry_path: PHASE15E_COMMAND_MODULE_REGISTRY_PATH,
        command_handler_registry_path: PHASE15E_COMMAND_HANDLER_REGISTRY_PATH,
        app_state_path: PHASE15E_APP_STATE_PATH,
        existing_identity_command_path: PHASE15E_EXISTING_IDENTITY_COMMAND_PATH,
        proposed_passport_command_path: PHASE15E_PROPOSED_PASSPORT_COMMAND_PATH,
        existing_identity_command_name: PHASE15E_EXISTING_IDENTITY_COMMAND_NAME,
        direct_svc_passport_dependency_present: false,
        crablink_native_core_dependency_present: true,
        svc_passport_dependency_pin_required_before_live_wiring: true,
        existing_dev_identity_is_compatibility_only: true,
        live_status_command_present: false,
        status_command_registered: false,
        ready_for_phase15f_status_wiring: true,
        runtime_io_performed: false,
        storage_mutated: false,
        wallet_or_ledger_mutated: false,
        material_exposed: false,
        contract_only: true,
    })
}

fn validate_draft(
    draft: &NativeClientStatusCommandWiringInspectionDraftV1,
) -> Result<(), NativeClientStatusCommandWiringInspectionReviewError> {
    if draft.contract_domain != PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_DOMAIN {
        return Err(NativeClientStatusCommandWiringInspectionReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE15E_STATUS_COMMAND_WIRING_INSPECTION_VERSION {
        return Err(NativeClientStatusCommandWiringInspectionReviewError::ContractVersionMismatch);
    }

    if draft.requester_label.is_empty() {
        return Err(NativeClientStatusCommandWiringInspectionReviewError::MissingRequesterLabel);
    }

    if !draft.native_feature_enabled {
        return Err(NativeClientStatusCommandWiringInspectionReviewError::NativeFeatureNotEnabled);
    }

    if !draft.desktop_surface {
        return Err(NativeClientStatusCommandWiringInspectionReviewError::NotDesktopSurface);
    }

    if !draft.phase15d_acceptance_reviewed {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::Phase15DAcceptanceNotReviewed,
        );
    }

    if [
        (draft.tauri_crate_root, PHASE15E_TAURI_CRATE_ROOT),
        (draft.tauri_cargo_path, PHASE15E_TAURI_CARGO_PATH),
        (
            draft.command_module_registry_path,
            PHASE15E_COMMAND_MODULE_REGISTRY_PATH,
        ),
        (
            draft.command_handler_registry_path,
            PHASE15E_COMMAND_HANDLER_REGISTRY_PATH,
        ),
        (draft.app_state_path, PHASE15E_APP_STATE_PATH),
        (
            draft.existing_identity_command_path,
            PHASE15E_EXISTING_IDENTITY_COMMAND_PATH,
        ),
        (
            draft.proposed_passport_command_path,
            PHASE15E_PROPOSED_PASSPORT_COMMAND_PATH,
        ),
    ]
    .iter()
    .any(|(actual, expected)| actual != expected)
    {
        return Err(NativeClientStatusCommandWiringInspectionReviewError::TauriAnchorMismatch);
    }

    if draft.status_command_name != PHASE15A_PASSPORT_STATUS_COMMAND {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::StatusCommandNameMismatch,
        );
    }

    if draft.existing_identity_command_name != PHASE15E_EXISTING_IDENTITY_COMMAND_NAME {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::ExistingIdentityCommandNameMismatch,
        );
    }

    if draft.direct_svc_passport_dependency_present {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::DirectSvcPassportDependencyUnexpected,
        );
    }

    if !draft.crablink_native_core_dependency_present {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::CrablinkNativeCoreDependencyNotObserved,
        );
    }

    if !draft.dev_passport_label_present {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::DevPassportCompatibilityNotObserved,
        );
    }

    if draft.dev_label_treated_as_native_truth {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::DevPassportLabelPromotedToNativeTruth,
        );
    }

    if draft.live_status_command_present {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::LiveStatusCommandUnexpected,
        );
    }

    if draft.status_command_registered {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::StatusCommandRegistrationUnexpected,
        );
    }

    if draft.forbidden_command_target_present {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::ForbiddenCommandTargetObserved,
        );
    }

    if !draft.redacted_dto_target_confirmed {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::RedactedDtoTargetNotConfirmed,
        );
    }

    if draft.exposes_secret_material
        || draft.exposes_capability_material
        || draft.exposes_vault_material
    {
        return Err(NativeClientStatusCommandWiringInspectionReviewError::MaterialExposureObserved);
    }

    if draft.requests_runtime_change
        || draft.requests_storage_mutation
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(
            NativeClientStatusCommandWiringInspectionReviewError::UnsafeInspectionAuthorityFlag,
        );
    }

    Ok(())
}

fn validate_phase15d_acceptance(
    acceptance: &NativeClientCommandSurfaceAcceptanceDecisionV1,
) -> Result<(), NativeClientStatusCommandWiringInspectionReviewError> {
    if native_client_command_surface_acceptance_posture().phase_label
        != NATIVE_PASSPORT_PHASE15D_LABEL
    {
        return Err(NativeClientStatusCommandWiringInspectionReviewError::Phase15DPostureNotGreen);
    }

    if acceptance.contract_domain != PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN
        || acceptance.contract_version != PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_VERSION
        || acceptance.target_label != PHASE15C_REDACTED_DTO_TARGET
        || !acceptance.status_command_reviewed
        || !acceptance.command_inventory_reviewed
        || !acceptance.redacted_dto_adapter_reviewed
        || !acceptance.all_desktop_commands_allowlisted
        || !acceptance.forbidden_commands_denied
        || !acceptance.react_receives_only_redacted_dtos
        || !acceptance.status_command_available
        || !acceptance.lifecycle_command_contracts_available
        || !acceptance.unlock_command_contracts_available
        || !acceptance.device_command_contracts_available
        || !acceptance.proof_command_contracts_available
        || !acceptance.capability_command_contracts_available
        || !acceptance.username_command_contracts_available
        || acceptance.live_desktop_command_wiring_added
        || acceptance.lifecycle_runtime_added
        || acceptance.unlock_runtime_added
        || acceptance.proof_capability_username_runtime_added
        || acceptance.arbitrary_scope_issued
        || acceptance.policy_disabled
        || acceptance.platform_sealer_unseal_performed
        || acceptance.runtime_io_performed
        || acceptance.storage_mutated
        || acceptance.wallet_or_ledger_mutated
        || acceptance.secret_material_exposed
        || acceptance.capability_material_exposed
        || acceptance.vault_material_exposed
        || !acceptance.command_surface_contract_complete
        || acceptance.full_desktop_runtime_complete
        || !acceptance.contract_only
    {
        return Err(NativeClientStatusCommandWiringInspectionReviewError::Phase15DAcceptanceUnsafe);
    }

    Ok(())
}
