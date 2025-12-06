// crates/svc-admin/src/nodes/client.rs

//! Node admin HTTP client.
//!
//! This is the svc-admin side of the "admin plane" contract: it knows how to
//! talk to a node's `/api/v1/status`, `/healthz`, `/readyz`, `/version` and
//! control-plane action endpoints over HTTP(S).
//!
//! Design goals for v1:
//! - Honor per-node config (`base_url`, `insecure_http`, `default_timeout`).
//! - Fail fast on obviously bad config (no scheme, http:// with insecure_http=false).
//! - Treat /healthz + /readyz as *truthful* signals: any non-2xx ⇒ error.
//! - Prefer the aggregated `/api/v1/status` endpoint when available.
//! - Be conservative about parsing: fallback to "any 2xx with non-empty body" when
//!   `/readyz` doesn't return JSON.
//! - Keep control-plane actions (reload/shutdown) thin wrappers over POST endpoints.
//!
//! Normalization into `AdminStatusView` lives in `nodes::status` and is
//! called from here; the HTTP fetching logic itself stays thin.

use crate::config::NodeCfg;
use crate::dto::node::AdminStatusView;
use crate::error::{Error, Result};
use crate::nodes::status;
use crate::nodes::status::RawStatus;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::time::Duration;
use tracing::{debug, warn};

/// Thin wrapper around a shared `reqwest::Client`.
///
/// `NodeClient` itself is stateless; per-node configuration is passed to each
/// call via `&NodeCfg`.
#[derive(Clone)]
pub struct NodeClient {
    http: Client,
}

impl Default for NodeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeClient {
    /// Construct a new `NodeClient` with a basic `reqwest::Client`.
    ///
    /// This should never fail with our current configuration; if it does, we
    /// prefer a loud panic during boot rather than silently limping along.
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent("svc-admin/0.1.0")
            .build()
            .expect("building reqwest client for NodeClient should not fail");

        Self { http }
    }

    /// Build a full URL for a given node + path.
    ///
    /// - Ensures `base_url` has a scheme (`http://` or `https://`).
    /// - Denies plain `http://` unless `insecure_http=true` is set on the node.
    fn build_url(cfg: &NodeCfg, path: &str) -> Result<String> {
        let base = cfg.base_url.trim_end_matches('/');

        if !(base.starts_with("http://") || base.starts_with("https://")) {
            return Err(Error::Config(format!(
                "node base_url must start with http:// or https:// (got {base})"
            )));
        }

        if base.starts_with("http://") && !cfg.insecure_http {
            return Err(Error::Config(format!(
                "node base_url uses http:// but insecure_http=false (base_url={base})"
            )));
        }

        Ok(format!("{base}{path}"))
    }

    /// Pick an effective timeout for this node, if any.
    ///
    /// For now we only respect the per-node default; later we can layer in
    /// service-wide defaults from `Config::server`.
    fn effective_timeout(cfg: &NodeCfg) -> Option<Duration> {
        cfg.default_timeout
    }

    fn apply_timeout(
        &self,
        cfg: &NodeCfg,
        req: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        if let Some(t) = Self::effective_timeout(cfg) {
            req.timeout(t)
        } else {
            req
        }
    }

    async fn get_text(&self, cfg: &NodeCfg, path: &str) -> Result<String> {
        let url = Self::build_url(cfg, path)?;
        let req = self.http.get(&url);
        let req = self.apply_timeout(cfg, req);

        let rsp = req.send().await?.error_for_status()?;
        Ok(rsp.text().await?)
    }

    async fn get_json<T>(&self, cfg: &NodeCfg, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = Self::build_url(cfg, path)?;
        let req = self.http.get(&url);
        let req = self.apply_timeout(cfg, req);

        let rsp = req.send().await?.error_for_status()?;
        Ok(rsp.json().await?)
    }

    async fn post_unit_action(&self, cfg: &NodeCfg, path: &str) -> Result<()> {
        let url = Self::build_url(cfg, path)?;
        let req = self.http.post(&url);
        let req = self.apply_timeout(cfg, req);

        let rsp = req.send().await?;
        let status = rsp.status();

        if !status.is_success() {
            return Err(Error::Upstream(format!(
                "POST {} → non-success status {}",
                url, status
            )));
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Public API
    // -------------------------------------------------------------------------

    /// Probe `/healthz` for a node.
    ///
    /// Any 2xx with a non-empty body is considered "healthy". Non-2xx or
    /// network errors bubble up as `Error::Http` (via `reqwest::Error`).
    pub async fn fetch_health(&self, cfg: &NodeCfg) -> Result<bool> {
        let body = self.get_text(cfg, "/healthz").await?;
        Ok(!body.trim().is_empty())
    }

    /// Probe `/readyz` for a node.
    ///
    /// Happy path: node returns `{"ready":true|false}` JSON.
    ///
    /// Fallback path: if JSON parsing fails but the HTTP status is 2xx, we do a
    /// second attempt as plain text and treat "any non-empty body" as `ready=true`.
    pub async fn fetch_ready(&self, cfg: &NodeCfg) -> Result<bool> {
        #[derive(serde::Deserialize)]
        struct ReadyDto {
            ready: bool,
        }

        match self.get_json::<ReadyDto>(cfg, "/readyz").await {
            Ok(dto) => Ok(dto.ready),
            Err(e) => {
                warn!(
                    error = %e,
                    "failed to parse /readyz as JSON, falling back to text mode"
                );
                let body = self.get_text(cfg, "/readyz").await?;
                Ok(!body.trim().is_empty())
            }
        }
    }

    /// Fetch `/version` from the node.
    ///
    /// Returns:
    /// - `Ok(Some(version))` if the node responds with a non-empty body.
    /// - `Ok(None)` on empty body or if we can't fetch/parse the version.
    ///
    /// We **log and degrade** on error here instead of failing hard: version
    /// is informational, not an SLA signal.
    pub async fn fetch_version(&self, cfg: &NodeCfg) -> Result<Option<String>> {
        match self.get_text(cfg, "/version").await {
            Ok(body) => {
                let v = body.trim();
                if v.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(v.to_string()))
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to fetch /version from node");
                Ok(None)
            }
        }
    }

    /// Combined status call used by `NodeRegistry`.
    ///
    /// Preferred path:
    /// - Call `/api/v1/status` and normalize using `nodes::status::from_raw`.
    ///
    /// Fallback path:
    /// - If `/api/v1/status` fails, probe `healthz/readyz/version` and
    ///   return a placeholder view with whatever signal we managed to get.
    pub async fn fetch_status(
        &self,
        id: &str,
        cfg: &NodeCfg,
    ) -> Result<AdminStatusView> {
        // --- Preferred: aggregated status endpoint -------------------------
        match self.get_json::<RawStatus>(cfg, "/api/v1/status").await {
            Ok(raw) => {
                let view = status::from_raw(id, cfg, raw);
                debug!(
                    node_id = id,
                    version = view.version.as_deref().unwrap_or("unknown"),
                    planes = view.planes.len(),
                    "fetched node status via /api/v1/status"
                );
                return Ok(view);
            }
            Err(err) => {
                warn!(
                    node_id = id,
                    error = %err,
                    "failed to fetch /api/v1/status, degrading to health/ready/version probes"
                );
            }
        }

        // --- Fallback: triple probe + placeholder -------------------------
        let health_ok = self.fetch_health(cfg).await.unwrap_or(false);
        let ready_ok = self.fetch_ready(cfg).await.unwrap_or(false);
        let version = self.fetch_version(cfg).await.unwrap_or(None);

        let mut view = status::build_status_placeholder();
        view.id = id.to_string();
        view.display_name = cfg
            .display_name
            .clone()
            .unwrap_or_else(|| id.to_string());
        if version.is_some() {
            view.version = version;
        }

        let status_label = if !health_ok {
            "down"
        } else if !ready_ok {
            "degraded"
        } else {
            "ready"
        };

        debug!(
            node_id = id,
            %status_label,
            ready = ready_ok,
            health = health_ok,
            version = view.version.as_deref().unwrap_or("unknown"),
            "fetched node status via degraded probes"
        );

        Ok(view)
    }

    /// Ask the node to reload its configuration.
    ///
    /// Contract:
    /// - POST /api/v1/reload
    /// - Empty request/response body.
    pub async fn reload(&self, cfg: &NodeCfg) -> Result<()> {
        self.post_unit_action(cfg, "/api/v1/reload").await
    }

    /// Ask the node to shut down gracefully.
    ///
    /// Contract:
    /// - POST /api/v1/shutdown
    /// - Empty request/response body.
    pub async fn shutdown(&self, cfg: &NodeCfg) -> Result<()> {
        self.post_unit_action(cfg, "/api/v1/shutdown").await
    }

    /// Early primitive kept for backward-compat with older experiments.
    ///
    /// For now we implement this as a no-op; new call sites should use the
    /// more explicit methods above.
    pub async fn ping_node(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    fn make_test_node_cfg(addr: SocketAddr) -> NodeCfg {
        NodeCfg {
            base_url: format!("http://{}", addr),
            display_name: Some("Test Node".to_string()),
            environment: "dev".to_string(),
            insecure_http: true,
            forced_profile: None,
            macaroon_path: None,
            default_timeout: Some(Duration::from_secs(2)),
        }
    }

    async fn start_fake_admin_plane() -> SocketAddr {
        async fn healthz() -> &'static str {
            "ok"
        }

        async fn readyz() -> &'static str {
            // Simple non-JSON body still counts as "ready" in fallback path.
            "ready"
        }

        async fn version() -> &'static str {
            "1.2.3-test"
        }

        let app = Router::new()
            .route("/healthz", get(healthz))
            .route("/readyz", get(readyz))
            .route("/version", get(version));

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test admin plane");
        let addr = listener.local_addr().expect("local_addr");

        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("serve fake admin plane");
        });

        addr
    }

    #[tokio::test(flavor = "current_thread")]
    async fn node_client_can_talk_to_fake_admin_plane() {
        let addr = start_fake_admin_plane().await;
        let cfg = make_test_node_cfg(addr);
        let client = NodeClient::new();

        let healthy = client.fetch_health(&cfg).await.expect("health");
        assert!(healthy, "healthz should report healthy");

        let ready = client.fetch_ready(&cfg).await.expect("ready");
        assert!(ready, "readyz should be treated as ready via fallback");

        let version = client
            .fetch_version(&cfg)
            .await
            .expect("version fetch")
            .expect("version should be present");
        assert_eq!(version, "1.2.3-test");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn node_client_rejects_http_when_insecure_http_false() {
        let cfg = NodeCfg {
            base_url: "http://127.0.0.1:12345".to_string(),
            display_name: None,
            environment: "dev".to_string(),
            insecure_http: false,
            forced_profile: None,
            macaroon_path: None,
            default_timeout: None,
        };

        let client = NodeClient::new();
        let err = client.fetch_health(&cfg).await.unwrap_err();

        let msg = format!("{err}");
        assert!(
            msg.contains("insecure_http=false"),
            "expected config error about insecure_http, got: {msg}"
        );
    }
}
