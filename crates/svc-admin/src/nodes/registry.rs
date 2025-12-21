// crates/svc-admin/src/nodes/registry.rs
//
// RO:WHAT — In-memory registry of nodes known to svc-admin.
// RO:WHY  — Centralize node lookup, status fetching, and control-plane actions.
// RO:INTERACTS — config::NodesCfg, nodes::client::NodeClient, nodes::status, dto::node
// RO:INVARIANTS — node ids are config keys; no locks held across .await; failures degrade to placeholders (but keep correct id/display_name)
// RO:METRICS/LOGS — warns on failed upstream calls; audit logs live at router action handlers
// RO:SECURITY — registry does not grant authority; auth gates happen at router/action layer
// RO:TEST — exercised by HTTP smoke + future action tests

use crate::config::{NodeCfg, NodesCfg};
use crate::dto::node::{AdminStatusView, NodeActionResponse, NodeSummary};
use crate::dto::storage::{DatabaseDetailDto, DatabaseEntryDto, StorageSummaryDto};
use crate::error::{Error, Result};
use crate::nodes::client::NodeClient;
use crate::nodes::status;
use crate::dto::system::SystemSummaryDto;
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
    /// - On failure, log and fall back to a placeholder view **with correct identity**.
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

                let mut view = status::build_status_placeholder();
                view.id = id.to_string();
                view.display_name = cfg
                    .display_name
                    .clone()
                    .unwrap_or_else(|| id.to_string());
                view.profile = cfg.forced_profile.clone();
                Some(view)
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

    // -------------------------------------------------------------------------
    // Storage proxy (optional endpoints)
    // -------------------------------------------------------------------------
    //
    // Contract (node admin plane; future rollout):
    // - GET /api/v1/storage/summary
    // - GET /api/v1/storage/databases
    // - GET /api/v1/storage/databases/{name}
    //
    // IMPORTANT: these are optional. Missing endpoints (404/405/501) => Ok(None)
    // so the UI can fall back to mock mode cleanly.

    fn validate_db_name_for_path(name: &str) -> Result<()> {
        // Keep it strict for v1 to avoid pulling in extra URL encoding deps.
        // If you later need richer names, we can implement percent-encoding.
        let ok = name.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' )
        });

        if ok {
            Ok(())
        } else {
            Err(Error::Config(format!(
                "invalid database name for path segment: `{name}` (allowed: [A-Za-z0-9._-])"
            )))
        }
    }

    pub async fn try_storage_summary(&self, id: &str) -> Result<Option<StorageSummaryDto>> {
        let cfg = self.cfg_for(id)?;
        self.client
            .try_get_json::<StorageSummaryDto>(cfg, "/api/v1/storage/summary")
            .await
    }

    pub async fn try_storage_databases(&self, id: &str) -> Result<Option<Vec<DatabaseEntryDto>>> {
        let cfg = self.cfg_for(id)?;
        self.client
            .try_get_json::<Vec<DatabaseEntryDto>>(cfg, "/api/v1/storage/databases")
            .await
    }

    pub async fn try_storage_database_detail(
        &self,
        id: &str,
        name: &str,
    ) -> Result<Option<DatabaseDetailDto>> {
        Self::validate_db_name_for_path(name)?;
        let cfg = self.cfg_for(id)?;
        let path = format!("/api/v1/storage/databases/{name}");
        self.client
            .try_get_json::<DatabaseDetailDto>(cfg, &path)
            .await
    }
    pub async fn try_system_summary(&self, id: &str) -> Result<Option<SystemSummaryDto>> {
    let cfg = self.cfg_for(id)?;
    self.client
        .try_get_json::<SystemSummaryDto>(cfg, "/api/v1/system/summary")
        .await
    }


    // -------------------------------------------------------------------------
    // Actions
    // -------------------------------------------------------------------------

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

    /// Dev-only: proxy a synthetic crash request to the given node.
    ///
    /// This does *not* actually kill any workers; it just forwards the
    /// request to the node's `/api/v1/debug/crash` endpoint, which publishes
    /// a `KernelEvent::ServiceCrashed { service }` onto the bus. The node's
    /// supervisor then bumps restart counters.
    pub async fn debug_crash_node(
        &self,
        id: &str,
        service: Option<String>,
    ) -> Result<NodeActionResponse> {
        let cfg = self.cfg_for(id)?;

        // Delegate to the NodeClient's dev-only helper.
        self.client
            .debug_crash(cfg, service.as_deref())
            .await?;

        let action = match service.as_deref() {
            Some(svc) => format!("debug_crash({svc})"),
            None => "debug_crash".to_string(),
        };

        Ok(NodeActionResponse {
            node_id: id.to_string(),
            action,
            accepted: true,
            message: Some("debug crash forwarded to node admin plane".to_string()),
        })
    }
}
