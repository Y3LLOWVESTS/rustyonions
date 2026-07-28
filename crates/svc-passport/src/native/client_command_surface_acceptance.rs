//! RO:WHAT — Native Passport desktop command surface acceptance contract.
//! RO:WHY — Phase 15D closes the safe desktop command surface chain before live desktop command wiring.
//! RO:INTERACTS — Phase 15A status command, Phase 15B command inventory/denylist, Phase 15C redacted DTO adapter, CrabLink desktop callers, and React DTO consumers.
//! RO:INVARIANTS — accepts only reviewed/redacted/allowlisted command decisions. This phase does not add live command wiring, lifecycle runtime, unlock runtime, platform unseal, runtime I/O, storage mutation, wallet/ledger mutation, or secret implementation.
//! RO:TEST — tests/native_passport_phase15d_desktop_command_surface_acceptance.rs.

use super::{
    native_client_command_inventory_posture, native_client_redacted_command_dto_adapter_posture,
    native_client_status_command_posture, NativeClientCommandInventoryDecisionV1,
    NativeClientRedactedCommandDtoEnvelopeV1, NativeClientStatusCommandDecisionV1,
    NATIVE_PASSPORT_PHASE15A_LABEL, NATIVE_PASSPORT_PHASE15B_LABEL, NATIVE_PASSPORT_PHASE15C_LABEL,
    PHASE15A_PASSPORT_STATUS_COMMAND, PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN,
    PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION, PHASE15B_EXPECTED_COMMAND_COUNT,
    PHASE15B_EXPECTED_FORBIDDEN_COMMAND_COUNT, PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_DOMAIN,
    PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_VERSION, PHASE15C_REDACTED_DTO_TARGET,
};

pub const NATIVE_PASSPORT_PHASE15D_LABEL: &str =
    "NATIVE_PASSPORT_PHASE15D_DESKTOP_COMMAND_SURFACE_ACCEPTANCE";

pub const PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN: &str =
    "native-passport/desktop-command-surface-acceptance/v1";

pub const PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_VERSION: u16 = 1;

pub const PHASE15D_FORBIDDEN_COMMAND_SURFACE_FLAGS: &[&str] = &[
    "unreviewed_desktop_command",
    "unredacted_react_dto",
    "forbidden_desktop_command",
    "secret_material_to_react",
    "capability_material_to_react",
    "vault_material_to_react",
    "live_tauri_command",
    "lifecycle_runtime",
    "unlock_runtime",
    "platform_sealer_unseal",
    "runtime_io",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientCommandSurfaceAcceptanceDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub requester_label: &'static str,
    pub surface_label: &'static str,
    pub target_label: &'static str,
    pub expected_command_count: usize,
    pub expected_forbidden_command_count: usize,
    pub status_command_reviewed: bool,
    pub command_inventory_reviewed: bool,
    pub redacted_dto_adapter_reviewed: bool,
    pub all_desktop_commands_allowlisted: bool,
    pub forbidden_commands_denied: bool,
    pub react_receives_only_redacted_dtos: bool,
    pub status_command_available: bool,
    pub lifecycle_command_contracts_available: bool,
    pub unlock_command_contracts_available: bool,
    pub device_command_contracts_available: bool,
    pub proof_command_contracts_available: bool,
    pub capability_command_contracts_available: bool,
    pub username_command_contracts_available: bool,
    pub live_desktop_command_wiring_requested: bool,
    pub lifecycle_runtime_requested: bool,
    pub unlock_runtime_requested: bool,
    pub proof_capability_username_runtime_requested: bool,
    pub arbitrary_scope_issue_requested: bool,
    pub policy_disable_requested: bool,
    pub platform_sealer_unseal_requested: bool,
    pub runtime_io_requested: bool,
    pub storage_mutation_requested: bool,
    pub wallet_or_ledger_mutation_requested: bool,
    pub secret_material_exposure_requested: bool,
    pub capability_material_exposure_requested: bool,
    pub vault_material_exposure_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientCommandSurfaceAcceptanceDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub requester_label: &'static str,
    pub surface_label: &'static str,
    pub target_label: &'static str,
    pub command_count: usize,
    pub forbidden_command_count: usize,
    pub status_command_reviewed: bool,
    pub command_inventory_reviewed: bool,
    pub redacted_dto_adapter_reviewed: bool,
    pub all_desktop_commands_allowlisted: bool,
    pub forbidden_commands_denied: bool,
    pub react_receives_only_redacted_dtos: bool,
    pub status_command_available: bool,
    pub lifecycle_command_contracts_available: bool,
    pub unlock_command_contracts_available: bool,
    pub device_command_contracts_available: bool,
    pub proof_command_contracts_available: bool,
    pub capability_command_contracts_available: bool,
    pub username_command_contracts_available: bool,
    pub live_desktop_command_wiring_added: bool,
    pub lifecycle_runtime_added: bool,
    pub unlock_runtime_added: bool,
    pub proof_capability_username_runtime_added: bool,
    pub arbitrary_scope_issued: bool,
    pub policy_disabled: bool,
    pub platform_sealer_unseal_performed: bool,
    pub runtime_io_performed: bool,
    pub storage_mutated: bool,
    pub wallet_or_ledger_mutated: bool,
    pub secret_material_exposed: bool,
    pub capability_material_exposed: bool,
    pub vault_material_exposed: bool,
    pub command_surface_contract_complete: bool,
    pub full_desktop_runtime_complete: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeClientCommandSurfaceAcceptanceReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    MissingRequesterLabel,
    MissingSurfaceLabel,
    TargetMismatch,
    CommandCountMismatch,
    ForbiddenCommandCountMismatch,
    StatusCommandNotReviewed,
    CommandInventoryNotReviewed,
    RedactedDtoAdapterNotReviewed,
    CommandAllowlistNotComplete,
    ForbiddenCommandsNotDenied,
    ReactDtoNotRedacted,
    CommandContractGroupMissing,
    StatusPostureNotGreen,
    InventoryPostureNotGreen,
    RedactedDtoAdapterPostureNotGreen,
    StatusDecisionUnsafe,
    InventoryDecisionUnsafe,
    RedactedDtoEnvelopeUnsafe,
    ExposureRequested,
    LiveDesktopWiringRequested,
    UnsafeDesktopCommandSurfaceAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientCommandSurfaceAcceptancePosture {
    pub phase_label: &'static str,
    pub command_surface_acceptance_added: bool,
    pub status_command_contract_reused: bool,
    pub command_inventory_contract_reused: bool,
    pub redacted_dto_adapter_reused: bool,
    pub command_group_acceptance_added: bool,
    pub redacted_react_dto_acceptance_added: bool,
    pub forbidden_command_acceptance_added: bool,
    pub live_desktop_command_wiring_added: bool,
    pub lifecycle_runtime_added: bool,
    pub unlock_runtime_added: bool,
    pub proof_capability_username_runtime_added: bool,
    pub platform_sealer_unseal_added: bool,
    pub runtime_io_added: bool,
    pub storage_mutation_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub native_secret_implementation_added: bool,
    pub full_desktop_runtime_complete: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_client_command_surface_acceptance_posture(
) -> NativeClientCommandSurfaceAcceptancePosture {
    NativeClientCommandSurfaceAcceptancePosture {
        phase_label: NATIVE_PASSPORT_PHASE15D_LABEL,
        command_surface_acceptance_added: true,
        status_command_contract_reused: true,
        command_inventory_contract_reused: true,
        redacted_dto_adapter_reused: true,
        command_group_acceptance_added: true,
        redacted_react_dto_acceptance_added: true,
        forbidden_command_acceptance_added: true,
        live_desktop_command_wiring_added: false,
        lifecycle_runtime_added: false,
        unlock_runtime_added: false,
        proof_capability_username_runtime_added: false,
        platform_sealer_unseal_added: false,
        runtime_io_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        native_secret_implementation_added: false,
        full_desktop_runtime_complete: false,
        forbidden_authority_flags: PHASE15D_FORBIDDEN_COMMAND_SURFACE_FLAGS,
    }
}

pub fn review_native_client_command_surface_acceptance(
    draft: NativeClientCommandSurfaceAcceptanceDraftV1,
    status: &NativeClientStatusCommandDecisionV1,
    inventory: &NativeClientCommandInventoryDecisionV1,
    dto: &NativeClientRedactedCommandDtoEnvelopeV1,
) -> Result<
    NativeClientCommandSurfaceAcceptanceDecisionV1,
    NativeClientCommandSurfaceAcceptanceReviewError,
> {
    if draft.contract_domain != PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_VERSION {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::ContractVersionMismatch);
    }

    if draft.requester_label.is_empty() {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::MissingRequesterLabel);
    }

    if draft.surface_label.is_empty() {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::MissingSurfaceLabel);
    }

    if draft.target_label != PHASE15C_REDACTED_DTO_TARGET {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::TargetMismatch);
    }

    validate_draft(&draft)?;
    validate_postures()?;
    validate_status_decision(status)?;
    validate_inventory_decision(inventory)?;
    validate_redacted_dto(dto, status, inventory)?;

    Ok(NativeClientCommandSurfaceAcceptanceDecisionV1 {
        contract_domain: PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_DOMAIN,
        contract_version: PHASE15D_CLIENT_COMMAND_SURFACE_ACCEPTANCE_VERSION,
        requester_label: draft.requester_label,
        surface_label: draft.surface_label,
        target_label: draft.target_label,
        command_count: draft.expected_command_count,
        forbidden_command_count: draft.expected_forbidden_command_count,
        status_command_reviewed: true,
        command_inventory_reviewed: true,
        redacted_dto_adapter_reviewed: true,
        all_desktop_commands_allowlisted: true,
        forbidden_commands_denied: true,
        react_receives_only_redacted_dtos: true,
        status_command_available: true,
        lifecycle_command_contracts_available: true,
        unlock_command_contracts_available: true,
        device_command_contracts_available: true,
        proof_command_contracts_available: true,
        capability_command_contracts_available: true,
        username_command_contracts_available: true,
        live_desktop_command_wiring_added: false,
        lifecycle_runtime_added: false,
        unlock_runtime_added: false,
        proof_capability_username_runtime_added: false,
        arbitrary_scope_issued: false,
        policy_disabled: false,
        platform_sealer_unseal_performed: false,
        runtime_io_performed: false,
        storage_mutated: false,
        wallet_or_ledger_mutated: false,
        secret_material_exposed: false,
        capability_material_exposed: false,
        vault_material_exposed: false,
        command_surface_contract_complete: true,
        full_desktop_runtime_complete: false,
        contract_only: true,
    })
}

fn validate_draft(
    draft: &NativeClientCommandSurfaceAcceptanceDraftV1,
) -> Result<(), NativeClientCommandSurfaceAcceptanceReviewError> {
    if draft.expected_command_count != PHASE15B_EXPECTED_COMMAND_COUNT {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::CommandCountMismatch);
    }

    if draft.expected_forbidden_command_count != PHASE15B_EXPECTED_FORBIDDEN_COMMAND_COUNT {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::ForbiddenCommandCountMismatch);
    }

    if !draft.status_command_reviewed {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::StatusCommandNotReviewed);
    }

    if !draft.command_inventory_reviewed {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::CommandInventoryNotReviewed);
    }

    if !draft.redacted_dto_adapter_reviewed {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::RedactedDtoAdapterNotReviewed);
    }

    if !draft.all_desktop_commands_allowlisted {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::CommandAllowlistNotComplete);
    }

    if !draft.forbidden_commands_denied {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::ForbiddenCommandsNotDenied);
    }

    if !draft.react_receives_only_redacted_dtos {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::ReactDtoNotRedacted);
    }

    if !draft.status_command_available
        || !draft.lifecycle_command_contracts_available
        || !draft.unlock_command_contracts_available
        || !draft.device_command_contracts_available
        || !draft.proof_command_contracts_available
        || !draft.capability_command_contracts_available
        || !draft.username_command_contracts_available
    {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::CommandContractGroupMissing);
    }

    if draft.secret_material_exposure_requested
        || draft.capability_material_exposure_requested
        || draft.vault_material_exposure_requested
    {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::ExposureRequested);
    }

    if draft.live_desktop_command_wiring_requested {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::LiveDesktopWiringRequested);
    }

    validate_no_unsafe_flags(draft)
}

fn validate_postures() -> Result<(), NativeClientCommandSurfaceAcceptanceReviewError> {
    if native_client_status_command_posture().phase_label != NATIVE_PASSPORT_PHASE15A_LABEL {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::StatusPostureNotGreen);
    }

    if native_client_command_inventory_posture().phase_label != NATIVE_PASSPORT_PHASE15B_LABEL {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::InventoryPostureNotGreen);
    }

    if native_client_redacted_command_dto_adapter_posture().phase_label
        != NATIVE_PASSPORT_PHASE15C_LABEL
    {
        return Err(
            NativeClientCommandSurfaceAcceptanceReviewError::RedactedDtoAdapterPostureNotGreen,
        );
    }

    Ok(())
}

fn validate_status_decision(
    status: &NativeClientStatusCommandDecisionV1,
) -> Result<(), NativeClientCommandSurfaceAcceptanceReviewError> {
    if status.command_name != PHASE15A_PASSPORT_STATUS_COMMAND
        || !status.native_feature_enabled
        || !status.desktop_surface
        || !status.redacted_dto
        || !status.route_acceptance_green
        || !status.local_status_inspection_green
        || status.seed_phrase_exposed
        || status.private_key_exposed
        || status.raw_capability_exposed
        || status.vault_material_exposed
        || status.unlock_performed
        || status.root_confirmation_requested
        || status.platform_sealer_unseal_performed
        || status.runtime_io_performed
        || status.storage_mutated
        || status.wallet_or_ledger_mutated
        || status.arbitrary_scope_issued
        || status.policy_disabled
        || !status.contract_only
    {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::StatusDecisionUnsafe);
    }

    Ok(())
}

fn validate_inventory_decision(
    inventory: &NativeClientCommandInventoryDecisionV1,
) -> Result<(), NativeClientCommandSurfaceAcceptanceReviewError> {
    if inventory.contract_domain != PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN
        || inventory.contract_version != PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION
        || inventory.command_count != PHASE15B_EXPECTED_COMMAND_COUNT
        || inventory.forbidden_command_count != PHASE15B_EXPECTED_FORBIDDEN_COMMAND_COUNT
        || !inventory.status_command_contract_green
        || !inventory.expected_inventory_locked
        || !inventory.forbidden_inventory_locked
        || !inventory.redacted_react_dtos_required
        || !inventory.forbidden_commands_absent_from_allowlist
        || inventory.live_tauri_command_added
        || inventory.lifecycle_runtime_added
        || inventory.unlock_runtime_added
        || inventory.proof_capability_username_runtime_added
        || inventory.arbitrary_scope_issued
        || inventory.policy_disabled
        || inventory.platform_sealer_unseal_performed
        || inventory.runtime_io_performed
        || inventory.storage_mutated
        || inventory.wallet_or_ledger_mutated
        || inventory.secret_material_exposed
        || !inventory.contract_only
    {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::InventoryDecisionUnsafe);
    }

    Ok(())
}

fn validate_redacted_dto(
    dto: &NativeClientRedactedCommandDtoEnvelopeV1,
    status: &NativeClientStatusCommandDecisionV1,
    inventory: &NativeClientCommandInventoryDecisionV1,
) -> Result<(), NativeClientCommandSurfaceAcceptanceReviewError> {
    if dto.contract_domain != PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_DOMAIN
        || dto.contract_version != PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_VERSION
        || dto.command_name != PHASE15A_PASSPORT_STATUS_COMMAND
        || dto.target_label != PHASE15C_REDACTED_DTO_TARGET
        || dto.inventory_contract_domain != inventory.contract_domain
        || dto.inventory_contract_version != inventory.contract_version
        || dto.lock_state != status.lock_state
        || dto.capability_state != status.capability_state
        || dto.redacted_passport_identifier != status.redacted_passport_identifier
        || dto.redacted_device_identifier != status.redacted_device_identifier
        || dto.redacted_username_handle != status.redacted_username_handle
        || dto.redacted_capability_material != status.redacted_capability_material
        || !dto.redacted_react_dto
        || !dto.command_allowlisted
        || dto.forbidden_command
        || dto.live_tauri_command_added
        || dto.unlock_runtime_added
        || dto.platform_sealer_unseal_added
        || dto.runtime_io_added
        || dto.storage_mutation_added
        || dto.wallet_or_ledger_mutation_added
        || dto.secret_material_exposed
        || dto.capability_material_exposed
        || dto.vault_material_exposed
        || !dto.contract_only
    {
        return Err(NativeClientCommandSurfaceAcceptanceReviewError::RedactedDtoEnvelopeUnsafe);
    }

    Ok(())
}

fn validate_no_unsafe_flags(
    draft: &NativeClientCommandSurfaceAcceptanceDraftV1,
) -> Result<(), NativeClientCommandSurfaceAcceptanceReviewError> {
    if draft.lifecycle_runtime_requested
        || draft.unlock_runtime_requested
        || draft.proof_capability_username_runtime_requested
        || draft.arbitrary_scope_issue_requested
        || draft.policy_disable_requested
        || draft.platform_sealer_unseal_requested
        || draft.runtime_io_requested
        || draft.storage_mutation_requested
        || draft.wallet_or_ledger_mutation_requested
    {
        return Err(
            NativeClientCommandSurfaceAcceptanceReviewError::UnsafeDesktopCommandSurfaceAuthorityFlag,
        );
    }

    Ok(())
}
