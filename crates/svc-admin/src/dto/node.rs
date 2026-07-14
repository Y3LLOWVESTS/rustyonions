// crates/svc-admin/src/dto/node.rs
//
// RO:WHAT — DTOs for node inventory and status views.
// RO:WHY  — Keep the JSON contract between svc-admin and the SPA explicit
//          and decoupled from internal config/reg structs.
// RO:INTERACTS — nodes::registry, router::nodes, router::node_status,
//                metrics::sampler (for facet metrics).

use serde::{Deserialize, Serialize};

/// Summary used on the main node list.
///
/// NOTE: Uses snake_case field names to align with the TypeScript DTO
/// in `ui/src/types/admin-api.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    /// Registry key / stable identifier.
    pub id: String,
    /// Human-friendly name, configured per-node.
    pub display_name: String,
    /// Optional profile hint (e.g. "macronode" / "micronode").
    pub profile: Option<String>,
}

/// Detailed view used on the node detail page.
///
/// NOTE: Uses snake_case for `display_name` to match the SPA contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminStatusView {
    pub id: String,
    pub display_name: String,

    /// Optional profile hint, e.g. "macronode".
    pub profile: Option<String>,

    /// Product role in the two-node CrabLink model, e.g. "user_node" or "service_node".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_role: Option<String>,

    /// Runtime profile backing the product role, e.g. "micronode" or "macronode".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_profile: Option<String>,

    /// Version string reported by the node (e.g. "0.1.0").
    /// May be absent when we only have coarse health/ready probes.
    pub version: Option<String>,

    /// Optional uptime (seconds) reported by the node status endpoint.
    ///
    /// This is best-effort and may be missing on older nodes.
    pub uptime_seconds: Option<u64>,

    /// Optional node capability labels (read-only surfaces, etc.).
    ///
    /// This is intentionally optional so older nodes / older svc-admin
    /// builds don’t break UI capability gating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,

    /// Node-level readiness reported by the canonical status endpoint.
    ///
    /// When `/api/v1/status` is unavailable, svc-admin may populate this from
    /// the node's `/readyz` probe. `None` means readiness was not observable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready: Option<bool>,

    /// Truthful OAP runtime posture reported by the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oap: Option<OapStatusView>,

    /// Truthful provider/DHT publication posture reported by the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderStatusView>,

    /// Truthful serve-policy and moderation posture reported by the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<PolicyStatusView>,

    /// Process-local persistence-review and prune posture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence_review: Option<PersistenceReviewStatusView>,

    /// Canonical accounting, reward-plan, and epoch-transition
    /// posture reported by the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub economic_pipeline: Option<EconomicPipelineStatusView>,

    /// Canonical Service Node lifecycle, quorum, containment, and
    /// appeal posture reported by the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_node_lifecycle: Option<ServiceNodeLifecycleStatusView>,

    /// Reward-recipient binding posture.
    ///
    /// This is display/status truth only. Pending or runtime-local state must
    /// never be presented as registry finality, wallet mutation, ledger
    /// mutation, a confirmed receipt, or confirmed ROC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward_binding: Option<RewardBindingStatusView>,

    /// Reviewed service-evidence outbox posture.
    ///
    /// This is not accounting acceptance, reward eligibility, payout
    /// authority, wallet mutation, or ledger mutation unless the node reports
    /// those fields truthfully.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_evidence: Option<ServiceEvidenceStatusView>,

    /// Whether the node reports amnesia-first operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amnesia_mode: Option<bool>,

    /// Whether the node reports privacy-first behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_mode: Option<bool>,

    /// Whether public inbound serving is enabled for this node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_inbound_enabled: Option<bool>,

    /// Whether the service-node daemon can operate without the UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headless_mode: Option<bool>,

    /// Whether the optional local operator UI is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_ui_enabled: Option<bool>,

    /// Bind address for the optional local operator UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_ui_bind: Option<String>,

    /// Operator UI profile label, e.g. service_node_local.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_ui_profile: Option<String>,

    /// Whether node runtime requires the UI. Must remain false for service nodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_ui_runtime_required: Option<bool>,

    /// Whether passive verification work is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_enabled: Option<bool>,

    /// Whether content serving is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_serving_enabled: Option<bool>,

    /// Whether read-only economic replay/audit work is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub economic_replay_enabled: Option<bool>,

    /// Whether service-node quorum participation is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_quorum_enabled: Option<bool>,

    /// Whether this node participates in wallet execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_execution_participant: Option<bool>,

    /// Whether ledger replay/audit work is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger_replay_enabled: Option<bool>,

    /// User-IP publication posture, e.g. "forbidden" for normal user nodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_ip_publication: Option<String>,

    /// Peer-IP display posture, e.g. "forbidden".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_ip_display: Option<String>,

    /// Whether admin bind/socket addresses may be published through status DTOs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_bind_publication: Option<bool>,

    /// Whether service socket routes may be published through status DTOs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_socket_publication: Option<String>,

    /// Whether transport-specific routes may be published through status DTOs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_routes_public: Option<bool>,

    /// Whether raw socket publication is allowed through status DTOs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_socket_publication: Option<bool>,

    /// Per-plane status (gateway/storage/index/mailbox/overlay/dht).
    pub planes: Vec<PlaneStatus>,
}

/// OAP runtime status projected into the optional operator console.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OapStatusView {
    pub protocol: String,
    pub version: u16,
    pub runtime_state: String,
    pub max_frame_bytes: u32,
    pub stream_chunk_bytes: u64,
    pub object_fetch_active: bool,
    pub full_digest_verification_active: bool,
}

/// Provider and DHT publication status projected into the operator console.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusView {
    pub state: String,
    pub dht_worker_status: String,
    pub advertisement_active: bool,
    pub provider_records_published: u64,
    pub public_node_uri_format: String,
    pub residential_ip_publication: bool,
}

/// Serve-policy and moderation status projected into the operator console.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyStatusView {
    pub state: String,
    pub serve_policy_enforced: bool,
    pub oap_serve_policy_enforced: bool,
    pub operator_moderation_active: bool,
    pub global_moderation_active: bool,
    pub moderation_configured: bool,
    pub moderation_state: String,
    pub moderation_source: String,
    pub moderation_load_failed: bool,
    pub signed_policy_verified: bool,
    pub signed_policy_epoch: Option<u64>,
    pub signed_policy_expires_at_unix_s: Option<u64>,
    pub rollback_guard_persisted: bool,
    pub moderation_activation: String,
    pub moderation_hot_reload: bool,
    pub moderation_entries: ModerationEntryCountsView,
    pub unvetted_persistence_posture: String,
    pub serve_gate_phase: String,
    pub moderation_phase: String,
}

/// Low-cardinality moderation counts safe for operator display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationEntryCountsView {
    pub total: u64,
    pub global_deny: u64,
    pub local_block: u64,
    pub local_allow: u64,
    pub owner_tombstone: u64,
    pub quarantine: u64,
}

/// Process-local persistence-review and prune status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceReviewStatusView {
    pub state: String,
    pub candidates_total: u64,
    pub awaiting_decision: u64,
    pub pending_review: u64,
    pub persistence_approvals: u64,
    pub blocked_candidates: u64,
    pub quarantined_candidates: u64,
    pub completed_local_prunes: u64,
    pub durable_bytes_written: bool,
    pub reward_finality: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Canonical accounting → reward-plan → epoch-transition posture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicPipelineStatusView {
    pub stage: String,
    pub accounting_snapshot: EconomicAccountingSnapshotStatusView,
    pub reward_plan: Option<EconomicRewardPlanStatusView>,
    pub epoch_transition: Option<EconomicEpochTransitionStatusView>,
    pub epoch_payout_receipts: Option<EconomicEpochPayoutReceiptStatusView>,
    pub wallet_execution_reported: bool,
    pub ledger_receipt_reported: bool,
    pub confirmed_roc_reported: bool,
    pub finality_reported: bool,
    pub operator_projection_authorizes_economic_mutation: bool,
}

/// Canonical sealed accounting-snapshot reference.
/// Aggregate canonical epoch-payout receipt posture.
///
/// Recipient account identifiers and per-recipient balances are deliberately
/// omitted from this operator-facing projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicEpochPayoutReceiptStatusView {
    pub receipt_count: u64,
    pub recipient_count: u64,
    pub total_issued_minor: String,
    pub first_ledger_seq: u64,
    pub last_ledger_seq: u64,
    pub ledger_root: String,
    pub first_receipt_hash: String,
    pub last_receipt_hash: String,
    pub accepted_at_ms: u64,
    pub wallet_source: String,
    pub ledger_source: String,
    pub settlement_status: String,
    pub finality_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicAccountingSnapshotStatusView {
    pub chain_id: String,
    pub snapshot_id: String,
    pub snapshot_root: String,
    pub window_started_at_ms: u64,
    pub window_ended_at_ms: u64,
    pub sealed_at_ms: u64,
    pub source_event_count: u64,
    pub economic_receipt_count: u64,
    pub metering_count: u64,
    pub proof_eligible_count: u64,
    pub ad_budgeted_count: u64,
    pub analytics_only_count: u64,
}

/// Canonical non-mutating reward-plan reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicRewardPlanStatusView {
    pub plan_id: String,
    pub plan_root: String,
    pub snapshot_id: String,
    pub snapshot_root: String,
    pub source_event_class: String,
    pub planned_total_minor: String,
    pub payout_candidate_count: u64,
    pub capped_by_policy: bool,
    pub verification_ref: Option<String>,
    pub funding_budget_ref: Option<String>,
    pub produced_at_ms: u64,
}

/// Canonical quorum-reviewed epoch-transition planning material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicEpochTransitionStatusView {
    pub chain_id: String,
    pub epoch_id: String,
    pub transition_hash: String,
    pub accounting_snapshot_hash: String,
    pub reward_plan_hash: String,
    pub policy_hash: String,
    pub economics_config_hash: String,
    pub registry_root: String,
    pub reward_binding_root: String,
    pub evidence_root: String,
    pub reward_cap_minor_units: String,
    pub reward_total_minor_units: String,
    pub allocation_count: u64,
    pub eligible_service_node_count: u16,
    pub required_signature_references: u16,
    pub supplied_signature_references: u64,
    pub quorum_reference_threshold_met: bool,
    pub cryptographic_signatures_verified: bool,
    pub recipient_accounts_resolved: bool,
    pub produced_at_ms: u64,
}

/// Canonical Service Node lifecycle and quorum posture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNodeLifecycleStatusView {
    pub lifecycle_state: String,
    pub registered_at_epoch: u64,
    pub state_effective_epoch: u64,
    pub quorum_status: String,
    pub counts_toward_quorum: bool,
    pub probation_reward_cap_required: bool,
    pub enforcement: Option<ServiceNodeContainmentStatusView>,
    pub operator_projection_authorizes_state_change: bool,
    pub operator_projection_authorizes_economic_mutation: bool,
}

/// Canonical Phase 19 containment posture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNodeContainmentStatusView {
    pub status_id: String,
    pub state: String,
    pub reason: String,
    pub evidence_root: String,
    pub effective_epoch: u64,
    pub counts_toward_quorum: bool,
    pub permits_reward_planning: bool,
    pub authorizes_economic_mutation: bool,
    pub appeal: ServiceNodeAppealStatusView,
}

/// Canonical appeal posture attached to containment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNodeAppealStatusView {
    pub state: String,
    pub appeal_id: Option<String>,
    pub submitted_epoch: Option<u64>,
    pub resolved_epoch: Option<u64>,
    pub resolution_evidence_root: Option<String>,
    pub pending: bool,
    pub authorizes_state_change: bool,
}

/// Reward-recipient binding status projected into the operator console.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardBindingStatusView {
    pub state: String,
    pub reward_recipient_display_address: Option<String>,
    pub pending_rotation_display_address: Option<String>,
    pub updated_at_unix_s: Option<u64>,
    pub registry_finality: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
    pub confirmed_roc_minor_units: Option<u64>,
}

/// Bounded service-evidence outbox status projected into the operator console.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEvidenceStatusView {
    pub state: String,
    pub queued_records: u64,
    pub delivery_records: u64,
    pub reward_evidence_records: u64,
    pub signature_required: bool,
    pub replay_scope: String,
    pub durable: bool,
    pub accounting_accepted: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
}

/// Per-plane status used inside AdminStatusView.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaneStatus {
    pub name: String,
    pub health: String,
    pub ready: bool,

    /// Restart count for this plane, as reported by the node.
    ///
    /// Invariants:
    /// - Non-negative counter.
    /// - Exposed as a simple integer so the UI can show it without graphing.
    pub restart_count: u64,
}

/// Result of a node-level control-plane action (reload/shutdown/etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeActionResponse {
    /// Node id that the action targeted.
    pub node_id: String,
    /// Logical action name, e.g. "reload" or "shutdown" or "debug-crash".
    pub action: String,
    /// Whether the action was accepted by the node (best-effort).
    pub accepted: bool,
    /// Optional human-readable message for operators.
    pub message: Option<String>,
}

/// Capability flags for actions, exposed on the SPA side so we can show/hide
/// buttons per-node depending on config and node profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeActionsCapabilities {
    pub can_reload: bool,
    pub can_shutdown: bool,
}

impl NodeActionsCapabilities {
    pub fn disabled() -> Self {
        Self {
            can_reload: false,
            can_shutdown: false,
        }
    }
}
