// crates/svc-admin/src/nodes/status.rs
//
// WHAT: Normalization of raw node status into AdminStatusView.
// WHY: Keep HTTP fetching (NodeClient) separate from DTO shaping so the
//      admin plane contract can evolve without leaking everywhere.

use crate::config::NodeCfg;
use crate::dto::node::{AdminStatusView, PlaneStatus};

/// Raw per-plane status coming from `/api/v1/status`.
///
/// This is intentionally minimal and matches the shape used in
/// `tests/fake_node.rs`.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct RawPlane {
    pub name: String,
    pub health: String,
    pub ready: bool,
    pub restart_count: u64,
}

/// Raw aggregated status from `/api/v1/status`.
///
/// NOTE:
/// - `profile` and `version` are optional to allow nodes to omit them.
/// - Tests currently always provide both fields.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct RawStatus {
    pub profile: Option<String>,
    pub version: Option<String>,
    pub planes: Vec<RawPlane>,
}

/// Placeholder view used when we cannot fetch real status.
///
/// This is the same as `AdminStatusView::placeholder()` but wrapped in a
/// helper so callers don’t need to know the DTO details.
pub fn build_status_placeholder() -> AdminStatusView {
    AdminStatusView::placeholder()
}

/// Normalize a RawStatus + NodeCfg into an AdminStatusView.
///
/// Invariants:
/// - `id` is always the registry key, not derived from the node.
/// - `display_name` prefers NodeCfg.display_name, falls back to id.
/// - `profile` prefers raw.profile, falls back to NodeCfg.forced_profile.
/// - `version` is taken directly from raw.version.
/// - Planes are 1:1 mapped into PlaneStatus DTOs.
pub(crate) fn from_raw(id: &str, cfg: &NodeCfg, raw: RawStatus) -> AdminStatusView {
    let display_name = cfg
        .display_name
        .clone()
        .unwrap_or_else(|| id.to_string());

    let profile = raw
        .profile
        .or_else(|| cfg.forced_profile.clone());

    let version = raw.version;

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
        planes,
    }
}
