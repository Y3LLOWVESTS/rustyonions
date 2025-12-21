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

    /// Version string reported by the node (e.g., "0.1.0").
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
