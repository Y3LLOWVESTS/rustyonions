// crates/svc-admin/src/nodes/client.rs

//! RO:WHAT — Node admin HTTP client used by svc-admin to talk to node admin planes.
//! RO:WHY  — Centralize per-node HTTP rules (timeouts, http/https policy, optional endpoints).
//! RO:INTERACTS — crate::config::NodeCfg, crate::nodes::status, dto::node::AdminStatusView
//! RO:INVARIANTS — deny http:// unless insecure_http=true; no lock across .await; optional endpoints may be missing (404/405/501)
//! RO:METRICS/LOGS — logs degraded paths and upstream failures (no direct metrics)
//! RO:SECURITY — does not inject creds yet; node enforces its own auth/dev gates
//! RO:TEST — unit tests in this module

use crate::config::NodeCfg;
use crate::dto::node::AdminStatusView;
use crate::error::{Error, Result};
use crate::nodes::status::{self, RawStatus};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, warn};

/// How to interpret HTTP status codes for "optional endpoint" calls.
///
/// We historically treated 404/405/501 as “missing endpoint” to support gradual rollout.
/// That works well for capability endpoints like `/api/v1/system/summary`, but it can be
/// wrong for endpoints where 404 may be a *real* semantic (e.g. "run_id not found").
///
/// Default behavior preserves the legacy rollout posture.
#[derive(Debug, Clone, Copy)]
enum MissingPolicy {
    /// Treat 404/405/501 as “missing endpoint”.
    Treat404AsMissing,

    /// Treat only 405/501 as “missing endpoint” (404 is a real error).
    ///
    /// Use this when the path itself should exist, and 404 indicates a real
    /// domain condition rather than “capability absent”.
    Treat404AsError,
}

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
            .user_agent(concat!("svc-admin/", env!("CARGO_PKG_VERSION")))
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

    fn is_missing_endpoint_status(policy: MissingPolicy, status: reqwest::StatusCode) -> bool {
        match policy {
            MissingPolicy::Treat404AsMissing => {
                status == reqwest::StatusCode::NOT_FOUND
                    || status == reqwest::StatusCode::METHOD_NOT_ALLOWED
                    || status == reqwest::StatusCode::NOT_IMPLEMENTED
            }
            MissingPolicy::Treat404AsError => {
                status == reqwest::StatusCode::METHOD_NOT_ALLOWED
                    || status == reqwest::StatusCode::NOT_IMPLEMENTED
            }
        }
    }

    fn truncate_snippet(s: &str, max_chars: usize) -> String {
        if s.chars().count() <= max_chars {
            return s.to_string();
        }
        let mut out = String::with_capacity(max_chars + 1);
        for (i, ch) in s.chars().enumerate() {
            if i >= max_chars {
                out.push('…');
                break;
            }
            out.push(ch);
        }
        out
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

    /// Like `get_json`, but treats some statuses as “missing endpoint” and returns `Ok(None)`.
    ///
    /// IMPORTANT:
    /// - For non-missing non-2xx statuses, we return `Error::UpstreamStatus{status,..}` so
    ///   router layers can correctly preserve semantics (e.g., bench run_id 404).
    async fn get_json_optional_with_policy<T>(
        &self,
        cfg: &NodeCfg,
        path: &str,
        policy: MissingPolicy,
    ) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let url = Self::build_url(cfg, path)?;
        let req = self.http.get(&url);
        let req = self.apply_timeout(cfg, req);

        let rsp = req.send().await?;
        let status = rsp.status();

        if Self::is_missing_endpoint_status(policy, status) {
            return Ok(None);
        }

        if !status.is_success() {
            // best-effort body capture for diagnostics
            let body = match rsp.text().await {
                Ok(t) => Self::truncate_snippet(t.trim(), 400),
                Err(_) => "<unreadable body>".to_string(),
            };

            return Err(Error::UpstreamStatus {
                status: status.as_u16(),
                message: format!("GET {url} → status {status}; body: {body}"),
            });
        }

        Ok(Some(rsp.json().await?))
    }

    /// Legacy optional GET behavior: treat (404/405/501) as “missing endpoint”.
    async fn get_json_optional<T>(&self, cfg: &NodeCfg, path: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.get_json_optional_with_policy(cfg, path, MissingPolicy::Treat404AsMissing)
            .await
    }

    async fn post_unit_action(&self, cfg: &NodeCfg, path: &str) -> Result<()> {
        let url = Self::build_url(cfg, path)?;
        let req = self.http.post(&url);
        let req = self.apply_timeout(cfg, req);

        let rsp = req.send().await?;
        let status = rsp.status();

        if !status.is_success() {
            // Preserve status + best-effort body for audit/debuggability.
            let body = match rsp.text().await {
                Ok(t) => Self::truncate_snippet(t.trim(), 400),
                Err(_) => "<unreadable body>".to_string(),
            };

            return Err(Error::UpstreamStatus {
                status: status.as_u16(),
                message: format!("POST {url} → status {status}; body: {body}"),
            });
        }

        Ok(())
    }

    /// Strict JSON POST helper (non-optional).
    ///
    /// This is intentionally kept for future “must exist” endpoints. Most of the
    /// current svc-admin → node calls are *capability rollout* endpoints and
    /// should use `try_post_json*` instead.
    #[allow(dead_code)]
    async fn post_json<TReq, TResp>(&self, cfg: &NodeCfg, path: &str, body: &TReq) -> Result<TResp>
    where
        TReq: Serialize + ?Sized,
        TResp: DeserializeOwned,
    {
        let url = Self::build_url(cfg, path)?;
        let req = self.http.post(&url).json(body);
        let req = self.apply_timeout(cfg, req);

        let rsp = req.send().await?.error_for_status()?;
        Ok(rsp.json().await?)
    }

    async fn post_json_optional_with_policy<TReq, TResp>(
        &self,
        cfg: &NodeCfg,
        path: &str,
        body: &TReq,
        policy: MissingPolicy,
    ) -> Result<Option<TResp>>
    where
        TReq: Serialize + ?Sized,
        TResp: DeserializeOwned,
    {
        let url = Self::build_url(cfg, path)?;
        let req = self.http.post(&url).json(body);
        let req = self.apply_timeout(cfg, req);

        let rsp = req.send().await?;
        let status = rsp.status();

        if Self::is_missing_endpoint_status(policy, status) {
            return Ok(None);
        }

        if !status.is_success() {
            let body = match rsp.text().await {
                Ok(t) => Self::truncate_snippet(t.trim(), 400),
                Err(_) => "<unreadable body>".to_string(),
            };

            return Err(Error::UpstreamStatus {
                status: status.as_u16(),
                message: format!("POST {url} → status {status}; body: {body}"),
            });
        }

        Ok(Some(rsp.json().await?))
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
    pub async fn fetch_status(&self, id: &str, cfg: &NodeCfg) -> Result<AdminStatusView> {
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

        let health_ok = self.fetch_health(cfg).await.unwrap_or(false);
        let ready_ok = self.fetch_ready(cfg).await.unwrap_or(false);
        let version = self.fetch_version(cfg).await.unwrap_or(None);

        let mut view = status::build_status_placeholder();
        view.id = id.to_string();
        view.display_name = cfg.display_name.clone().unwrap_or_else(|| id.to_string());
        view.profile = cfg.forced_profile.clone();
        view.version = version;

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

    /// Optional helper for gradual rollout endpoints.
    ///
    /// Default behavior preserves legacy rollout posture:
    /// - (404/405/501) => Ok(None)
    pub async fn try_get_json<T>(&self, cfg: &NodeCfg, path: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.get_json_optional(cfg, path).await
    }

    /// Optional helper for endpoints where 404 is meaningful (not “capability absent”).
    ///
    /// - (405/501) => Ok(None)
    /// - 404 => Err(UpstreamStatus{status:404,...})
    pub async fn try_get_json_no_404<T>(&self, cfg: &NodeCfg, path: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.get_json_optional_with_policy(cfg, path, MissingPolicy::Treat404AsError)
            .await
    }

    /// Optional POST helper for gradual rollout endpoints (JSON request/response).
    ///
    /// Default behavior:
    /// - (404/405/501) => Ok(None)
    pub async fn try_post_json<TReq, TResp>(
        &self,
        cfg: &NodeCfg,
        path: &str,
        body: &TReq,
    ) -> Result<Option<TResp>>
    where
        TReq: Serialize + ?Sized,
        TResp: DeserializeOwned,
    {
        self.post_json_optional_with_policy(cfg, path, body, MissingPolicy::Treat404AsMissing)
            .await
    }

    /// Optional POST helper where 404 is meaningful.
    ///
    /// - (405/501) => Ok(None)
    /// - 404 => Err(UpstreamStatus{status:404,...})
    pub async fn try_post_json_no_404<TReq, TResp>(
        &self,
        cfg: &NodeCfg,
        path: &str,
        body: &TReq,
    ) -> Result<Option<TResp>>
    where
        TReq: Serialize + ?Sized,
        TResp: DeserializeOwned,
    {
        self.post_json_optional_with_policy(cfg, path, body, MissingPolicy::Treat404AsError)
            .await
    }

    pub async fn reload(&self, cfg: &NodeCfg) -> Result<()> {
        self.post_unit_action(cfg, "/api/v1/reload").await
    }

    pub async fn shutdown(&self, cfg: &NodeCfg) -> Result<()> {
        self.post_unit_action(cfg, "/api/v1/shutdown").await
    }

    pub async fn debug_crash(&self, cfg: &NodeCfg, service: Option<&str>) -> Result<()> {
        let mut path = String::from("/api/v1/debug/crash");
        if let Some(svc) = service {
            path.push_str("?service=");
            path.push_str(svc);
        }

        self.post_unit_action(cfg, &path).await
    }

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
