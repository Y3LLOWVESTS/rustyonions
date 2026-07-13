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
