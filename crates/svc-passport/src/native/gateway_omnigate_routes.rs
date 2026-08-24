//! RO:WHAT — Native Passport fixed gateway/Omnigate route contract.
//! RO:WHY — Phase 14 begins safe ingress/orchestration integration by freezing route names, caps, deadlines, correlation IDs, and redacted problem envelopes before live route mounting.
//! RO:INTERACTS — svc-gateway public routes, Omnigate v1 façade routes, svc-passport downstream authority routes, and future local-stack smoke tests.
//! RO:INVARIANTS — gateway is proxy/admission only, Omnigate is orchestration only, svc-passport remains Passport authority, problems are redacted, raw request/response body logging is disabled, and this contract adds no live route mount, storage mutation, wallet/ledger mutation, or secret/key access.
//! RO:TEST — tests/native_passport_phase14a_gateway_omnigate_route_contract.rs.

use std::collections::HashSet;

pub const NATIVE_PASSPORT_PHASE14A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT";

pub const PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN: &str =
    "native-passport/gateway-omnigate-route-contract/v1";

pub const PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION: u16 = 1;

pub const PHASE14A_MAX_BODY_CAP_BYTES: u32 = 16_384;
pub const PHASE14A_MAX_DEADLINE_MS: u32 = 5_000;

pub const PHASE14A_FORBIDDEN_GATEWAY_OMNIGATE_AUTHORITY_FLAGS: &[&str] = &[
    "gateway_identity_authority",
    "omnigate_identity_authority",
    "raw_request_body_logging",
    "raw_response_body_logging",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeGatewayOmnigateRouteMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeGatewayOmnigateRouteKind {
    Register,
    ChallengeIssue,
    ProofSubmit,
    DeviceAuthorize,
    DeviceSessionChallenge,
    DeviceSessionProof,
    DeviceRevoke,
    CapabilityStatus,
    CapabilityRefresh,
    CapabilityRevoke,
    LocalStatus,
    UsernameClaim,
    UsernameTransfer,
    UsernameRelease,
    ProtectedRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeGatewayOmnigateRouteSpecV1 {
    pub kind: NativeGatewayOmnigateRouteKind,
    pub method: NativeGatewayOmnigateRouteMethod,
    pub public_gateway_path: &'static str,
    pub omnigate_path: &'static str,
    pub downstream_passport_path: &'static str,
    pub requires_device_bound_capability: bool,
    pub requires_fresh_proof: bool,
    pub mutating_scope_allowed: bool,
    pub body_cap_bytes: u32,
    pub deadline_ms: u32,
    pub correlation_id_required: bool,
    pub redacted_problem_required: bool,
    pub raw_body_logging_disabled: bool,
    pub gateway_proxy_only: bool,
    pub omnigate_orchestration_only: bool,
    pub svc_passport_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGatewayOmnigateRouteCatalogDecisionV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub route_count: usize,
    pub all_routes_fixed: bool,
    pub gateway_proxy_only: bool,
    pub omnigate_orchestration_only: bool,
    pub svc_passport_authority: bool,
    pub body_caps_reviewed: bool,
    pub deadlines_reviewed: bool,
    pub correlation_ids_required: bool,
    pub redacted_problems_required: bool,
    pub raw_body_logging_disabled: bool,
    pub mutating_scope_escalation_allowed: bool,
    pub storage_mutated_inside_gateway_or_omnigate: bool,
    pub wallet_or_ledger_mutated: bool,
    pub secret_material_exposed: bool,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGatewayOmnigateProblemEnvelopeDraftV1 {
    pub code: &'static str,
    pub http_status: u16,
    pub retryable: bool,
    pub correlation_id: &'static str,
    pub public_message: &'static str,
    pub redacted: bool,
    pub includes_raw_request_body: bool,
    pub includes_raw_response_body: bool,
    pub includes_secret_material: bool,
    pub includes_capability_material: bool,
    pub includes_wallet_or_ledger_material: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGatewayOmnigateProblemEnvelopeV1 {
    pub code: &'static str,
    pub http_status: u16,
    pub retryable: bool,
    pub correlation_id: &'static str,
    pub public_message: &'static str,
    pub redacted: bool,
    pub raw_request_body_exposed: bool,
    pub raw_response_body_exposed: bool,
    pub secret_material_exposed: bool,
    pub capability_material_exposed: bool,
    pub wallet_or_ledger_material_exposed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGatewayOmnigateRouteReviewError {
    EmptyRouteCatalog,
    RouteCountMismatch,
    DuplicateGatewayRoute,
    DuplicateOmnigateRoute,
    DuplicateDownstreamRoute,
    RouteShapeMismatch,
    BodyCapExceeded,
    DeadlineExceeded,
    MissingCorrelationIdRequirement,
    MissingRedactedProblemRequirement,
    RawBodyLoggingEnabled,
    GatewayClaimsIdentityAuthority,
    OmnigateClaimsIdentityAuthority,
    SvcPassportAuthorityMissing,
    MutatingScopeEscalationAllowed,
    ProblemCodeMissing,
    ProblemStatusInvalid,
    ProblemCorrelationIdMissing,
    ProblemMessageMissing,
    ProblemEnvelopeNotRedacted,
    ProblemEnvelopeLeaksMaterial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGatewayOmnigateRouteContractPosture {
    pub phase_label: &'static str,
    pub route_contract_added: bool,
    pub fixed_gateway_routes_added: bool,
    pub fixed_omnigate_routes_added: bool,
    pub downstream_passport_route_mapping_added: bool,
    pub body_caps_added: bool,
    pub deadlines_added: bool,
    pub correlation_id_review_added: bool,
    pub redacted_problem_envelope_added: bool,
    pub raw_body_logging_disabled: bool,
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

pub const PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG: &[NativeGatewayOmnigateRouteSpecV1] = &[
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::Register,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/register",
        omnigate_path: "/v1/identity/passport/register",
        downstream_passport_path: "/v1/passport/register",
        requires_device_bound_capability: false,
        requires_fresh_proof: true,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    // CN-4 controlled beta narrows ChallengeIssue to RegisterRoot.
    // Generic caller-selected challenge purposes remain unmounted.
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::ChallengeIssue,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/register/challenge",
        omnigate_path: "/v1/identity/passport/register/challenge",
        downstream_passport_path: "/v1/passport/register/challenge",
        requires_device_bound_capability: false,
        requires_fresh_proof: false,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    // CN-4 controlled beta narrows ProofSubmit to RegisterRoot.
    // Generic caller-selected proof surfaces remain unmounted.
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::ProofSubmit,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/register/proof",
        omnigate_path: "/v1/identity/passport/register/proof",
        downstream_passport_path: "/v1/passport/register/proof",
        requires_device_bound_capability: false,
        requires_fresh_proof: true,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::DeviceAuthorize,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/device/authorize",
        omnigate_path: "/v1/identity/passport/device/authorize",
        downstream_passport_path: "/v1/passport/device/authorize",
        requires_device_bound_capability: false,
        requires_fresh_proof: true,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    // CN-4 device possession uses compact fixed paths, but svc-passport fixes
    // their purpose to ProveSession. Callers cannot select another purpose.
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::DeviceSessionChallenge,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/challenge",
        omnigate_path: "/v1/identity/passport/challenge",
        downstream_passport_path: "/v1/passport/challenge",
        requires_device_bound_capability: false,
        requires_fresh_proof: false,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::DeviceSessionProof,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/prove",
        omnigate_path: "/v1/identity/passport/prove",
        downstream_passport_path: "/v1/passport/prove",
        requires_device_bound_capability: false,
        requires_fresh_proof: true,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::DeviceRevoke,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/device/revoke",
        omnigate_path: "/v1/identity/passport/device/revoke",
        downstream_passport_path: "/v1/passport/device/revoke",
        requires_device_bound_capability: false,
        requires_fresh_proof: true,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::CapabilityStatus,
        method: NativeGatewayOmnigateRouteMethod::Get,
        public_gateway_path: "/identity/passport/capability/status",
        omnigate_path: "/v1/identity/passport/capability/status",
        downstream_passport_path: "/v1/passport/capability/status",
        requires_device_bound_capability: true,
        requires_fresh_proof: false,
        mutating_scope_allowed: false,
        body_cap_bytes: 0,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::CapabilityRefresh,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/capability/refresh",
        omnigate_path: "/v1/identity/passport/capability/refresh",
        downstream_passport_path: "/v1/passport/capability/refresh",
        requires_device_bound_capability: true,
        requires_fresh_proof: true,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::CapabilityRevoke,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/capability/revoke",
        omnigate_path: "/v1/identity/passport/capability/revoke",
        downstream_passport_path: "/v1/passport/capability/revoke",
        requires_device_bound_capability: true,
        requires_fresh_proof: true,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::LocalStatus,
        method: NativeGatewayOmnigateRouteMethod::Get,
        public_gateway_path: "/identity/passport/status",
        omnigate_path: "/v1/identity/passport/status",
        downstream_passport_path: "/v1/passport/status",
        requires_device_bound_capability: false,
        requires_fresh_proof: false,
        mutating_scope_allowed: false,
        body_cap_bytes: 0,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::UsernameClaim,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/username/claim",
        omnigate_path: "/v1/identity/passport/username/claim",
        downstream_passport_path: "/v1/passport/username/claim",
        requires_device_bound_capability: false,
        requires_fresh_proof: true,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::UsernameTransfer,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/username/transfer",
        omnigate_path: "/v1/identity/passport/username/transfer",
        downstream_passport_path: "/v1/passport/username/transfer",
        requires_device_bound_capability: false,
        requires_fresh_proof: true,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::UsernameRelease,
        method: NativeGatewayOmnigateRouteMethod::Post,
        public_gateway_path: "/identity/passport/username/release",
        omnigate_path: "/v1/identity/passport/username/release",
        downstream_passport_path: "/v1/passport/username/release",
        requires_device_bound_capability: false,
        requires_fresh_proof: true,
        mutating_scope_allowed: false,
        body_cap_bytes: PHASE14A_MAX_BODY_CAP_BYTES,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
    NativeGatewayOmnigateRouteSpecV1 {
        kind: NativeGatewayOmnigateRouteKind::ProtectedRead,
        method: NativeGatewayOmnigateRouteMethod::Get,
        public_gateway_path: "/identity/passport/protected/read",
        omnigate_path: "/v1/identity/passport/protected/read",
        downstream_passport_path: "/v1/passport/protected/read",
        requires_device_bound_capability: true,
        requires_fresh_proof: false,
        mutating_scope_allowed: false,
        body_cap_bytes: 0,
        deadline_ms: PHASE14A_MAX_DEADLINE_MS,
        correlation_id_required: true,
        redacted_problem_required: true,
        raw_body_logging_disabled: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
    },
];

pub fn native_gateway_omnigate_route_contract_posture() -> NativeGatewayOmnigateRouteContractPosture
{
    NativeGatewayOmnigateRouteContractPosture {
        phase_label: NATIVE_PASSPORT_PHASE14A_LABEL,
        route_contract_added: true,
        fixed_gateway_routes_added: true,
        fixed_omnigate_routes_added: true,
        downstream_passport_route_mapping_added: true,
        body_caps_added: true,
        deadlines_added: true,
        correlation_id_review_added: true,
        redacted_problem_envelope_added: true,
        raw_body_logging_disabled: true,
        live_gateway_route_mount_added: false,
        live_omnigate_route_mount_added: false,
        gateway_identity_authority_added: false,
        omnigate_identity_authority_added: false,
        storage_mutation_inside_gateway_or_omnigate_added: false,
        wallet_or_ledger_mutation_added: false,
        secret_key_access_added: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE14A_FORBIDDEN_GATEWAY_OMNIGATE_AUTHORITY_FLAGS,
    }
}

pub fn review_native_gateway_omnigate_route_catalog(
    specs: &[NativeGatewayOmnigateRouteSpecV1],
) -> Result<NativeGatewayOmnigateRouteCatalogDecisionV1, NativeGatewayOmnigateRouteReviewError> {
    if specs.is_empty() {
        return Err(NativeGatewayOmnigateRouteReviewError::EmptyRouteCatalog);
    }

    if specs.len() != PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.len() {
        return Err(NativeGatewayOmnigateRouteReviewError::RouteCountMismatch);
    }

    let mut gateway_routes = HashSet::new();
    let mut omnigate_routes = HashSet::new();
    let mut downstream_routes = HashSet::new();

    for spec in specs {
        if spec.mutating_scope_allowed {
            return Err(NativeGatewayOmnigateRouteReviewError::MutatingScopeEscalationAllowed);
        }

        validate_route_shape(spec)?;

        if !gateway_routes.insert((spec.method, spec.public_gateway_path)) {
            return Err(NativeGatewayOmnigateRouteReviewError::DuplicateGatewayRoute);
        }

        if !omnigate_routes.insert((spec.method, spec.omnigate_path)) {
            return Err(NativeGatewayOmnigateRouteReviewError::DuplicateOmnigateRoute);
        }

        if !downstream_routes.insert((spec.method, spec.downstream_passport_path)) {
            return Err(NativeGatewayOmnigateRouteReviewError::DuplicateDownstreamRoute);
        }

        if spec.body_cap_bytes > PHASE14A_MAX_BODY_CAP_BYTES {
            return Err(NativeGatewayOmnigateRouteReviewError::BodyCapExceeded);
        }

        if spec.deadline_ms == 0 || spec.deadline_ms > PHASE14A_MAX_DEADLINE_MS {
            return Err(NativeGatewayOmnigateRouteReviewError::DeadlineExceeded);
        }

        if !spec.correlation_id_required {
            return Err(NativeGatewayOmnigateRouteReviewError::MissingCorrelationIdRequirement);
        }

        if !spec.redacted_problem_required {
            return Err(NativeGatewayOmnigateRouteReviewError::MissingRedactedProblemRequirement);
        }

        if !spec.raw_body_logging_disabled {
            return Err(NativeGatewayOmnigateRouteReviewError::RawBodyLoggingEnabled);
        }

        if !spec.gateway_proxy_only {
            return Err(NativeGatewayOmnigateRouteReviewError::GatewayClaimsIdentityAuthority);
        }

        if !spec.omnigate_orchestration_only {
            return Err(NativeGatewayOmnigateRouteReviewError::OmnigateClaimsIdentityAuthority);
        }

        if !spec.svc_passport_authority {
            return Err(NativeGatewayOmnigateRouteReviewError::SvcPassportAuthorityMissing);
        }

        if spec.mutating_scope_allowed {
            return Err(NativeGatewayOmnigateRouteReviewError::MutatingScopeEscalationAllowed);
        }
    }

    Ok(NativeGatewayOmnigateRouteCatalogDecisionV1 {
        contract_domain: PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN,
        contract_version: PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION,
        route_count: specs.len(),
        all_routes_fixed: true,
        gateway_proxy_only: true,
        omnigate_orchestration_only: true,
        svc_passport_authority: true,
        body_caps_reviewed: true,
        deadlines_reviewed: true,
        correlation_ids_required: true,
        redacted_problems_required: true,
        raw_body_logging_disabled: true,
        mutating_scope_escalation_allowed: false,
        storage_mutated_inside_gateway_or_omnigate: false,
        wallet_or_ledger_mutated: false,
        secret_material_exposed: false,
        contract_only: true,
    })
}

pub fn review_native_gateway_omnigate_problem_envelope(
    draft: NativeGatewayOmnigateProblemEnvelopeDraftV1,
) -> Result<NativeGatewayOmnigateProblemEnvelopeV1, NativeGatewayOmnigateRouteReviewError> {
    if draft.code.trim().is_empty() {
        return Err(NativeGatewayOmnigateRouteReviewError::ProblemCodeMissing);
    }

    if !(400..=599).contains(&draft.http_status) {
        return Err(NativeGatewayOmnigateRouteReviewError::ProblemStatusInvalid);
    }

    if draft.correlation_id.trim().is_empty() {
        return Err(NativeGatewayOmnigateRouteReviewError::ProblemCorrelationIdMissing);
    }

    if draft.public_message.trim().is_empty() {
        return Err(NativeGatewayOmnigateRouteReviewError::ProblemMessageMissing);
    }

    if !draft.redacted {
        return Err(NativeGatewayOmnigateRouteReviewError::ProblemEnvelopeNotRedacted);
    }

    if draft.includes_raw_request_body
        || draft.includes_raw_response_body
        || draft.includes_secret_material
        || draft.includes_capability_material
        || draft.includes_wallet_or_ledger_material
    {
        return Err(NativeGatewayOmnigateRouteReviewError::ProblemEnvelopeLeaksMaterial);
    }

    Ok(NativeGatewayOmnigateProblemEnvelopeV1 {
        code: draft.code,
        http_status: draft.http_status,
        retryable: draft.retryable,
        correlation_id: draft.correlation_id,
        public_message: draft.public_message,
        redacted: true,
        raw_request_body_exposed: false,
        raw_response_body_exposed: false,
        secret_material_exposed: false,
        capability_material_exposed: false,
        wallet_or_ledger_material_exposed: false,
    })
}

fn validate_route_shape(
    spec: &NativeGatewayOmnigateRouteSpecV1,
) -> Result<(), NativeGatewayOmnigateRouteReviewError> {
    let expected = expected_route_spec(spec.kind);

    if spec.method != expected.method
        || spec.public_gateway_path != expected.public_gateway_path
        || spec.omnigate_path != expected.omnigate_path
        || spec.downstream_passport_path != expected.downstream_passport_path
        || spec.requires_device_bound_capability != expected.requires_device_bound_capability
        || spec.requires_fresh_proof != expected.requires_fresh_proof
        || spec.body_cap_bytes != expected.body_cap_bytes
    {
        return Err(NativeGatewayOmnigateRouteReviewError::RouteShapeMismatch);
    }

    Ok(())
}

fn expected_route_spec(kind: NativeGatewayOmnigateRouteKind) -> NativeGatewayOmnigateRouteSpecV1 {
    PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG
        .iter()
        .copied()
        .find(|spec| spec.kind == kind)
        .expect("every Phase 14A route kind must have a fixed route spec")
}
