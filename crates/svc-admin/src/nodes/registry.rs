use crate::config::{NodeCfg, NodesCfg};
use crate::dto::node::{AdminStatusView, NodeSummary};
use crate::nodes::status;
use std::collections::BTreeMap;

/// In-memory registry of configured nodes.
///
/// This is a thin wrapper around the loaded NodesCfg plus a few helpers to
/// expose summaries and status views to handlers.
#[derive(Clone)]
pub struct NodeRegistry {
    nodes: BTreeMap<String, NodeCfg>,
}

impl NodeRegistry {
    pub fn new(cfg: &NodesCfg) -> Self {
        Self {
            nodes: cfg.clone(),
        }
    }

    /// Return summaries of all configured nodes, in a stable order.
    pub fn list_summaries(&self) -> Vec<NodeSummary> {
        self.nodes
            .iter()
            .map(|(id, cfg)| NodeSummary {
                id: id.clone(),
                display_name: cfg
                    .display_name
                    .clone()
                    .unwrap_or_else(|| id.clone()),
                // Profile will come from real node status once we plumb it through.
                profile: None,
            })
            .collect()
    }

    /// Look up a node by id and build an AdminStatusView for it.
    ///
    /// For now this uses a placeholder normalizer; later it will call NodeClient
    /// and hydrate real status/metrics from the node.
    pub async fn get_status(&self, id: &str) -> Option<AdminStatusView> {
        let cfg = self.nodes.get(id)?;
        let mut view = status::build_status_placeholder();
        view.id = id.to_string();
        view.display_name = cfg
            .display_name
            .clone()
            .unwrap_or_else(|| id.to_string());
        Some(view)
    }

    /// Returns true if a node with this id exists in the registry.
    pub fn contains(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }
}
