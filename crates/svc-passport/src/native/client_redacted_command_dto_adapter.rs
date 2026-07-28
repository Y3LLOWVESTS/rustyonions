//! RO:WHAT — Native Passport desktop redacted command DTO adapter.
//! RO:WHY — Phase 15C adapts reviewed command decisions into safe React-facing DTO envelopes before live Tauri command wiring.
//! RO:INTERACTS — Phase 15A status command contract, Phase 15B command inventory and denylist, CrabLink desktop command callers, and React UI state.
//! RO:INVARIANTS — only allowlisted commands may adapt, the status DTO is redacted, forbidden commands are rejected, and the adapter never exposes recovery words, signing material, capability material, vault material, routes, runtime I/O, storage mutation, or wallet/ledger mutation.
//! RO:TEST — tests/native_passport_phase15c_desktop_redacted_command_dto_adapter.rs.

use super::{
    native_client_command_inventory_posture, native_client_status_command_posture,
    NativeClientCommandInventoryDecisionV1, NativeClientStatusCapabilityState,
    NativeClientStatusCommandDecisionV1, NativeClientStatusLockState,
    NATIVE_PASSPORT_PHASE15A_LABEL, NATIVE_PASSPORT_PHASE15B_LABEL,
    PHASE15A_PASSPORT_STATUS_COMMAND, PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN,
    PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION,
};

pub const NATIVE_PASSPORT_PHASE15C_LABEL: &str =
    "NATIVE_PASSPORT_PHASE15C_DESKTOP_REDACTED_COMMAND_DTO_ADAPTER";

pub const PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_DOMAIN: &str =
    "native-passport/desktop-redacted-command-dto-adapter/v1";

pub const PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_VERSION: u16 = 1;

pub const PHASE15C_REDACTED_DTO_TARGET: &str = "crablink-desktop-react";

pub const PHASE15C_FORBIDDEN_REDACTED_DTO_ADAPTER_FLAGS: &[&str] = &[
    "unreviewed_command_to_react",
    "forbidden_command_to_react",
    "unredacted_identifier_to_react",
    "secret_material_to_react",
    "capability_material_to_react",
    "vault_material_to_react",
    "live_tauri_command",
    "unlock_runtime",
    "platform_sealer_unseal",
    "runtime_io",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientRedactedCommandDtoAdapterDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub requester_label: &'static str,
    pub target_label: &'static str,
    pub command_name: &'static str,
    pub status_decision_reviewed: bool,
    pub inventory_decision_reviewed: bool,
    pub command_allowlisted: bool,
    pub command_forbidden: bool,
    pub redacted_react_dto_required: bool,
    pub exposes_identifier_material: bool,
    pub exposes_secret_material: bool,
    pub exposes_capability_material: bool,
    pub exposes_vault_material: bool,
    pub requests_live_tauri_command: bool,
    pub requests_unlock_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_runtime_io: bool,
    pub requests_storage_mutation: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientRedactedCommandDtoEnvelopeV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub command_name: &'static str,
    pub target_label: &'static str,
    pub requester_label: &'static str,
    pub inventory_contract_domain: &'static str,
    pub inventory_contract_version: u16,
    pub status_command_phase_label: &'static str,
    pub inventory_phase_label: &'static str,
    pub lock_state: NativeClientStatusLockState,
    pub capability_state: NativeClientStatusCapabilityState,
    pub redacted_passport_identifier: &'static str,
    pub redacted_device_identifier: &'static str,
    pub redacted_username_handle: &'static str,
    pub redacted_capability_material: &'static str,
    pub redacted_react_dto: bool,
    pub command_allowlisted: bool,
    pub forbidden_command: bool,
    pub live_tauri_command_added: bool,
    pub unlock_runtime_added: bool,
    pub platform_sealer_unseal_added: bool,
    pub runtime_io_added: bool,
    pub storage_mutation_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub secret_material_exposed: bool,
    pub capability_material_exposed: bool,
    pub vault_material_exposed: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeClientRedactedCommandDtoAdapterReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    MissingRequesterLabel,
    TargetMismatch,
    CommandNameMismatch,
    StatusDecisionNotReviewed,
    InventoryDecisionNotReviewed,
    CommandNotAllowlisted,
    ForbiddenCommandRequested,
    RedactedReactDtoNotRequired,
    StatusPostureNotGreen,
    InventoryPostureNotGreen,
    StatusDecisionUnsafe,
    InventoryDecisionUnsafe,
    InventoryDecisionContractMismatch,
    IdentifierMaterialExposure,
    SecretMaterialExposure,
    CapabilityMaterialExposure,
    VaultMaterialExposure,
    UnsafeAdapterAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientRedactedCommandDtoAdapterPosture {
    pub phase_label: &'static str,
    pub redacted_command_dto_adapter_added: bool,
    pub status_command_contract_reused: bool,
    pub command_inventory_contract_reused: bool,
    pub react_dto_target_locked: bool,
    pub forbidden_command_rejection_added: bool,
    pub unredacted_material_rejection_added: bool,
    pub live_tauri_command_added: bool,
    pub unlock_runtime_added: bool,
    pub platform_sealer_unseal_added: bool,
    pub runtime_io_added: bool,
    pub storage_mutation_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_client_redacted_command_dto_adapter_posture(
) -> NativeClientRedactedCommandDtoAdapterPosture {
    NativeClientRedactedCommandDtoAdapterPosture {
        phase_label: NATIVE_PASSPORT_PHASE15C_LABEL,
        redacted_command_dto_adapter_added: true,
        status_command_contract_reused: true,
        command_inventory_contract_reused: true,
        react_dto_target_locked: true,
        forbidden_command_rejection_added: true,
        unredacted_material_rejection_added: true,
        live_tauri_command_added: false,
        unlock_runtime_added: false,
        platform_sealer_unseal_added: false,
        runtime_io_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE15C_FORBIDDEN_REDACTED_DTO_ADAPTER_FLAGS,
    }
}

pub fn adapt_native_client_status_to_redacted_command_dto(
    draft: NativeClientRedactedCommandDtoAdapterDraftV1,
    status: &NativeClientStatusCommandDecisionV1,
    inventory: &NativeClientCommandInventoryDecisionV1,
) -> Result<
    NativeClientRedactedCommandDtoEnvelopeV1,
    NativeClientRedactedCommandDtoAdapterReviewError,
> {
    if draft.contract_domain != PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_DOMAIN {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_VERSION {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::ContractVersionMismatch);
    }

    if draft.requester_label.is_empty() {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::MissingRequesterLabel);
    }

    if draft.target_label != PHASE15C_REDACTED_DTO_TARGET {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::TargetMismatch);
    }

    if draft.command_name != PHASE15A_PASSPORT_STATUS_COMMAND
        || status.command_name != PHASE15A_PASSPORT_STATUS_COMMAND
    {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::CommandNameMismatch);
    }

    if !draft.status_decision_reviewed {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::StatusDecisionNotReviewed);
    }

    if !draft.inventory_decision_reviewed {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::InventoryDecisionNotReviewed);
    }

    if !draft.command_allowlisted {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::CommandNotAllowlisted);
    }

    if draft.command_forbidden {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::ForbiddenCommandRequested);
    }

    if !draft.redacted_react_dto_required {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::RedactedReactDtoNotRequired);
    }

    let status_posture = native_client_status_command_posture();
    if status_posture.phase_label != NATIVE_PASSPORT_PHASE15A_LABEL {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::StatusPostureNotGreen);
    }

    let inventory_posture = native_client_command_inventory_posture();
    if inventory_posture.phase_label != NATIVE_PASSPORT_PHASE15B_LABEL {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::InventoryPostureNotGreen);
    }

    validate_status_decision(status)?;
    validate_inventory_decision(inventory)?;
    validate_no_exposure_flags(&draft)?;
    validate_no_unsafe_flags(&draft)?;

    Ok(NativeClientRedactedCommandDtoEnvelopeV1 {
        contract_domain: PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_DOMAIN,
        contract_version: PHASE15C_REDACTED_COMMAND_DTO_ADAPTER_VERSION,
        command_name: PHASE15A_PASSPORT_STATUS_COMMAND,
        target_label: PHASE15C_REDACTED_DTO_TARGET,
        requester_label: draft.requester_label,
        inventory_contract_domain: inventory.contract_domain,
        inventory_contract_version: inventory.contract_version,
        status_command_phase_label: NATIVE_PASSPORT_PHASE15A_LABEL,
        inventory_phase_label: NATIVE_PASSPORT_PHASE15B_LABEL,
        lock_state: status.lock_state,
        capability_state: status.capability_state,
        redacted_passport_identifier: status.redacted_passport_identifier,
        redacted_device_identifier: status.redacted_device_identifier,
        redacted_username_handle: status.redacted_username_handle,
        redacted_capability_material: status.redacted_capability_material,
        redacted_react_dto: true,
        command_allowlisted: true,
        forbidden_command: false,
        live_tauri_command_added: false,
        unlock_runtime_added: false,
        platform_sealer_unseal_added: false,
        runtime_io_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        secret_material_exposed: false,
        capability_material_exposed: false,
        vault_material_exposed: false,
        contract_only: true,
    })
}

fn validate_status_decision(
    status: &NativeClientStatusCommandDecisionV1,
) -> Result<(), NativeClientRedactedCommandDtoAdapterReviewError> {
    if !status.native_feature_enabled
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
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::StatusDecisionUnsafe);
    }

    Ok(())
}

fn validate_inventory_decision(
    inventory: &NativeClientCommandInventoryDecisionV1,
) -> Result<(), NativeClientRedactedCommandDtoAdapterReviewError> {
    if inventory.contract_domain != PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN
        || inventory.contract_version != PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION
    {
        return Err(
            NativeClientRedactedCommandDtoAdapterReviewError::InventoryDecisionContractMismatch,
        );
    }

    if !inventory.status_command_contract_green
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
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::InventoryDecisionUnsafe);
    }

    Ok(())
}

fn validate_no_exposure_flags(
    draft: &NativeClientRedactedCommandDtoAdapterDraftV1,
) -> Result<(), NativeClientRedactedCommandDtoAdapterReviewError> {
    if draft.exposes_identifier_material {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::IdentifierMaterialExposure);
    }

    if draft.exposes_secret_material {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::SecretMaterialExposure);
    }

    if draft.exposes_capability_material {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::CapabilityMaterialExposure);
    }

    if draft.exposes_vault_material {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::VaultMaterialExposure);
    }

    Ok(())
}

fn validate_no_unsafe_flags(
    draft: &NativeClientRedactedCommandDtoAdapterDraftV1,
) -> Result<(), NativeClientRedactedCommandDtoAdapterReviewError> {
    if draft.requests_live_tauri_command
        || draft.requests_unlock_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_runtime_io
        || draft.requests_storage_mutation
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeClientRedactedCommandDtoAdapterReviewError::UnsafeAdapterAuthorityFlag);
    }

    Ok(())
}
