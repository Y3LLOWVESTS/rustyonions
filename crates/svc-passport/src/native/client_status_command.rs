//! RO:WHAT — Native Passport desktop `passport_status` command contract.
//! RO:WHY — Phase 15A starts CrabLink desktop integration with a safe redacted status command before create/restore/unlock/proof/capability/username command wiring.
//! RO:INTERACTS — CrabLink desktop command surface, local status inspection posture, Phase 14 route acceptance posture, and React redacted DTO consumers.
//! RO:INVARIANTS — this is a command contract only. It returns redacted status DTOs, requires native-passport gating, does not expose recovery words, signing material, capability material, vault material, routes, runtime I/O, storage mutation, or wallet/ledger mutation.
//! RO:TEST — tests/native_passport_phase15a_client_status_command.rs.

use super::{
    native_gateway_omnigate_route_mount_acceptance_posture, native_local_status_inspection_posture,
    NATIVE_PASSPORT_PHASE13_LABEL, NATIVE_PASSPORT_PHASE14D_LABEL, PHASE13_REDACTED_VALUE,
};

pub const NATIVE_PASSPORT_PHASE15A_LABEL: &str = "NATIVE_PASSPORT_PHASE15A_CLIENT_STATUS_COMMAND";

pub const PHASE15A_CLIENT_STATUS_COMMAND_DOMAIN: &str = "native-passport/client-status-command/v1";

pub const PHASE15A_CLIENT_STATUS_COMMAND_VERSION: u16 = 1;

pub const PHASE15A_PASSPORT_STATUS_COMMAND: &str = "passport_status";

pub const PHASE15A_EXPECTED_DESKTOP_COMMANDS: &[&str] = &[
    "passport_status",
    "passport_create_native",
    "passport_restore_native",
    "passport_unlock_operational",
    "passport_unlock_root",
    "passport_lock",
    "passport_clear",
    "passport_device_list",
    "passport_device_authorize",
    "passport_device_revoke",
    "passport_prove",
    "passport_capability_status",
    "passport_capability_refresh",
    "passport_capability_revoke",
    "passport_username_status",
    "passport_username_claim",
    "passport_username_transfer",
    "passport_username_release",
];

pub const PHASE15A_FORBIDDEN_DESKTOP_COMMANDS: &[&str] = &[
    "passport_get_seed_to_webview",
    "passport_export_private_key",
    "passport_get_device_private_key",
    "passport_get_raw_capability",
    "passport_issue_arbitrary_scope",
    "passport_disable_policy",
];

pub const PHASE15A_FORBIDDEN_STATUS_COMMAND_AUTHORITY_FLAGS: &[&str] = &[
    "seed_to_webview",
    "private_key_export",
    "device_key_export",
    "raw_capability_export",
    "arbitrary_scope_issue",
    "policy_disable",
    "vault_unlock",
    "root_unlock",
    "operational_unlock",
    "platform_sealer_unseal",
    "runtime_io",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeClientStatusLockState {
    NoPassport,
    Locked,
    OperationalUnlocked,
    RootUnlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeClientStatusCapabilityState {
    Absent,
    PresentRedacted,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientStatusCommandDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub command_name: &'static str,
    pub requester_label: &'static str,
    pub surface_label: &'static str,
    pub native_feature_enabled: bool,
    pub desktop_surface: bool,
    pub redacted_dto_required: bool,
    pub latest_completed_phase_label: &'static str,
    pub lock_state: NativeClientStatusLockState,
    pub capability_state: NativeClientStatusCapabilityState,
    pub has_passport_identifier: bool,
    pub has_device_identifier: bool,
    pub has_confirmed_username: bool,
    pub route_acceptance_green: bool,
    pub local_status_inspection_green: bool,
    pub exposes_recovery_words: bool,
    pub exposes_root_signing_material: bool,
    pub exposes_device_signing_material: bool,
    pub exposes_capability_material: bool,
    pub exposes_vault_material: bool,
    pub requests_unlock: bool,
    pub requests_root_confirmation: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_runtime_io: bool,
    pub requests_storage_mutation: bool,
    pub requests_wallet_or_ledger_mutation: bool,
    pub requests_arbitrary_scope_issue: bool,
    pub requests_policy_disable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientStatusCommandDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub command_name: &'static str,
    pub requester_label: &'static str,
    pub surface_label: &'static str,
    pub latest_completed_phase_label: &'static str,
    pub lock_state: NativeClientStatusLockState,
    pub capability_state: NativeClientStatusCapabilityState,
    pub redacted_passport_identifier: &'static str,
    pub redacted_device_identifier: &'static str,
    pub redacted_username_handle: &'static str,
    pub redacted_capability_material: &'static str,
    pub native_feature_enabled: bool,
    pub desktop_surface: bool,
    pub redacted_dto: bool,
    pub route_acceptance_green: bool,
    pub local_status_inspection_green: bool,
    pub seed_phrase_exposed: bool,
    pub private_key_exposed: bool,
    pub raw_capability_exposed: bool,
    pub vault_material_exposed: bool,
    pub unlock_performed: bool,
    pub root_confirmation_requested: bool,
    pub platform_sealer_unseal_performed: bool,
    pub runtime_io_performed: bool,
    pub storage_mutated: bool,
    pub wallet_or_ledger_mutated: bool,
    pub arbitrary_scope_issued: bool,
    pub policy_disabled: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeClientStatusCommandReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    CommandNameMismatch,
    MissingRequesterLabel,
    MissingSurfaceLabel,
    NativeFeatureNotEnabled,
    NotDesktopSurface,
    RedactedDtoNotRequired,
    MissingLatestCompletedPhaseLabel,
    RouteAcceptanceNotGreen,
    LocalStatusInspectionNotGreen,
    SecretMaterialExposure,
    CapabilityMaterialExposure,
    VaultMaterialExposure,
    UnsafeStatusCommandAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientStatusCommandPosture {
    pub phase_label: &'static str,
    pub status_command_contract_added: bool,
    pub passport_status_command_named: bool,
    pub expected_desktop_command_inventory_added: bool,
    pub forbidden_command_inventory_added: bool,
    pub redacted_dto_added: bool,
    pub local_status_inspection_reused: bool,
    pub route_acceptance_reused: bool,
    pub live_tauri_command_added: bool,
    pub create_restore_unlock_command_implementation_added: bool,
    pub proof_or_capability_command_implementation_added: bool,
    pub username_command_implementation_added: bool,
    pub seed_to_webview_added: bool,
    pub private_key_export_added: bool,
    pub device_key_export_added: bool,
    pub raw_capability_export_added: bool,
    pub arbitrary_scope_issue_added: bool,
    pub policy_disable_added: bool,
    pub platform_sealer_unseal_added: bool,
    pub runtime_io_added: bool,
    pub storage_mutation_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_client_status_command_posture() -> NativeClientStatusCommandPosture {
    NativeClientStatusCommandPosture {
        phase_label: NATIVE_PASSPORT_PHASE15A_LABEL,
        status_command_contract_added: true,
        passport_status_command_named: true,
        expected_desktop_command_inventory_added: true,
        forbidden_command_inventory_added: true,
        redacted_dto_added: true,
        local_status_inspection_reused: true,
        route_acceptance_reused: true,
        live_tauri_command_added: false,
        create_restore_unlock_command_implementation_added: false,
        proof_or_capability_command_implementation_added: false,
        username_command_implementation_added: false,
        seed_to_webview_added: false,
        private_key_export_added: false,
        device_key_export_added: false,
        raw_capability_export_added: false,
        arbitrary_scope_issue_added: false,
        policy_disable_added: false,
        platform_sealer_unseal_added: false,
        runtime_io_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE15A_FORBIDDEN_STATUS_COMMAND_AUTHORITY_FLAGS,
    }
}

pub fn review_native_client_status_command(
    draft: NativeClientStatusCommandDraftV1,
) -> Result<NativeClientStatusCommandDecisionV1, NativeClientStatusCommandReviewError> {
    if draft.contract_domain != PHASE15A_CLIENT_STATUS_COMMAND_DOMAIN {
        return Err(NativeClientStatusCommandReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE15A_CLIENT_STATUS_COMMAND_VERSION {
        return Err(NativeClientStatusCommandReviewError::ContractVersionMismatch);
    }

    if draft.command_name != PHASE15A_PASSPORT_STATUS_COMMAND {
        return Err(NativeClientStatusCommandReviewError::CommandNameMismatch);
    }

    if draft.requester_label.is_empty() {
        return Err(NativeClientStatusCommandReviewError::MissingRequesterLabel);
    }

    if draft.surface_label.is_empty() {
        return Err(NativeClientStatusCommandReviewError::MissingSurfaceLabel);
    }

    if !draft.native_feature_enabled {
        return Err(NativeClientStatusCommandReviewError::NativeFeatureNotEnabled);
    }

    if !draft.desktop_surface {
        return Err(NativeClientStatusCommandReviewError::NotDesktopSurface);
    }

    if !draft.redacted_dto_required {
        return Err(NativeClientStatusCommandReviewError::RedactedDtoNotRequired);
    }

    if draft.latest_completed_phase_label.is_empty() {
        return Err(NativeClientStatusCommandReviewError::MissingLatestCompletedPhaseLabel);
    }

    let route_posture = native_gateway_omnigate_route_mount_acceptance_posture();
    if !draft.route_acceptance_green || route_posture.phase_label != NATIVE_PASSPORT_PHASE14D_LABEL
    {
        return Err(NativeClientStatusCommandReviewError::RouteAcceptanceNotGreen);
    }

    let local_posture = native_local_status_inspection_posture();
    if !draft.local_status_inspection_green
        || local_posture.phase_label != NATIVE_PASSPORT_PHASE13_LABEL
    {
        return Err(NativeClientStatusCommandReviewError::LocalStatusInspectionNotGreen);
    }

    if draft.exposes_recovery_words
        || draft.exposes_root_signing_material
        || draft.exposes_device_signing_material
    {
        return Err(NativeClientStatusCommandReviewError::SecretMaterialExposure);
    }

    if draft.exposes_capability_material {
        return Err(NativeClientStatusCommandReviewError::CapabilityMaterialExposure);
    }

    if draft.exposes_vault_material {
        return Err(NativeClientStatusCommandReviewError::VaultMaterialExposure);
    }

    validate_no_unsafe_flags(&draft)?;

    Ok(NativeClientStatusCommandDecisionV1 {
        contract_domain: PHASE15A_CLIENT_STATUS_COMMAND_DOMAIN,
        contract_version: PHASE15A_CLIENT_STATUS_COMMAND_VERSION,
        command_name: PHASE15A_PASSPORT_STATUS_COMMAND,
        requester_label: draft.requester_label,
        surface_label: draft.surface_label,
        latest_completed_phase_label: draft.latest_completed_phase_label,
        lock_state: draft.lock_state,
        capability_state: draft.capability_state,
        redacted_passport_identifier: redacted_if_present(draft.has_passport_identifier),
        redacted_device_identifier: redacted_if_present(draft.has_device_identifier),
        redacted_username_handle: redacted_if_present(draft.has_confirmed_username),
        redacted_capability_material: redacted_if_present(matches!(
            draft.capability_state,
            NativeClientStatusCapabilityState::PresentRedacted
        )),
        native_feature_enabled: true,
        desktop_surface: true,
        redacted_dto: true,
        route_acceptance_green: true,
        local_status_inspection_green: true,
        seed_phrase_exposed: false,
        private_key_exposed: false,
        raw_capability_exposed: false,
        vault_material_exposed: false,
        unlock_performed: false,
        root_confirmation_requested: false,
        platform_sealer_unseal_performed: false,
        runtime_io_performed: false,
        storage_mutated: false,
        wallet_or_ledger_mutated: false,
        arbitrary_scope_issued: false,
        policy_disabled: false,
        contract_only: true,
    })
}

fn redacted_if_present(present: bool) -> &'static str {
    if present {
        PHASE13_REDACTED_VALUE
    } else {
        "ABSENT"
    }
}

fn validate_no_unsafe_flags(
    draft: &NativeClientStatusCommandDraftV1,
) -> Result<(), NativeClientStatusCommandReviewError> {
    if draft.requests_unlock
        || draft.requests_root_confirmation
        || draft.requests_platform_sealer_unseal
        || draft.requests_runtime_io
        || draft.requests_storage_mutation
        || draft.requests_wallet_or_ledger_mutation
        || draft.requests_arbitrary_scope_issue
        || draft.requests_policy_disable
    {
        return Err(NativeClientStatusCommandReviewError::UnsafeStatusCommandAuthorityFlag);
    }

    Ok(())
}
