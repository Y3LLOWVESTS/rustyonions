// crates/svc-admin/src/config/nodes.rs
//
// WHAT: Node registry configuration (NodeCfg + NodesCfg).
// WHY:  Keeps per-node settings isolated and reusable by NodeRegistry,
//       samplers, and UI DTOs.
// NOTE: Default nodes must be "truthful". Dev scripts can set
//       SVC_ADMIN_NODES_CLEAR=1 to start from an empty registry and
//       supply nodes purely via env.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env, path::PathBuf, time::Duration};

/// Config for one node in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCfg {
    /// Base URL for the node's admin plane.
    pub base_url: String,

    /// Optional pretty display name.
    pub display_name: Option<String>,

    /// Environment tag ("dev", "staging", "prod").
    pub environment: String,

    /// Whether to allow insecure HTTP for this node (dev-only).
    pub insecure_http: bool,

    /// Optionally force a profile label like "macronode", "micronode".
    pub forced_profile: Option<String>,

    /// Optional macaroon path or similar credential.
    pub macaroon_path: Option<PathBuf>,

    /// Optional per-node default timeout override for scrapes.
    ///
    /// We mark this as `skip` so it is not loaded via serde; env-loading
    /// can still patch it in later if we add per-node timeout support.
    #[serde(skip)]
    pub default_timeout: Option<Duration>,
}

pub type NodesCfg = BTreeMap<String, NodeCfg>;

fn env_truthy(key: &str) -> bool {
    match env::var(key) {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes" || v == "on"
        }
        Err(_) => false,
    }
}

/// Seed node configuration used by Config::default().
///
/// IMPORTANT: These must be "truthful" and minimal.
/// - No fake nodes.
/// - No multiple entries pointing to the same process.
/// - Dev scripts can request a clean slate via `SVC_ADMIN_NODES_CLEAR=1`.
pub(crate) fn default_nodes() -> NodesCfg {
    if env_truthy("SVC_ADMIN_NODES_CLEAR") {
        return BTreeMap::new();
    }

    let mut nodes = BTreeMap::new();

    // Minimal default: one local macronode (so `cargo run -p svc-admin` works out of the box).
    nodes.insert(
        "macronode".to_string(),
        NodeCfg {
            base_url: "http://127.0.0.1:8080".to_string(),
            display_name: Some("Macronode".to_string()),
            environment: "dev".to_string(),
            insecure_http: true,
            forced_profile: Some("macronode".to_string()),
            macaroon_path: None,
            default_timeout: Some(Duration::from_secs(2)),
        },
    );

    nodes
}
