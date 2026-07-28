//! RO:WHAT — Native Passport local status and inspection snapshot.
//! RO:WHY — P3 Identity & Keys + local operator UX. Gives a safe, redacted, local-only view of native Passport posture after the username/private-beta phases are parked.
//! RO:INTERACTS — native feature posture, redacted status posture, username/capability/proof/replay surface labels, future local CLI/operator inspection.
//! RO:INVARIANTS — inspection is local-only and redacted. It does not expose Passport identifiers, Device identifiers, usernames, proof material, capability material, vault material, private keys, routes, runtime I/O, storage mutation, wallet/ledger mutation, or index authority.
//! RO:TEST — tests/native_passport_phase13_local_status_and_inspection.rs.

pub const NATIVE_PASSPORT_PHASE13_LABEL: &str =
    "NATIVE_PASSPORT_PHASE13_LOCAL_STATUS_AND_INSPECTION";

pub const PHASE13_LOCAL_STATUS_INSPECTION_DOMAIN: &str =
    "native-passport/local-status-inspection/v1";

pub const PHASE13_LOCAL_STATUS_INSPECTION_VERSION: u16 = 1;

pub const PHASE13_REDACTED_VALUE: &str = "REDACTED";

pub const PHASE13_FORBIDDEN_LOCAL_STATUS_AUTHORITY_FLAGS: &[&str] = &[
    "passport_identifier_exposure",
    "device_identifier_exposure",
    "username_handle_exposure",
    "proof_material_exposure",
    "capability_material_exposure",
    "vault_material_exposure",
    "secret_key_access",
    "key_loading",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_unseal",
    "runtime_io",
    "route_added",
    "storage_mutation_inside_svc_passport_native",
    "index_projection_authority",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLocalStatusInspectionDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub requester_label: &'static str,
    pub local_node_label: &'static str,
    pub status_requested_at_ms: u64,
    pub enabled_surface_count: usize,
    pub expected_enabled_surface_count: usize,
    pub latest_completed_phase_label: &'static str,
    pub native_feature_enabled: bool,
    pub local_only: bool,
    pub inspection_only: bool,
    pub redaction_required: bool,
    pub exposes_passport_identifier: bool,
    pub exposes_device_identifier: bool,
    pub exposes_username_handle: bool,
    pub exposes_proof_material: bool,
    pub exposes_capability_material: bool,
    pub exposes_vault_material: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_loading: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_runtime_io: bool,
    pub adds_routes: bool,
    pub requests_storage_mutation_inside_native: bool,
    pub treats_index_projection_as_authority: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLocalStatusInspectionSnapshotV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub phase_label: &'static str,
    pub requester_label: &'static str,
    pub local_node_label: &'static str,
    pub status_requested_at_ms: u64,
    pub enabled_surface_count: usize,
    pub latest_completed_phase_label: &'static str,
    pub native_feature_enabled: bool,
    pub local_only: bool,
    pub inspection_only: bool,
    pub redacted_passport_identifier: &'static str,
    pub redacted_device_identifier: &'static str,
    pub redacted_username_handle: &'static str,
    pub redacted_proof_material: &'static str,
    pub redacted_capability_material: &'static str,
    pub redacted_vault_material: &'static str,
    pub routes_added: bool,
    pub runtime_io_performed: bool,
    pub storage_mutated_inside_native: bool,
    pub index_projection_authoritative: bool,
    pub wallet_or_ledger_mutated: bool,
    pub secret_material_exposed: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeLocalStatusInspectionReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    MissingRequesterLabel,
    MissingLocalNodeLabel,
    MissingPhaseLabel,
    NativeFeatureNotEnabled,
    NotLocalOnly,
    NotInspectionOnly,
    RedactionNotRequired,
    SurfaceCountMismatch,
    IdentifierExposure,
    UsernameHandleExposure,
    ProofMaterialExposure,
    CapabilityMaterialExposure,
    VaultMaterialExposure,
    UnsafeLocalStatusAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLocalStatusInspectionPosture {
    pub phase_label: &'static str,
    pub local_status_inspection_added: bool,
    pub redacted_snapshot_added: bool,
    pub local_only_review_added: bool,
    pub route_added: bool,
    pub runtime_io_added: bool,
    pub storage_mutation_inside_native_added: bool,
    pub index_projection_authority_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub secret_key_access_added: bool,
    pub key_loading_added: bool,
    pub key_derivation_runtime_added: bool,
    pub vault_unlock_added: bool,
    pub vault_runtime_added: bool,
    pub platform_sealer_unseal_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_local_status_inspection_posture() -> NativeLocalStatusInspectionPosture {
    NativeLocalStatusInspectionPosture {
        phase_label: NATIVE_PASSPORT_PHASE13_LABEL,
        local_status_inspection_added: true,
        redacted_snapshot_added: true,
        local_only_review_added: true,
        route_added: false,
        runtime_io_added: false,
        storage_mutation_inside_native_added: false,
        index_projection_authority_added: false,
        wallet_or_ledger_mutation_added: false,
        secret_key_access_added: false,
        key_loading_added: false,
        key_derivation_runtime_added: false,
        vault_unlock_added: false,
        vault_runtime_added: false,
        platform_sealer_unseal_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE13_FORBIDDEN_LOCAL_STATUS_AUTHORITY_FLAGS,
    }
}

pub fn review_native_local_status_inspection(
    draft: NativeLocalStatusInspectionDraftV1,
) -> Result<NativeLocalStatusInspectionSnapshotV1, NativeLocalStatusInspectionReviewError> {
    if draft.contract_domain != PHASE13_LOCAL_STATUS_INSPECTION_DOMAIN {
        return Err(NativeLocalStatusInspectionReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE13_LOCAL_STATUS_INSPECTION_VERSION {
        return Err(NativeLocalStatusInspectionReviewError::ContractVersionMismatch);
    }

    if draft.requester_label.is_empty() {
        return Err(NativeLocalStatusInspectionReviewError::MissingRequesterLabel);
    }

    if draft.local_node_label.is_empty() {
        return Err(NativeLocalStatusInspectionReviewError::MissingLocalNodeLabel);
    }

    if draft.latest_completed_phase_label.is_empty() {
        return Err(NativeLocalStatusInspectionReviewError::MissingPhaseLabel);
    }

    if !draft.native_feature_enabled {
        return Err(NativeLocalStatusInspectionReviewError::NativeFeatureNotEnabled);
    }

    if !draft.local_only {
        return Err(NativeLocalStatusInspectionReviewError::NotLocalOnly);
    }

    if !draft.inspection_only {
        return Err(NativeLocalStatusInspectionReviewError::NotInspectionOnly);
    }

    if !draft.redaction_required {
        return Err(NativeLocalStatusInspectionReviewError::RedactionNotRequired);
    }

    if draft.enabled_surface_count != draft.expected_enabled_surface_count {
        return Err(NativeLocalStatusInspectionReviewError::SurfaceCountMismatch);
    }

    if draft.exposes_passport_identifier || draft.exposes_device_identifier {
        return Err(NativeLocalStatusInspectionReviewError::IdentifierExposure);
    }

    if draft.exposes_username_handle {
        return Err(NativeLocalStatusInspectionReviewError::UsernameHandleExposure);
    }

    if draft.exposes_proof_material {
        return Err(NativeLocalStatusInspectionReviewError::ProofMaterialExposure);
    }

    if draft.exposes_capability_material {
        return Err(NativeLocalStatusInspectionReviewError::CapabilityMaterialExposure);
    }

    if draft.exposes_vault_material {
        return Err(NativeLocalStatusInspectionReviewError::VaultMaterialExposure);
    }

    validate_no_unsafe_flags(&draft)?;

    Ok(NativeLocalStatusInspectionSnapshotV1 {
        contract_domain: PHASE13_LOCAL_STATUS_INSPECTION_DOMAIN,
        contract_version: PHASE13_LOCAL_STATUS_INSPECTION_VERSION,
        phase_label: NATIVE_PASSPORT_PHASE13_LABEL,
        requester_label: draft.requester_label,
        local_node_label: draft.local_node_label,
        status_requested_at_ms: draft.status_requested_at_ms,
        enabled_surface_count: draft.enabled_surface_count,
        latest_completed_phase_label: draft.latest_completed_phase_label,
        native_feature_enabled: true,
        local_only: true,
        inspection_only: true,
        redacted_passport_identifier: PHASE13_REDACTED_VALUE,
        redacted_device_identifier: PHASE13_REDACTED_VALUE,
        redacted_username_handle: PHASE13_REDACTED_VALUE,
        redacted_proof_material: PHASE13_REDACTED_VALUE,
        redacted_capability_material: PHASE13_REDACTED_VALUE,
        redacted_vault_material: PHASE13_REDACTED_VALUE,
        routes_added: false,
        runtime_io_performed: false,
        storage_mutated_inside_native: false,
        index_projection_authoritative: false,
        wallet_or_ledger_mutated: false,
        secret_material_exposed: false,
        contract_only: true,
    })
}

fn validate_no_unsafe_flags(
    draft: &NativeLocalStatusInspectionDraftV1,
) -> Result<(), NativeLocalStatusInspectionReviewError> {
    if draft.requests_secret_key_access
        || draft.requests_key_loading
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation_inside_native
        || draft.treats_index_projection_as_authority
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeLocalStatusInspectionReviewError::UnsafeLocalStatusAuthorityFlag);
    }

    Ok(())
}
