// crates/svc-admin/src/nodes/registry.rs
//
// RO:WHAT — In-memory registry of nodes known to svc-admin.
// RO:WHY  — Centralize node lookup, status fetching, and control-plane
//          actions (reload/shutdown) behind a simple API.
// RO:INTERACTS — config::NodesCfg, nodes::client::NodeClient, dto::node,
//                metrics::sampler, router.
// RO:INVARIANTS —
//   - Registry is read-only at runtime (Arc<NodesCfg>).
//   - Node ids are always the config keys, not derived from node responses.
//   - No locks are held across `await` boundaries (purely read-only and
//     relies on Clone/Arc).
//
// RO:METRICS/LOGS — Emits warnings on failed status/action calls.
// RO:TEST HOOKS — Exercised indirectly by http_smoke + future action tests.

use crate::config::{NodeCfg, NodesCfg};
use crate::dto::node::{AdminStatusView, NodeActionResponse, NodeSummary};
use crate::error::{Error, Result};
use crate::nodes::client::NodeClient;
use crate::nodes::status;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Read-only node registry backed by configuration.
#[derive(Clone)]
pub struct NodeRegistry {
    nodes: Arc<BTreeMap<String, NodeCfg>>,
    client: NodeClient,
}

impl NodeRegistry {
    /// Construct a new registry from the config-driven map.
    pub fn new(cfg: &NodesCfg) -> Self {
        Self {
            nodes: Arc::new(cfg.clone()),
            client: NodeClient::new(),
        }
    }

    /// Return a summary list for UI listing.
    pub fn list_summaries(&self) -> Vec<NodeSummary> {
        self.nodes
            .iter()
            .map(|(id, cfg)| NodeSummary {
                id: id.clone(),
                display_name: cfg
                    .display_name
                    .clone()
                    .unwrap_or_else(|| id.clone()),
                profile: cfg.forced_profile.clone(),
            })
            .collect()
    }

    /// Whether a node id exists in the registry.
    pub fn contains(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    /// Return the status view for a given node id, if present.
    ///
    /// This will:
    /// - Look up the node in the config registry.
    /// - Call NodeClient::fetch_status to do real HTTP calls.
    /// - On failure, log and fall back to a placeholder view.
    pub async fn get_status(&self, id: &str) -> Option<AdminStatusView> {
        let cfg = match self.nodes.get(id) {
            Some(c) => c,
            None => return None,
        };

        match self.client.fetch_status(id, cfg).await {
            Ok(view) => Some(view),
            Err(err) => {
                tracing::warn!(
                    node_id = id,
                    error = %err,
                    "failed to fetch node status; returning placeholder view"
                );
                Some(status::build_status_placeholder())
            }
        }
    }

    /// Internal helper: fetch cfg for a node id or yield a config error.
    fn cfg_for(&self, id: &str) -> Result<&NodeCfg> {
        self.nodes.get(id).ok_or_else(|| {
            Error::Config(format!(
                "unknown node id `{id}` requested for control-plane action"
            ))
        })
    }

    /// Ask a node to reload configuration.
    pub async fn reload_node(&self, id: &str) -> Result<NodeActionResponse> {
        let cfg = self.cfg_for(id)?;
        self.client.reload(cfg).await?;
        Ok(NodeActionResponse {
            node_id: id.to_string(),
            action: "reload".to_string(),
            accepted: true,
            message: None,
        })
    }

    /// Ask a node to shut down gracefully.
    pub async fn shutdown_node(&self, id: &str) -> Result<NodeActionResponse> {
        let cfg = self.cfg_for(id)?;
        self.client.shutdown(cfg).await?;
        Ok(NodeActionResponse {
            node_id: id.to_string(),
            action: "shutdown".to_string(),
            accepted: true,
            message: None,
        })
    }
}
