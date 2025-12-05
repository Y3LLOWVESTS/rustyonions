// crates/svc-admin/src/nodes/registry.rs

//! In-memory node registry backed by config, with helper methods
//! for listing nodes and fetching their status via `NodeClient`.

use crate::config::{NodeCfg, NodesCfg};
use crate::dto::node::{AdminStatusView, NodeSummary};
use crate::nodes::client::NodeClient;
use crate::nodes::status;
use std::collections::BTreeMap;
use std::sync::Arc;

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
                // We don’t know profile yet; leave as None/null for now.
                profile: None,
            })
            .collect()
    }

    /// Return the status view for a given node id, if present.
    ///
    /// This will:
    /// - Look up the node in the config registry.
    /// - Call `NodeClient::fetch_status` to do real HTTP calls.
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
                    "failed to fetch status from node; returning placeholder"
                );

                let mut view = status::build_status_placeholder();
                view.id = id.to_string();
                view.display_name = cfg
                    .display_name
                    .clone()
                    .unwrap_or_else(|| id.to_string());
                Some(view)
            }
        }
    }

    /// Simple existence check, useful for pre-validating IDs.
    pub fn contains(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }
}
