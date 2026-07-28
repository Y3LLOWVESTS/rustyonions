//! RO:WHAT — Native Passport gateway/Omnigate route mount acceptance contract.
//! RO:WHY — Phase 14D closes the fixed gateway/Omnigate integration path by accepting the full fixed-route admission chain before real route mounting work moves outside the native Passport DTO layer.
//! RO:INTERACTS — Phase 14A route catalog, Phase 14B Omnigate admission, Phase 14C gateway admission, typed redacted problems, and future svc-gateway/Omnigate route mounting.
//! RO:INVARIANTS — every mounted route must come from the fixed catalog and pass gateway admission plus Omnigate admission. This phase validates the mount plan only; it does not mount live routes, add routers, mutate storage, touch wallet/ledger state, or access secrets/keys.
//! RO:TEST — tests/native_passport_phase14d_gateway_omnigate_route_mount_acceptance.rs.

use std::collections::HashSet;

use super::{
    review_native_gateway_omnigate_route_catalog, NativeGatewayFixedRouteAdmissionDecisionV1,
    NativeGatewayOmnigateRouteKind, NativeGatewayOmnigateRouteMethod,
    NativeGatewayOmnigateRouteReviewError, NativeGatewayOmnigateRouteSpecV1,
    PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG, PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN,
    PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION,
    PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN,
    PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION, PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_DOMAIN,
    PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_VERSION,
};

pub const NATIVE_PASSPORT_PHASE14D_LABEL: &str =
    "NATIVE_PASSPORT_PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE";

pub const PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_DOMAIN: &str =
    "native-passport/gateway-omnigate-route-mount-acceptance/v1";

pub const PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_VERSION: u16 = 1;

pub const PHASE14D_FORBIDDEN_ROUTE_MOUNT_AUTHORITY_FLAGS: &[&str] = &[
    "dynamic_route_mount",
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
pub struct NativeGatewayOmnigateRouteMountAcceptanceDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub expected_route_count: usize,
    pub fixed_route_catalog_reviewed: bool,
    pub gateway_admission_reviewed: bool,
    pub omnigate_admission_reviewed: bool,
    pub all_routes_have_correlation_id: bool,
    pub all_routes_have_body_caps: bool,
    pub all_routes_have_deadlines: bool,
    pub typed_redacted_problem_available: bool,
    pub gateway_proxy_only: bool,
    pub omnigate_orchestration_only: bool,
    pub svc_passport_authority: bool,
    pub mount_plan_only: bool,
    pub requests_live_gateway_route_mount: bool,
    pub requests_live_omnigate_route_mount: bool,
    pub requests_dynamic_route_mount: bool,
    pub caller_controlled_downstream_url: bool,
    pub requests_request_body_logging: bool,
    pub requests_response_body_logging: bool,
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
pub struct NativeGatewayOmnigateRouteMountAcceptanceDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub route_contract_domain: &'static str,
    pub route_contract_version: u16,
    pub omnigate_admission_contract_domain: &'static str,
    pub omnigate_admission_contract_version: u16,
    pub gateway_admission_contract_domain: &'static str,
    pub gateway_admission_contract_version: u16,
    pub accepted_route_count: usize,
    pub all_fixed_routes_covered: bool,
    pub fixed_route_catalog_reviewed: bool,
    pub gateway_admission_reviewed: bool,
    pub omnigate_admission_reviewed: bool,
    pub all_routes_have_correlation_id: bool,
    pub all_routes_have_body_caps: bool,
    pub all_routes_have_deadlines: bool,
    pub typed_redacted_problem_available: bool,
    pub gateway_proxy_only: bool,
    pub omnigate_orchestration_only: bool,
    pub svc_passport_authority: bool,
    pub mount_plan_only: bool,
    pub live_gateway_route_mount_added: bool,
    pub live_omnigate_route_mount_added: bool,
    pub dynamic_route_mount_added: bool,
    pub storage_mutated_inside_gateway_or_omnigate: bool,
    pub wallet_or_ledger_mutated: bool,
    pub secret_material_exposed: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGatewayOmnigateRouteMountAcceptanceReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    RouteCatalogRejected(NativeGatewayOmnigateRouteReviewError),
    EmptyGatewayAdmissionDecisionSet,
    RouteCountMismatch,
    FixedRouteCatalogNotReviewed,
    GatewayAdmissionNotReviewed,
    OmnigateAdmissionNotReviewed,
    CorrelationIdReviewMissing,
    BodyCapReviewMissing,
    DeadlineReviewMissing,
    TypedRedactedProblemUnavailable,
    GatewayClaimsIdentityAuthority,
    OmnigateClaimsIdentityAuthority,
    SvcPassportAuthorityMissing,
    NotMountPlanOnly,
    LiveRouteMountRequested,
    DynamicRouteMountRequested,
    CallerControlledDownstreamUrl,
    RequestOrResponseBodyLoggingRequested,
    DuplicateGatewayRouteDecision,
    MissingFixedRouteDecision,
    UnknownFixedRouteDecision,
    GatewayAdmissionDecisionContractMismatch,
    OmnigateAdmissionDecisionContractMismatch,
    RouteContractMismatch,
    GatewayAdmissionDecisionNotAccepted,
    RouteDecisionShapeMismatch,
    RouteDecisionUnsafe,
    UnsafeRouteMountAuthorityFlag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGatewayOmnigateRouteMountAcceptancePosture {
    pub phase_label: &'static str,
    pub route_mount_acceptance_added: bool,
    pub fixed_route_catalog_reused: bool,
    pub gateway_admission_chain_reused: bool,
    pub omnigate_admission_chain_reused: bool,
    pub body_cap_acceptance_added: bool,
    pub deadline_acceptance_added: bool,
    pub correlation_id_acceptance_added: bool,
    pub typed_redacted_problem_acceptance_added: bool,
    pub live_gateway_route_mount_added: bool,
    pub live_omnigate_route_mount_added: bool,
    pub dynamic_route_mount_added: bool,
    pub gateway_identity_authority_added: bool,
    pub omnigate_identity_authority_added: bool,
    pub storage_mutation_inside_gateway_or_omnigate_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub secret_key_access_added: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_gateway_omnigate_route_mount_acceptance_posture(
) -> NativeGatewayOmnigateRouteMountAcceptancePosture {
    NativeGatewayOmnigateRouteMountAcceptancePosture {
        phase_label: NATIVE_PASSPORT_PHASE14D_LABEL,
        route_mount_acceptance_added: true,
        fixed_route_catalog_reused: true,
        gateway_admission_chain_reused: true,
        omnigate_admission_chain_reused: true,
        body_cap_acceptance_added: true,
        deadline_acceptance_added: true,
        correlation_id_acceptance_added: true,
        typed_redacted_problem_acceptance_added: true,
        live_gateway_route_mount_added: false,
        live_omnigate_route_mount_added: false,
        dynamic_route_mount_added: false,
        gateway_identity_authority_added: false,
        omnigate_identity_authority_added: false,
        storage_mutation_inside_gateway_or_omnigate_added: false,
        wallet_or_ledger_mutation_added: false,
        secret_key_access_added: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE14D_FORBIDDEN_ROUTE_MOUNT_AUTHORITY_FLAGS,
    }
}

pub fn review_native_gateway_omnigate_route_mount_acceptance(
    draft: NativeGatewayOmnigateRouteMountAcceptanceDraftV1,
    gateway_admissions: &[NativeGatewayFixedRouteAdmissionDecisionV1],
) -> Result<
    NativeGatewayOmnigateRouteMountAcceptanceDecisionV1,
    NativeGatewayOmnigateRouteMountAcceptanceReviewError,
> {
    if draft.contract_domain != PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_DOMAIN {
        return Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_VERSION {
        return Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::ContractVersionMismatch);
    }

    review_native_gateway_omnigate_route_catalog(PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG)
        .map_err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::RouteCatalogRejected)?;

    validate_draft(&draft)?;
    validate_gateway_admissions(&draft, gateway_admissions)?;

    Ok(NativeGatewayOmnigateRouteMountAcceptanceDecisionV1 {
        contract_domain: PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_DOMAIN,
        contract_version: PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_VERSION,
        route_contract_domain: PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN,
        route_contract_version: PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION,
        omnigate_admission_contract_domain: PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN,
        omnigate_admission_contract_version: PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION,
        gateway_admission_contract_domain: PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_DOMAIN,
        gateway_admission_contract_version: PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_VERSION,
        accepted_route_count: gateway_admissions.len(),
        all_fixed_routes_covered: true,
        fixed_route_catalog_reviewed: true,
        gateway_admission_reviewed: true,
        omnigate_admission_reviewed: true,
        all_routes_have_correlation_id: true,
        all_routes_have_body_caps: true,
        all_routes_have_deadlines: true,
        typed_redacted_problem_available: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
        mount_plan_only: true,
        live_gateway_route_mount_added: false,
        live_omnigate_route_mount_added: false,
        dynamic_route_mount_added: false,
        storage_mutated_inside_gateway_or_omnigate: false,
        wallet_or_ledger_mutated: false,
        secret_material_exposed: false,
        contract_only: true,
    })
}

fn validate_draft(
    draft: &NativeGatewayOmnigateRouteMountAcceptanceDraftV1,
) -> Result<(), NativeGatewayOmnigateRouteMountAcceptanceReviewError> {
    if draft.expected_route_count != PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.len() {
        return Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::RouteCountMismatch);
    }

    if !draft.fixed_route_catalog_reviewed {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::FixedRouteCatalogNotReviewed,
        );
    }

    if !draft.gateway_admission_reviewed {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::GatewayAdmissionNotReviewed,
        );
    }

    if !draft.omnigate_admission_reviewed {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::OmnigateAdmissionNotReviewed,
        );
    }

    if !draft.all_routes_have_correlation_id {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::CorrelationIdReviewMissing,
        );
    }

    if !draft.all_routes_have_body_caps {
        return Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::BodyCapReviewMissing);
    }

    if !draft.all_routes_have_deadlines {
        return Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::DeadlineReviewMissing);
    }

    if !draft.typed_redacted_problem_available {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::TypedRedactedProblemUnavailable,
        );
    }

    if !draft.gateway_proxy_only {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::GatewayClaimsIdentityAuthority,
        );
    }

    if !draft.omnigate_orchestration_only {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::OmnigateClaimsIdentityAuthority,
        );
    }

    if !draft.svc_passport_authority {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::SvcPassportAuthorityMissing,
        );
    }

    if !draft.mount_plan_only {
        return Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::NotMountPlanOnly);
    }

    if draft.requests_live_gateway_route_mount || draft.requests_live_omnigate_route_mount {
        return Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::LiveRouteMountRequested);
    }

    if draft.requests_dynamic_route_mount {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::DynamicRouteMountRequested,
        );
    }

    if draft.caller_controlled_downstream_url {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::CallerControlledDownstreamUrl,
        );
    }

    if draft.requests_request_body_logging || draft.requests_response_body_logging {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::RequestOrResponseBodyLoggingRequested,
        );
    }

    validate_no_unsafe_flags(draft)
}

fn validate_gateway_admissions(
    draft: &NativeGatewayOmnigateRouteMountAcceptanceDraftV1,
    gateway_admissions: &[NativeGatewayFixedRouteAdmissionDecisionV1],
) -> Result<(), NativeGatewayOmnigateRouteMountAcceptanceReviewError> {
    if gateway_admissions.is_empty() {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::EmptyGatewayAdmissionDecisionSet,
        );
    }

    if gateway_admissions.len() != draft.expected_route_count {
        return Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::RouteCountMismatch);
    }

    let mut seen_routes = HashSet::new();

    for decision in gateway_admissions {
        validate_decision_contracts(decision)?;
        let fixed = fixed_route_spec(decision.route_kind).ok_or(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::UnknownFixedRouteDecision,
        )?;

        if !seen_routes.insert((decision.method, decision.public_gateway_path)) {
            return Err(
                NativeGatewayOmnigateRouteMountAcceptanceReviewError::DuplicateGatewayRouteDecision,
            );
        }

        validate_decision_shape(decision, fixed)?;
        validate_decision_safety(decision)?;
    }

    for fixed in PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG {
        if !seen_routes.contains(&(fixed.method, fixed.public_gateway_path)) {
            return Err(
                NativeGatewayOmnigateRouteMountAcceptanceReviewError::MissingFixedRouteDecision,
            );
        }
    }

    Ok(())
}

fn validate_decision_contracts(
    decision: &NativeGatewayFixedRouteAdmissionDecisionV1,
) -> Result<(), NativeGatewayOmnigateRouteMountAcceptanceReviewError> {
    if decision.contract_domain != PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_DOMAIN
        || decision.contract_version != PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_VERSION
    {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::GatewayAdmissionDecisionContractMismatch,
        );
    }

    if decision.omnigate_admission_contract_domain != PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN
        || decision.omnigate_admission_contract_version
            != PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION
    {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::OmnigateAdmissionDecisionContractMismatch,
        );
    }

    if decision.route_contract_domain != PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN
        || decision.route_contract_version != PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION
    {
        return Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::RouteContractMismatch);
    }

    Ok(())
}

fn validate_decision_shape(
    decision: &NativeGatewayFixedRouteAdmissionDecisionV1,
    fixed: &NativeGatewayOmnigateRouteSpecV1,
) -> Result<(), NativeGatewayOmnigateRouteMountAcceptanceReviewError> {
    if decision.method != fixed.method
        || decision.public_gateway_path != fixed.public_gateway_path
        || decision.omnigate_path != fixed.omnigate_path
        || decision.downstream_passport_path != fixed.downstream_passport_path
        || decision.device_bound_capability_required != fixed.requires_device_bound_capability
        || decision.fresh_proof_required != fixed.requires_fresh_proof
    {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::RouteDecisionShapeMismatch,
        );
    }

    Ok(())
}

fn validate_decision_safety(
    decision: &NativeGatewayFixedRouteAdmissionDecisionV1,
) -> Result<(), NativeGatewayOmnigateRouteMountAcceptanceReviewError> {
    if !decision.fixed_gateway_route_admitted || !decision.omnigate_fixed_route_admission_reviewed {
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::GatewayAdmissionDecisionNotAccepted,
        );
    }

    if !decision.gateway_proxy_only
        || !decision.omnigate_orchestration_only
        || !decision.svc_passport_authority
        || !decision.typed_redacted_problem_available
        || !decision.request_body_logging_disabled
        || !decision.response_body_logging_disabled
        || decision.mutating_scope_allowed
        || decision.live_gateway_mount_added
        || decision.live_omnigate_mount_added
        || decision.storage_mutated_inside_gateway_or_omnigate
        || decision.wallet_or_ledger_mutated
        || decision.secret_material_exposed
        || !decision.contract_only
        || decision.correlation_id.trim().is_empty()
    {
        return Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::RouteDecisionUnsafe);
    }

    Ok(())
}

fn fixed_route_spec(
    route_kind: NativeGatewayOmnigateRouteKind,
) -> Option<&'static NativeGatewayOmnigateRouteSpecV1> {
    PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG
        .iter()
        .find(|spec| spec.kind == route_kind)
}

fn validate_no_unsafe_flags(
    draft: &NativeGatewayOmnigateRouteMountAcceptanceDraftV1,
) -> Result<(), NativeGatewayOmnigateRouteMountAcceptanceReviewError> {
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
        return Err(
            NativeGatewayOmnigateRouteMountAcceptanceReviewError::UnsafeRouteMountAuthorityFlag,
        );
    }

    Ok(())
}

pub fn native_gateway_omnigate_route_mount_acceptance_fixed_route_count() -> usize {
    PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.len()
}

pub fn native_gateway_omnigate_route_mount_acceptance_fixed_route_methods(
) -> &'static [NativeGatewayOmnigateRouteMethod] {
    &[
        NativeGatewayOmnigateRouteMethod::Post,
        NativeGatewayOmnigateRouteMethod::Get,
    ]
}

pub fn native_gateway_omnigate_route_mount_acceptance_fixed_route_kinds(
) -> &'static [NativeGatewayOmnigateRouteKind] {
    &[
        NativeGatewayOmnigateRouteKind::Register,
        NativeGatewayOmnigateRouteKind::ChallengeIssue,
        NativeGatewayOmnigateRouteKind::ProofSubmit,
        NativeGatewayOmnigateRouteKind::DeviceAuthorize,
        NativeGatewayOmnigateRouteKind::DeviceRevoke,
        NativeGatewayOmnigateRouteKind::CapabilityStatus,
        NativeGatewayOmnigateRouteKind::CapabilityRefresh,
        NativeGatewayOmnigateRouteKind::CapabilityRevoke,
        NativeGatewayOmnigateRouteKind::LocalStatus,
        NativeGatewayOmnigateRouteKind::UsernameClaim,
        NativeGatewayOmnigateRouteKind::UsernameTransfer,
        NativeGatewayOmnigateRouteKind::UsernameRelease,
        NativeGatewayOmnigateRouteKind::ProtectedRead,
    ]
}
