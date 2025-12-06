// crates/svc-admin/src/dto/node.rs
//
// RO:WHAT — DTOs for node listing, status, and control-plane actions.
// RO:WHY  — Provide a stable JSON contract between svc-admin and its SPA
//          for node inventory, per-plane status, and action results.
// RO:INTERACTS — nodes::registry, nodes::status, router, ui/src/types/admin-api.ts.
// RO:INVARIANTS —
//   - Field names are camelCase on the wire via serde’s default rules.
//   - NodeSummary/AdminStatusView must remain backwards compatible with
//     ALL_DOCS / API.md.
//   - No business logic in DTO layer; shaping only.

use serde::{Deserialize, Serialize};

/// Minimal information for listing nodes in the SPA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    /// Registry id (stable key).
    pub id: String,
    /// Human-facing name (falls back to id if not configured).
    pub display_name: String,
    /// Optional profile hint, e.g. "macronode" / "micronode".
    pub profile: Option<String>,
}

/// Per-plane status as exposed on the node admin plane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaneStatus {
    /// Plane name, e.g. "overlay", "gateway", "storage".
    pub name: String,
    /// Health string: "healthy" | "degraded" | "down".
    pub health: String,
    /// Whether the plane reports itself "ready".
    pub ready: bool,
    /// Restart counter from the node side (monotonic, best-effort).
    pub restart_count: u64,
}

/// Aggregated view for a single node used on the node detail page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminStatusView {
    /// Registry id (matches NodeSummary.id).
    pub id: String,
    /// Human-facing name (same semantics as NodeSummary.display_name).
    pub display_name: String,
    /// Optional profile hint.
    pub profile: Option<String>,
    /// Optional version string reported by the node.
    pub version: Option<String>,
    /// Per-plane status list.
    pub planes: Vec<PlaneStatus>,
}

impl AdminStatusView {
    /// Placeholder view used when we cannot talk to a real node.
    ///
    /// This is primarily for dev/demo; production callers should prefer
    /// `nodes::status::build_status_placeholder()` which wraps this.
    pub fn placeholder() -> Self {
        Self {
            id: "example-node".into(),
            display_name: "Example Node".into(),
            profile: Some("macronode".into()),
            version: Some("0.0.0".into()),
            planes: vec![],
        }
    }
}

/// Generic response for node actions like reload/shutdown.
///
/// This is intentionally minimal: the primary contract is that an action
/// was accepted by svc-admin and forwarded to the node. For richer behavior
/// we can later add correlation ids or node-returned messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeActionResponse {
    /// Registry id of the target node.
    pub node_id: String,
    /// Action verb, e.g. "reload" or "shutdown".
    pub action: String,
    /// Whether the action was accepted and forwarded to the node.
    pub accepted: bool,
    /// Optional human-facing message (error detail, etc.).
    pub message: Option<String>,
}
