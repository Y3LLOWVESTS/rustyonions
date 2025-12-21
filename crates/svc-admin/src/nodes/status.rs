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
use crate::dto::node::{AdminStatusView, PlaneStatus};

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
    pub version: String,

    /// Best-effort uptime in seconds; may be missing on older nodes.
    pub uptime_seconds: Option<u64>,

    /// Optional capability strings; may be missing on older nodes.
    pub capabilities: Option<Vec<String>>,

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
        version: None,
        uptime_seconds: None,
        capabilities: None,
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
/// - Planes are 1:1 mapped into PlaneStatus DTOs.
pub fn from_raw(id: &str, cfg: &NodeCfg, raw: RawStatus) -> AdminStatusView {
    let display_name = cfg
        .display_name
        .clone()
        .unwrap_or_else(|| id.to_string());

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
        version,
        uptime_seconds: raw.uptime_seconds,
        capabilities: raw.capabilities,
        planes,
    }
}
