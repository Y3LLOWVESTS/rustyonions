// crates/svc-admin/src/nodes/registry.rs

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::config::{NodeCfg, NodesCfg};
use crate::dto::bench::{BenchRunReqDto, BenchRunRespDto, BenchRunResultDto, BenchRunStatusDto};
use crate::dto::node::{AdminStatusView, NodeActionResponse, NodeSummary};
use crate::dto::storage::{DatabaseDetailDto, DatabaseEntryDto, StorageSummaryDto};
use crate::dto::system::SystemSummaryDto;
use crate::error::{Error, Result};
use crate::nodes::client::NodeClient;
use crate::nodes::status;

#[derive(Clone)]
pub struct NodeRegistry {
    nodes: Arc<BTreeMap<String, NodeCfg>>,
    client: NodeClient,
}

impl NodeRegistry {
    pub fn new(cfg: &NodesCfg) -> Self {
        Self {
            nodes: Arc::new(cfg.clone()),
            client: NodeClient::new(),
        }
    }

    pub fn list_summaries(&self) -> Vec<NodeSummary> {
        self.nodes
            .iter()
            .map(|(id, cfg)| NodeSummary {
                id: id.clone(),
                display_name: cfg.display_name.clone().unwrap_or_else(|| id.clone()),
                profile: cfg.forced_profile.clone(),
            })
            .collect()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

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
                view.display_name = cfg.display_name.clone().unwrap_or_else(|| id.to_string());
                view.profile = cfg.forced_profile.clone();
                Some(view)
            }
        }
    }

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

    fn validate_db_name_for_path(name: &str) -> Result<()> {
        let ok = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));

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

    // -------------------------------------------------------------------------
    // System summary proxy (optional endpoint)
    // -------------------------------------------------------------------------

    pub async fn try_system_summary(&self, id: &str) -> Result<Option<SystemSummaryDto>> {
        let cfg = self.cfg_for(id)?;
        self.client
            .try_get_json::<SystemSummaryDto>(cfg, "/api/v1/system/summary")
            .await
    }

    // -------------------------------------------------------------------------
    // Network accounting proxy (optional endpoint)
    // -------------------------------------------------------------------------

    /// Proxy `/api/v1/system/net/accounting` from the node admin plane.
    ///
    /// Capability rollout posture:
    /// - (404/405/501) => Ok(None)
    ///
    /// We return `serde_json::Value` intentionally so svc-admin does not become
    /// the "schema owner" while the node-side DTO stabilizes.
    pub async fn try_system_net_accounting(
        &self,
        id: &str,
    ) -> Result<Option<serde_json::Value>> {
        let cfg = self.cfg_for(id)?;
        self.client
            .try_get_json::<serde_json::Value>(cfg, "/api/v1/system/net/accounting")
            .await
    }

    // -------------------------------------------------------------------------
    // Benchmarks proxy (optional endpoints)
    // -------------------------------------------------------------------------

    fn validate_run_id_for_path(run_id: &str) -> Result<()> {
        if run_id.is_empty() || run_id.len() > 128 {
            return Err(Error::Config(format!(
                "invalid run_id length for path segment: `{run_id}`"
            )));
        }

        let ok = run_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));

        if ok {
            Ok(())
        } else {
            Err(Error::Config(format!(
                "invalid run_id for path segment: `{run_id}` (allowed: [A-Za-z0-9_-])"
            )))
        }
    }

    pub async fn try_bench_run(
        &self,
        id: &str,
        req: &BenchRunReqDto,
    ) -> Result<Option<BenchRunRespDto>> {
        let cfg = self.cfg_for(id)?;
        self.client
            .try_post_json::<BenchRunReqDto, BenchRunRespDto>(cfg, "/api/v1/bench/run", req)
            .await
    }

    /// 404 is *semantic* here (run_id not found), not “capability missing”.
    pub async fn try_bench_status(
        &self,
        id: &str,
        run_id: &str,
    ) -> Result<Option<BenchRunStatusDto>> {
        Self::validate_run_id_for_path(run_id)?;
        let cfg = self.cfg_for(id)?;
        let path = format!("/api/v1/bench/runs/{run_id}");
        self.client
            .try_get_json_no_404::<BenchRunStatusDto>(cfg, &path)
            .await
    }

    /// 404 is *semantic* here (run_id not found), not “capability missing”.
    pub async fn try_bench_result(
        &self,
        id: &str,
        run_id: &str,
    ) -> Result<Option<BenchRunResultDto>> {
        Self::validate_run_id_for_path(run_id)?;
        let cfg = self.cfg_for(id)?;
        let path = format!("/api/v1/bench/runs/{run_id}/result");
        self.client
            .try_get_json_no_404::<BenchRunResultDto>(cfg, &path)
            .await
    }

    // -------------------------------------------------------------------------
    // Actions
    // -------------------------------------------------------------------------

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

    pub async fn debug_crash_node(
        &self,
        id: &str,
        service: Option<String>,
    ) -> Result<NodeActionResponse> {
        let cfg = self.cfg_for(id)?;
        self.client.debug_crash(cfg, service.as_deref()).await?;

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
