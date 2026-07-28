//! RO:WHAT — Native Passport desktop command inventory and denylist contract.
//! RO:WHY — Phase 15B freezes the command names CrabLink desktop may eventually wire before any live Tauri command implementation is added.
//! RO:INTERACTS — Phase 15A status command contract, desktop command allowlist, desktop command denylist, and future CrabLink React command callers.
//! RO:INVARIANTS — only allowlisted command names may be surfaced; explicitly forbidden command names are rejected; responses must be redacted for React; this phase adds no live Tauri command, route mount, unlock runtime, storage mutation, wallet/ledger mutation, or secret/key access.
//! RO:TEST — tests/native_passport_phase15b_desktop_command_inventory_and_denylist.rs.

use std::collections::HashSet;

use super::{
    native_client_status_command_posture, NATIVE_PASSPORT_PHASE15A_LABEL,
    PHASE15A_EXPECTED_DESKTOP_COMMANDS, PHASE15A_FORBIDDEN_DESKTOP_COMMANDS,
    PHASE15A_PASSPORT_STATUS_COMMAND,
};

pub const NATIVE_PASSPORT_PHASE15B_LABEL: &str =
    "NATIVE_PASSPORT_PHASE15B_DESKTOP_COMMAND_INVENTORY_AND_DENYLIST";

pub const PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN: &str =
    "native-passport/client-command-inventory/v1";

pub const PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION: u16 = 1;

pub const PHASE15B_EXPECTED_COMMAND_COUNT: usize = 18;
pub const PHASE15B_EXPECTED_FORBIDDEN_COMMAND_COUNT: usize = 6;

pub const PHASE15B_FORBIDDEN_CLIENT_COMMAND_AUTHORITY_FLAGS: &[&str] = &[
    "seed_to_webview",
    "private_key_export",
    "device_key_export",
    "raw_capability_export",
    "arbitrary_scope_issue",
    "policy_disable",
    "live_tauri_command",
    "unlock_runtime",
    "platform_sealer_unseal",
    "runtime_io",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeClientCommandCategory {
    Status,
    Lifecycle,
    Unlock,
    Device,
    Proof,
    Capability,
    Username,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeClientCommandRisk {
    ReadOnlyRedacted,
    RequiresOperationalUnlock,
    RequiresRootConfirmation,
    RequiresFreshProof,
    Prohibited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeClientCommandInventoryEntryV1 {
    pub command_name: &'static str,
    pub category: NativeClientCommandCategory,
    pub risk: NativeClientCommandRisk,
    pub redacted_response_required: bool,
    pub secret_response_allowed: bool,
    pub live_tauri_wiring_allowed_now: bool,
    pub mutating_behavior_allowed_now: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeClientForbiddenCommandEntryV1 {
    pub command_name: &'static str,
    pub risk: NativeClientCommandRisk,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientCommandInventoryDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub requester_label: &'static str,
    pub surface_label: &'static str,
    pub native_feature_enabled: bool,
    pub desktop_surface: bool,
    pub status_command_contract_green: bool,
    pub command_inventory_count: usize,
    pub forbidden_command_count: usize,
    pub expected_inventory_included: bool,
    pub forbidden_inventory_included: bool,
    pub redacted_react_dtos_required: bool,
    pub secrets_to_react_allowed: bool,
    pub live_tauri_wiring_requested: bool,
    pub lifecycle_runtime_requested: bool,
    pub unlock_runtime_requested: bool,
    pub proof_capability_username_runtime_requested: bool,
    pub arbitrary_scope_issue_requested: bool,
    pub policy_disable_requested: bool,
    pub platform_sealer_unseal_requested: bool,
    pub runtime_io_requested: bool,
    pub storage_mutation_requested: bool,
    pub wallet_or_ledger_mutation_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientCommandInventoryDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub command_count: usize,
    pub forbidden_command_count: usize,
    pub status_command_contract_green: bool,
    pub expected_inventory_locked: bool,
    pub forbidden_inventory_locked: bool,
    pub redacted_react_dtos_required: bool,
    pub forbidden_commands_absent_from_allowlist: bool,
    pub live_tauri_command_added: bool,
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
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeClientCommandInventoryReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    MissingRequesterLabel,
    MissingSurfaceLabel,
    NativeFeatureNotEnabled,
    NotDesktopSurface,
    StatusCommandContractNotGreen,
    CommandCountMismatch,
    ForbiddenCommandCountMismatch,
    ExpectedInventoryMissing,
    ForbiddenInventoryMissing,
    RedactedReactDtosNotRequired,
    SecretsToReactAllowed,
    EmptyInventory,
    EmptyForbiddenInventory,
    DuplicateCommand,
    DuplicateForbiddenCommand,
    UnknownCommand,
    MissingExpectedCommand,
    ForbiddenCommandAllowed,
    ForbiddenCommandMissing,
    CommandShapeMismatch,
    ForbiddenCommandShapeMismatch,
    LiveTauriWiringRequested,
    UnsafeClientCommandAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeClientCommandInventoryPosture {
    pub phase_label: &'static str,
    pub command_inventory_contract_added: bool,
    pub expected_command_inventory_locked: bool,
    pub forbidden_command_inventory_locked: bool,
    pub redacted_react_dto_policy_added: bool,
    pub status_command_contract_reused: bool,
    pub live_tauri_command_added: bool,
    pub lifecycle_runtime_added: bool,
    pub unlock_runtime_added: bool,
    pub proof_capability_username_runtime_added: bool,
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

pub const PHASE15B_CLIENT_COMMAND_INVENTORY: &[NativeClientCommandInventoryEntryV1] = &[
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_status",
        category: NativeClientCommandCategory::Status,
        risk: NativeClientCommandRisk::ReadOnlyRedacted,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_create_native",
        category: NativeClientCommandCategory::Lifecycle,
        risk: NativeClientCommandRisk::RequiresRootConfirmation,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_restore_native",
        category: NativeClientCommandCategory::Lifecycle,
        risk: NativeClientCommandRisk::RequiresRootConfirmation,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_unlock_operational",
        category: NativeClientCommandCategory::Unlock,
        risk: NativeClientCommandRisk::RequiresOperationalUnlock,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_unlock_root",
        category: NativeClientCommandCategory::Unlock,
        risk: NativeClientCommandRisk::RequiresRootConfirmation,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_lock",
        category: NativeClientCommandCategory::Unlock,
        risk: NativeClientCommandRisk::ReadOnlyRedacted,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_clear",
        category: NativeClientCommandCategory::Lifecycle,
        risk: NativeClientCommandRisk::RequiresRootConfirmation,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_device_list",
        category: NativeClientCommandCategory::Device,
        risk: NativeClientCommandRisk::ReadOnlyRedacted,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_device_authorize",
        category: NativeClientCommandCategory::Device,
        risk: NativeClientCommandRisk::RequiresFreshProof,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_device_revoke",
        category: NativeClientCommandCategory::Device,
        risk: NativeClientCommandRisk::RequiresFreshProof,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_prove",
        category: NativeClientCommandCategory::Proof,
        risk: NativeClientCommandRisk::RequiresFreshProof,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_capability_status",
        category: NativeClientCommandCategory::Capability,
        risk: NativeClientCommandRisk::ReadOnlyRedacted,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_capability_refresh",
        category: NativeClientCommandCategory::Capability,
        risk: NativeClientCommandRisk::RequiresFreshProof,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_capability_revoke",
        category: NativeClientCommandCategory::Capability,
        risk: NativeClientCommandRisk::RequiresFreshProof,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_username_status",
        category: NativeClientCommandCategory::Username,
        risk: NativeClientCommandRisk::ReadOnlyRedacted,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_username_claim",
        category: NativeClientCommandCategory::Username,
        risk: NativeClientCommandRisk::RequiresFreshProof,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_username_transfer",
        category: NativeClientCommandCategory::Username,
        risk: NativeClientCommandRisk::RequiresFreshProof,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
    NativeClientCommandInventoryEntryV1 {
        command_name: "passport_username_release",
        category: NativeClientCommandCategory::Username,
        risk: NativeClientCommandRisk::RequiresFreshProof,
        redacted_response_required: true,
        secret_response_allowed: false,
        live_tauri_wiring_allowed_now: false,
        mutating_behavior_allowed_now: false,
    },
];

pub const PHASE15B_CLIENT_COMMAND_DENYLIST: &[NativeClientForbiddenCommandEntryV1] = &[
    NativeClientForbiddenCommandEntryV1 {
        command_name: "passport_get_seed_to_webview",
        risk: NativeClientCommandRisk::Prohibited,
        reason: "React must never receive recovery material.",
    },
    NativeClientForbiddenCommandEntryV1 {
        command_name: "passport_export_private_key",
        risk: NativeClientCommandRisk::Prohibited,
        reason: "Private signing material must not leave native custody.",
    },
    NativeClientForbiddenCommandEntryV1 {
        command_name: "passport_get_device_private_key",
        risk: NativeClientCommandRisk::Prohibited,
        reason: "Device signing material must not be exported.",
    },
    NativeClientForbiddenCommandEntryV1 {
        command_name: "passport_get_raw_capability",
        risk: NativeClientCommandRisk::Prohibited,
        reason: "Capability material must only be represented as redacted status.",
    },
    NativeClientForbiddenCommandEntryV1 {
        command_name: "passport_issue_arbitrary_scope",
        risk: NativeClientCommandRisk::Prohibited,
        reason: "Scope issuance must remain policy-bound.",
    },
    NativeClientForbiddenCommandEntryV1 {
        command_name: "passport_disable_policy",
        risk: NativeClientCommandRisk::Prohibited,
        reason: "Client commands must not bypass policy.",
    },
];

pub fn native_client_command_inventory_posture() -> NativeClientCommandInventoryPosture {
    NativeClientCommandInventoryPosture {
        phase_label: NATIVE_PASSPORT_PHASE15B_LABEL,
        command_inventory_contract_added: true,
        expected_command_inventory_locked: true,
        forbidden_command_inventory_locked: true,
        redacted_react_dto_policy_added: true,
        status_command_contract_reused: true,
        live_tauri_command_added: false,
        lifecycle_runtime_added: false,
        unlock_runtime_added: false,
        proof_capability_username_runtime_added: false,
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
        forbidden_authority_flags: PHASE15B_FORBIDDEN_CLIENT_COMMAND_AUTHORITY_FLAGS,
    }
}

pub fn review_native_client_command_inventory(
    draft: NativeClientCommandInventoryDraftV1,
    inventory: &[NativeClientCommandInventoryEntryV1],
    denylist: &[NativeClientForbiddenCommandEntryV1],
) -> Result<NativeClientCommandInventoryDecisionV1, NativeClientCommandInventoryReviewError> {
    if draft.contract_domain != PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN {
        return Err(NativeClientCommandInventoryReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION {
        return Err(NativeClientCommandInventoryReviewError::ContractVersionMismatch);
    }

    if draft.requester_label.is_empty() {
        return Err(NativeClientCommandInventoryReviewError::MissingRequesterLabel);
    }

    if draft.surface_label.is_empty() {
        return Err(NativeClientCommandInventoryReviewError::MissingSurfaceLabel);
    }

    if !draft.native_feature_enabled {
        return Err(NativeClientCommandInventoryReviewError::NativeFeatureNotEnabled);
    }

    if !draft.desktop_surface {
        return Err(NativeClientCommandInventoryReviewError::NotDesktopSurface);
    }

    let status_posture = native_client_status_command_posture();
    if !draft.status_command_contract_green
        || status_posture.phase_label != NATIVE_PASSPORT_PHASE15A_LABEL
    {
        return Err(NativeClientCommandInventoryReviewError::StatusCommandContractNotGreen);
    }

    if draft.command_inventory_count != PHASE15B_EXPECTED_COMMAND_COUNT {
        return Err(NativeClientCommandInventoryReviewError::CommandCountMismatch);
    }

    if draft.forbidden_command_count != PHASE15B_EXPECTED_FORBIDDEN_COMMAND_COUNT {
        return Err(NativeClientCommandInventoryReviewError::ForbiddenCommandCountMismatch);
    }

    if !draft.expected_inventory_included {
        return Err(NativeClientCommandInventoryReviewError::ExpectedInventoryMissing);
    }

    if !draft.forbidden_inventory_included {
        return Err(NativeClientCommandInventoryReviewError::ForbiddenInventoryMissing);
    }

    if !draft.redacted_react_dtos_required {
        return Err(NativeClientCommandInventoryReviewError::RedactedReactDtosNotRequired);
    }

    if draft.secrets_to_react_allowed {
        return Err(NativeClientCommandInventoryReviewError::SecretsToReactAllowed);
    }

    if draft.live_tauri_wiring_requested {
        return Err(NativeClientCommandInventoryReviewError::LiveTauriWiringRequested);
    }

    validate_no_unsafe_flags(&draft)?;
    validate_inventory(inventory, denylist)?;

    Ok(NativeClientCommandInventoryDecisionV1 {
        contract_domain: PHASE15B_CLIENT_COMMAND_INVENTORY_DOMAIN,
        contract_version: PHASE15B_CLIENT_COMMAND_INVENTORY_VERSION,
        command_count: inventory.len(),
        forbidden_command_count: denylist.len(),
        status_command_contract_green: true,
        expected_inventory_locked: true,
        forbidden_inventory_locked: true,
        redacted_react_dtos_required: true,
        forbidden_commands_absent_from_allowlist: true,
        live_tauri_command_added: false,
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
        contract_only: true,
    })
}

fn validate_inventory(
    inventory: &[NativeClientCommandInventoryEntryV1],
    denylist: &[NativeClientForbiddenCommandEntryV1],
) -> Result<(), NativeClientCommandInventoryReviewError> {
    if inventory.is_empty() {
        return Err(NativeClientCommandInventoryReviewError::EmptyInventory);
    }

    if denylist.is_empty() {
        return Err(NativeClientCommandInventoryReviewError::EmptyForbiddenInventory);
    }

    if inventory.len() != PHASE15B_EXPECTED_COMMAND_COUNT {
        return Err(NativeClientCommandInventoryReviewError::CommandCountMismatch);
    }

    if denylist.len() != PHASE15B_EXPECTED_FORBIDDEN_COMMAND_COUNT {
        return Err(NativeClientCommandInventoryReviewError::ForbiddenCommandCountMismatch);
    }

    let mut seen_inventory = HashSet::new();
    for entry in inventory {
        if !seen_inventory.insert(entry.command_name) {
            return Err(NativeClientCommandInventoryReviewError::DuplicateCommand);
        }

        if !PHASE15A_EXPECTED_DESKTOP_COMMANDS.contains(&entry.command_name) {
            return Err(NativeClientCommandInventoryReviewError::UnknownCommand);
        }

        let expected = expected_inventory_entry(entry.command_name)
            .ok_or(NativeClientCommandInventoryReviewError::UnknownCommand)?;

        if entry != expected {
            return Err(NativeClientCommandInventoryReviewError::CommandShapeMismatch);
        }

        if !entry.redacted_response_required
            || entry.secret_response_allowed
            || entry.live_tauri_wiring_allowed_now
            || entry.mutating_behavior_allowed_now
        {
            return Err(NativeClientCommandInventoryReviewError::CommandShapeMismatch);
        }
    }

    for expected in PHASE15A_EXPECTED_DESKTOP_COMMANDS {
        if !seen_inventory.contains(expected) {
            return Err(NativeClientCommandInventoryReviewError::MissingExpectedCommand);
        }
    }

    let mut seen_forbidden = HashSet::new();
    for entry in denylist {
        if !seen_forbidden.insert(entry.command_name) {
            return Err(NativeClientCommandInventoryReviewError::DuplicateForbiddenCommand);
        }

        if !PHASE15A_FORBIDDEN_DESKTOP_COMMANDS.contains(&entry.command_name) {
            return Err(NativeClientCommandInventoryReviewError::ForbiddenCommandShapeMismatch);
        }

        if entry.risk != NativeClientCommandRisk::Prohibited || entry.reason.trim().is_empty() {
            return Err(NativeClientCommandInventoryReviewError::ForbiddenCommandShapeMismatch);
        }

        if seen_inventory.contains(entry.command_name) {
            return Err(NativeClientCommandInventoryReviewError::ForbiddenCommandAllowed);
        }
    }

    for forbidden in PHASE15A_FORBIDDEN_DESKTOP_COMMANDS {
        if !seen_forbidden.contains(forbidden) {
            return Err(NativeClientCommandInventoryReviewError::ForbiddenCommandMissing);
        }
    }

    Ok(())
}

fn expected_inventory_entry(
    command_name: &str,
) -> Option<&'static NativeClientCommandInventoryEntryV1> {
    PHASE15B_CLIENT_COMMAND_INVENTORY
        .iter()
        .find(|entry| entry.command_name == command_name)
}

fn validate_no_unsafe_flags(
    draft: &NativeClientCommandInventoryDraftV1,
) -> Result<(), NativeClientCommandInventoryReviewError> {
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
        return Err(NativeClientCommandInventoryReviewError::UnsafeClientCommandAuthorityFlag);
    }

    Ok(())
}

pub fn native_client_command_inventory_command_names() -> &'static [&'static str] {
    PHASE15A_EXPECTED_DESKTOP_COMMANDS
}

pub fn native_client_command_inventory_forbidden_command_names() -> &'static [&'static str] {
    PHASE15A_FORBIDDEN_DESKTOP_COMMANDS
}

pub fn native_client_command_inventory_status_command_name() -> &'static str {
    PHASE15A_PASSPORT_STATUS_COMMAND
}
