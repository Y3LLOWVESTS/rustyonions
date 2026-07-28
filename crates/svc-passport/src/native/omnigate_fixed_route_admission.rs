//! RO:WHAT — Native Passport Omnigate fixed-route admission review.
//! RO:WHY — Phase 14B adds the admission layer above the Phase 14A fixed route catalog before live gateway/Omnigate mounts are introduced.
//! RO:INTERACTS — Phase 14A fixed route catalog, typed redacted problem posture, gateway proxy admission, Omnigate orchestration admission, and downstream svc-passport authority.
//! RO:INVARIANTS — admission only accepts fixed catalog routes, caps body size/deadline, requires correlation IDs, preserves required proof/capability posture, blocks mutating scope escalation, keeps gateway proxy-only, keeps Omnigate orchestration-only, and does not add live mounts, storage mutation, wallet/ledger mutation, or secret/key access.
//! RO:TEST — tests/native_passport_phase14b_omnigate_fixed_route_admission.rs.

use super::{
    review_native_gateway_omnigate_route_catalog, NativeGatewayOmnigateRouteKind,
    NativeGatewayOmnigateRouteMethod, NativeGatewayOmnigateRouteReviewError,
    NativeGatewayOmnigateRouteSpecV1, PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG,
    PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN,
    PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION,
};

pub const NATIVE_PASSPORT_PHASE14B_LABEL: &str =
    "NATIVE_PASSPORT_PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION";

pub const PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN: &str =
    "native-passport/omnigate-fixed-route-admission/v1";

pub const PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION: u16 = 1;

pub const PHASE14B_FORBIDDEN_ADMISSION_AUTHORITY_FLAGS: &[&str] = &[
    "dynamic_route_admission",
    "caller_controlled_downstream_url",
    "gateway_identity_authority",
    "omnigate_identity_authority",
    "request_body_logging",
    "response_body_logging",
    "secret_echo",
    "capability_material_echo",
    "wallet_or_ledger_material_echo",
    "wallet_spend",
    "ledger_mutation",
    "storage_mutation_inside_gateway",
    "storage_mutation_inside_omnigate",
    "secret_key_access",
    "key_loading",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_unseal",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOmnigateFixedRouteAdmissionDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub route_kind: NativeGatewayOmnigateRouteKind,
    pub method: NativeGatewayOmnigateRouteMethod,
    pub public_gateway_path: &'static str,
    pub omnigate_path: &'static str,
    pub downstream_passport_path: &'static str,
    pub request_body_len_bytes: u32,
    pub deadline_ms: u32,
    pub correlation_id: &'static str,
    pub device_bound_capability_present: bool,
    pub fresh_proof_present: bool,
    pub mutating_scope_requested: bool,
    pub gateway_proxy_only: bool,
    pub omnigate_orchestration_only: bool,
    pub svc_passport_authority: bool,
    pub fixed_route_catalog_reviewed: bool,
    pub typed_redacted_problem_available: bool,
    pub request_body_logging_disabled: bool,
    pub response_body_logging_disabled: bool,
    pub caller_controlled_downstream_url: bool,
    pub requests_live_gateway_mount: bool,
    pub requests_live_omnigate_mount: bool,
    pub requests_storage_mutation_inside_gateway: bool,
    pub requests_storage_mutation_inside_omnigate: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_loading: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOmnigateFixedRouteAdmissionDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub route_contract_domain: &'static str,
    pub route_contract_version: u16,
    pub route_kind: NativeGatewayOmnigateRouteKind,
    pub method: NativeGatewayOmnigateRouteMethod,
    pub public_gateway_path: &'static str,
    pub omnigate_path: &'static str,
    pub downstream_passport_path: &'static str,
    pub request_body_len_bytes: u32,
    pub deadline_ms: u32,
    pub correlation_id: &'static str,
    pub fixed_route_admitted: bool,
    pub device_bound_capability_required: bool,
    pub device_bound_capability_present: bool,
    pub fresh_proof_required: bool,
    pub fresh_proof_present: bool,
    pub mutating_scope_allowed: bool,
    pub gateway_proxy_only: bool,
    pub omnigate_orchestration_only: bool,
    pub svc_passport_authority: bool,
    pub typed_redacted_problem_available: bool,
    pub request_body_logging_disabled: bool,
    pub response_body_logging_disabled: bool,
    pub live_gateway_mount_added: bool,
    pub live_omnigate_mount_added: bool,
    pub storage_mutated_inside_gateway_or_omnigate: bool,
    pub wallet_or_ledger_mutated: bool,
    pub secret_material_exposed: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeOmnigateFixedRouteAdmissionReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    RouteCatalogRejected(NativeGatewayOmnigateRouteReviewError),
    RouteKindMissing,
    MethodMismatch,
    GatewayPathMismatch,
    OmnigatePathMismatch,
    DownstreamPassportPathMismatch,
    BodyCapExceeded,
    DeadlineExceeded,
    MissingCorrelationId,
    MissingRequiredDeviceBoundCapability,
    MissingRequiredFreshProof,
    MutatingScopeEscalationRequested,
    GatewayClaimsIdentityAuthority,
    OmnigateClaimsIdentityAuthority,
    SvcPassportAuthorityMissing,
    FixedRouteCatalogNotReviewed,
    TypedRedactedProblemUnavailable,
    RequestOrResponseBodyLoggingEnabled,
    CallerControlledDownstreamUrl,
    LiveRouteMountRequested,
    UnsafeAdmissionAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOmnigateFixedRouteAdmissionPosture {
    pub phase_label: &'static str,
    pub fixed_route_admission_added: bool,
    pub fixed_route_catalog_reused: bool,
    pub body_cap_review_added: bool,
    pub deadline_review_added: bool,
    pub correlation_id_review_added: bool,
    pub capability_requirement_review_added: bool,
    pub fresh_proof_requirement_review_added: bool,
    pub typed_redacted_problem_review_added: bool,
    pub live_gateway_route_mount_added: bool,
    pub live_omnigate_route_mount_added: bool,
    pub gateway_identity_authority_added: bool,
    pub omnigate_identity_authority_added: bool,
    pub storage_mutation_inside_gateway_or_omnigate_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub secret_key_access_added: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_omnigate_fixed_route_admission_posture() -> NativeOmnigateFixedRouteAdmissionPosture {
    NativeOmnigateFixedRouteAdmissionPosture {
        phase_label: NATIVE_PASSPORT_PHASE14B_LABEL,
        fixed_route_admission_added: true,
        fixed_route_catalog_reused: true,
        body_cap_review_added: true,
        deadline_review_added: true,
        correlation_id_review_added: true,
        capability_requirement_review_added: true,
        fresh_proof_requirement_review_added: true,
        typed_redacted_problem_review_added: true,
        live_gateway_route_mount_added: false,
        live_omnigate_route_mount_added: false,
        gateway_identity_authority_added: false,
        omnigate_identity_authority_added: false,
        storage_mutation_inside_gateway_or_omnigate_added: false,
        wallet_or_ledger_mutation_added: false,
        secret_key_access_added: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE14B_FORBIDDEN_ADMISSION_AUTHORITY_FLAGS,
    }
}

pub fn review_native_omnigate_fixed_route_admission(
    draft: NativeOmnigateFixedRouteAdmissionDraftV1,
) -> Result<NativeOmnigateFixedRouteAdmissionDecisionV1, NativeOmnigateFixedRouteAdmissionReviewError>
{
    if draft.contract_domain != PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::ContractVersionMismatch);
    }

    review_native_gateway_omnigate_route_catalog(PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG)
        .map_err(NativeOmnigateFixedRouteAdmissionReviewError::RouteCatalogRejected)?;

    let expected = fixed_route_spec(draft.route_kind)
        .ok_or(NativeOmnigateFixedRouteAdmissionReviewError::RouteKindMissing)?;

    validate_draft_against_fixed_spec(&draft, expected)?;
    validate_admission_guards(&draft, expected)?;
    validate_no_unsafe_flags(&draft)?;

    Ok(NativeOmnigateFixedRouteAdmissionDecisionV1 {
        contract_domain: PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN,
        contract_version: PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION,
        route_contract_domain: PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN,
        route_contract_version: PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION,
        route_kind: draft.route_kind,
        method: draft.method,
        public_gateway_path: draft.public_gateway_path,
        omnigate_path: draft.omnigate_path,
        downstream_passport_path: draft.downstream_passport_path,
        request_body_len_bytes: draft.request_body_len_bytes,
        deadline_ms: draft.deadline_ms,
        correlation_id: draft.correlation_id,
        fixed_route_admitted: true,
        device_bound_capability_required: expected.requires_device_bound_capability,
        device_bound_capability_present: draft.device_bound_capability_present,
        fresh_proof_required: expected.requires_fresh_proof,
        fresh_proof_present: draft.fresh_proof_present,
        mutating_scope_allowed: false,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
        typed_redacted_problem_available: true,
        request_body_logging_disabled: true,
        response_body_logging_disabled: true,
        live_gateway_mount_added: false,
        live_omnigate_mount_added: false,
        storage_mutated_inside_gateway_or_omnigate: false,
        wallet_or_ledger_mutated: false,
        secret_material_exposed: false,
        contract_only: true,
    })
}

fn fixed_route_spec(
    route_kind: NativeGatewayOmnigateRouteKind,
) -> Option<&'static NativeGatewayOmnigateRouteSpecV1> {
    PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG
        .iter()
        .find(|spec| spec.kind == route_kind)
}

fn validate_draft_against_fixed_spec(
    draft: &NativeOmnigateFixedRouteAdmissionDraftV1,
    expected: &NativeGatewayOmnigateRouteSpecV1,
) -> Result<(), NativeOmnigateFixedRouteAdmissionReviewError> {
    if draft.method != expected.method {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::MethodMismatch);
    }

    if draft.public_gateway_path != expected.public_gateway_path {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::GatewayPathMismatch);
    }

    if draft.omnigate_path != expected.omnigate_path {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::OmnigatePathMismatch);
    }

    if draft.downstream_passport_path != expected.downstream_passport_path {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::DownstreamPassportPathMismatch);
    }

    Ok(())
}

fn validate_admission_guards(
    draft: &NativeOmnigateFixedRouteAdmissionDraftV1,
    expected: &NativeGatewayOmnigateRouteSpecV1,
) -> Result<(), NativeOmnigateFixedRouteAdmissionReviewError> {
    if draft.request_body_len_bytes > expected.body_cap_bytes {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::BodyCapExceeded);
    }

    if draft.deadline_ms == 0 || draft.deadline_ms > expected.deadline_ms {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::DeadlineExceeded);
    }

    if draft.correlation_id.trim().is_empty() {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::MissingCorrelationId);
    }

    if expected.requires_device_bound_capability && !draft.device_bound_capability_present {
        return Err(
            NativeOmnigateFixedRouteAdmissionReviewError::MissingRequiredDeviceBoundCapability,
        );
    }

    if expected.requires_fresh_proof && !draft.fresh_proof_present {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::MissingRequiredFreshProof);
    }

    if draft.mutating_scope_requested || expected.mutating_scope_allowed {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::MutatingScopeEscalationRequested);
    }

    if !draft.gateway_proxy_only {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::GatewayClaimsIdentityAuthority);
    }

    if !draft.omnigate_orchestration_only {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::OmnigateClaimsIdentityAuthority);
    }

    if !draft.svc_passport_authority {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::SvcPassportAuthorityMissing);
    }

    if !draft.fixed_route_catalog_reviewed {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::FixedRouteCatalogNotReviewed);
    }

    if !draft.typed_redacted_problem_available {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::TypedRedactedProblemUnavailable);
    }

    if !draft.request_body_logging_disabled || !draft.response_body_logging_disabled {
        return Err(
            NativeOmnigateFixedRouteAdmissionReviewError::RequestOrResponseBodyLoggingEnabled,
        );
    }

    if draft.caller_controlled_downstream_url {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::CallerControlledDownstreamUrl);
    }

    if draft.requests_live_gateway_mount || draft.requests_live_omnigate_mount {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::LiveRouteMountRequested);
    }

    Ok(())
}

fn validate_no_unsafe_flags(
    draft: &NativeOmnigateFixedRouteAdmissionDraftV1,
) -> Result<(), NativeOmnigateFixedRouteAdmissionReviewError> {
    if draft.requests_storage_mutation_inside_gateway
        || draft.requests_storage_mutation_inside_omnigate
        || draft.requests_secret_key_access
        || draft.requests_key_loading
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeOmnigateFixedRouteAdmissionReviewError::UnsafeAdmissionAuthorityFlag);
    }

    Ok(())
}
