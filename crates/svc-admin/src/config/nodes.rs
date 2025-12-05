// crates/svc-admin/src/config/nodes.rs
//
// WHAT: Node registry configuration (NodeCfg + NodesCfg).
// WHY: Keeps per-node settings isolated and reusable by NodeRegistry,
//      samplers, and UI DTOs.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};

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
    #[serde(skip)]
    pub default_timeout: Option<Duration>,
}

pub type NodesCfg = BTreeMap<String, NodeCfg>;

/// Seed node configuration used by Config::default().
pub(crate) fn default_nodes() -> NodesCfg {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        "example-node".to_string(),
        NodeCfg {
            base_url: "http://127.0.0.1:9000".to_string(),
            display_name: Some("Example Node".to_string()),
            environment: "dev".to_string(),
            insecure_http: true,
            forced_profile: Some("macronode".to_string()),
            macaroon_path: None,
            default_timeout: Some(Duration::from_secs(2)),
        },
    );
    nodes
}
