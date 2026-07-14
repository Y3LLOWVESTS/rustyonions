// crates/svc-admin/src/nodes/status.rs
//
// RO:WHAT — Normalization helpers for node status.
// RO:WHY  — Keep the `/api/v1/status` wire contract isolated so NodeClient
//          and the SPA-facing DTOs can stay simple.
// RO:INTERACTS — dto::node, config::NodeCfg, nodes::client.
//
// Wire shape here mirrors the macronode/micronode RON-STATUS-V1 subset.

use serde::{Deserialize, Serialize};

use crate::config::NodeCfg;
use crate::dto::node::{
    AdminStatusView, OapStatusView, PersistenceReviewStatusView, PlaneStatus, PolicyStatusView,
    ProviderStatusView, RewardBindingStatusView, ServiceEvidenceStatusView,
};

/// Internal representation of `/api/v1/status` responses from nodes.
///
/// Mirrors the macronode/micronode admin-plane status DTO.
///
/// IMPORTANT:
/// - Fields must be tolerant of partial rollout across node versions.
/// - Anything that might be missing must be `Option<...>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawStatus {
    pub profile: Option<String>,

    /// Optional two-node product role, e.g. "user_node" or "service_node".
    pub node_role: Option<String>,

    /// Optional runtime profile backing the product role.
    pub node_profile: Option<String>,

    pub version: String,

    /// Best-effort uptime in seconds; may be missing on older nodes.
    pub uptime_seconds: Option<u64>,

    /// Optional capability strings; may be missing on older nodes.
    pub capabilities: Option<Vec<String>>,

    /// Optional node-level readiness from the canonical status endpoint.
    pub ready: Option<bool>,

    /// Optional operator-console truth blocks. Older nodes may omit all of
    /// these while still satisfying the base RON-STATUS-V1 projection.
    pub oap: Option<OapStatusView>,
    pub provider: Option<ProviderStatusView>,
    pub policy: Option<PolicyStatusView>,
    pub persistence_review: Option<PersistenceReviewStatusView>,

    /// Canonical accounting, reward-plan, and epoch-transition posture.
    ///
    /// Older nodes may omit this field. Absence must remain `None`; svc-admin
    /// must not manufacture snapshots, plans, payouts, receipts, balances,
    /// confirmed ROC, or finality.
    pub economic_pipeline: Option<crate::dto::node::EconomicPipelineStatusView>,

    pub service_node_lifecycle: Option<crate::dto::node::ServiceNodeLifecycleStatusView>,

    pub reward_binding: Option<RewardBindingStatusView>,
    pub service_evidence: Option<ServiceEvidenceStatusView>,

    pub amnesia_mode: Option<bool>,
    pub privacy_mode: Option<bool>,
    pub public_inbound_enabled: Option<bool>,
    pub headless_mode: Option<bool>,
    pub admin_ui_enabled: Option<bool>,
    pub admin_ui_bind: Option<String>,
    pub operator_ui_profile: Option<String>,
    pub admin_ui_runtime_required: Option<bool>,
    pub verification_enabled: Option<bool>,
    pub content_serving_enabled: Option<bool>,
    pub economic_replay_enabled: Option<bool>,
    pub service_quorum_enabled: Option<bool>,
    pub wallet_execution_participant: Option<bool>,
    pub ledger_replay_enabled: Option<bool>,
    pub user_ip_publication: Option<String>,
    pub peer_ip_display: Option<String>,
    pub admin_bind_publication: Option<bool>,
    pub service_socket_publication: Option<String>,
    pub transport_routes_public: Option<bool>,
    pub raw_socket_publication: Option<bool>,

    pub planes: Vec<RawPlaneStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPlaneStatus {
    pub name: String,
    pub health: String,
    pub ready: bool,
    // Matches macronode's `restart_count` field on the wire exactly.
    pub restart_count: u64,
}

/// Build a placeholder view used when we cannot reach the node.
///
/// This is what NodeClient falls back to when `/api/v1/status` is missing
/// or fails, combined with coarse /healthz + /readyz + /version probes.
pub fn build_status_placeholder() -> AdminStatusView {
    AdminStatusView {
        id: "unknown".to_string(),
        display_name: "Unknown node".to_string(),
        profile: None,
        node_role: None,
        node_profile: None,
        version: None,
        uptime_seconds: None,
        capabilities: None,
        ready: None,
        oap: None,
        provider: None,
        policy: None,
        persistence_review: None,
        economic_pipeline: None,
        service_node_lifecycle: None,
        reward_binding: None,
        service_evidence: None,
        amnesia_mode: None,
        privacy_mode: None,
        public_inbound_enabled: None,
        headless_mode: None,
        admin_ui_enabled: None,
        admin_ui_bind: None,
        operator_ui_profile: None,
        admin_ui_runtime_required: None,
        verification_enabled: None,
        content_serving_enabled: None,
        economic_replay_enabled: None,
        service_quorum_enabled: None,
        wallet_execution_participant: None,
        ledger_replay_enabled: None,
        user_ip_publication: None,
        peer_ip_display: None,
        admin_bind_publication: None,
        service_socket_publication: None,
        transport_routes_public: None,
        raw_socket_publication: None,
        planes: Vec::new(),
    }
}

/// Normalize a RawStatus + NodeCfg into an AdminStatusView.
///
/// Invariants:
/// - `id` is always the registry key, not derived from the node.
/// - `display_name` prefers NodeCfg.display_name, falls back to id.
/// - `profile` prefers raw.profile, falls back to NodeCfg.forced_profile.
/// - `version` is taken from raw.version.
/// - `uptime_seconds` is best-effort passthrough.
/// - `capabilities` is best-effort passthrough.
/// - Readiness and operator-console truth blocks are passed through without
///   manufacturing success, finality, eligibility, payout, or mutation state.
/// - Planes are 1:1 mapped into PlaneStatus DTOs.
pub fn from_raw(id: &str, cfg: &NodeCfg, raw: RawStatus) -> AdminStatusView {
    let display_name = cfg.display_name.clone().unwrap_or_else(|| id.to_string());

    let profile = raw.profile.or_else(|| cfg.forced_profile.clone());

    let version = Some(raw.version);

    let planes = raw
        .planes
        .into_iter()
        .map(|p| PlaneStatus {
            name: p.name,
            health: p.health,
            ready: p.ready,
            restart_count: p.restart_count,
        })
        .collect();

    AdminStatusView {
        id: id.to_string(),
        display_name,
        profile,
        node_role: raw.node_role,
        node_profile: raw.node_profile,
        version,
        uptime_seconds: raw.uptime_seconds,
        capabilities: raw.capabilities,
        ready: raw.ready,
        oap: raw.oap,
        provider: raw.provider,
        policy: raw.policy,
        persistence_review: raw.persistence_review,
        economic_pipeline: raw.economic_pipeline,
        service_node_lifecycle: raw.service_node_lifecycle,
        reward_binding: raw.reward_binding,
        service_evidence: raw.service_evidence,
        amnesia_mode: raw.amnesia_mode,
        privacy_mode: raw.privacy_mode,
        public_inbound_enabled: raw.public_inbound_enabled,
        headless_mode: raw.headless_mode,
        admin_ui_enabled: raw.admin_ui_enabled,
        admin_ui_bind: raw.admin_ui_bind,
        operator_ui_profile: raw.operator_ui_profile,
        admin_ui_runtime_required: raw.admin_ui_runtime_required,
        verification_enabled: raw.verification_enabled,
        content_serving_enabled: raw.content_serving_enabled,
        economic_replay_enabled: raw.economic_replay_enabled,
        service_quorum_enabled: raw.service_quorum_enabled,
        wallet_execution_participant: raw.wallet_execution_participant,
        ledger_replay_enabled: raw.ledger_replay_enabled,
        user_ip_publication: raw.user_ip_publication,
        peer_ip_display: raw.peer_ip_display,
        admin_bind_publication: raw.admin_bind_publication,
        service_socket_publication: raw.service_socket_publication,
        transport_routes_public: raw.transport_routes_public,
        raw_socket_publication: raw.raw_socket_publication,
        planes,
    }
}
