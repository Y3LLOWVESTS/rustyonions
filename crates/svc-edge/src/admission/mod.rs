//! Admission chain (custom minimal layer) for the API plane.
//!
//! RO:WHAT
//! - Enforce **timeout** and **inflight cap** with a tiny, cloneable Tower `Layer`.
//!
//! RO:WHY
//! - Keeps the service `Clone + Send + 'static` with `Error = Infallible` to satisfy
//!   `Router::route_layer` bounds in axum 0.7, avoiding brittle combinator stacks.
//! - No `http_body`/`bytes` deps; no body-type churn.
//!
//! RO:INVARIANTS
//! - No locks across `.await`; uses `Arc<Semaphore>`.
//! - Deterministic rejections (408/503); no ambiguous 500s from the layer itself.
//!
//! RO:METRICS
//! - Ticks `edge_rejects_total{reason}` on 408 (timeout) and 503 (busy).
//!
//! RO:CONFIG
//! - Prefer `apply_with(router, timeout, max_inflight)` for env/Config-driven caps.
//! - `apply_defaults(...)` kept for dev convenience.

use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use axum::{
    response::{IntoResponse, Response},
    Json, Router,
};
use http::{Request, StatusCode};
use serde_json::json;
use tokio::sync::Semaphore;
use tower::{Layer, Service};

/// Default tunables (dev convenience).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_MAX_INFLIGHT: usize = 256;

/// Apply admission guards with dev defaults.
pub fn apply_defaults<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    apply_with(router, DEFAULT_TIMEOUT, DEFAULT_MAX_INFLIGHT)
}

/// Apply admission guards with explicit caps (Config/env-friendly).
pub fn apply_with<S>(router: Router<S>, timeout: Duration, max_inflight: usize) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let layer = AdmissionLayer::new(AdmissionConfig {
        timeout,
        max_inflight,
    });
    router.route_layer(layer)
}

/// Configuration for the admission guard.
#[derive(Debug, Clone, Copy)]
pub struct AdmissionConfig {
    /// Per-request wall-clock timeout (e.g., 5s → 408 on expiry).
    pub timeout: Duration,
    /// Maximum number of inflight requests (beyond this → 503).
    pub max_inflight: usize,
}

/// Layer that installs [`AdmissionService`] on the router.
#[derive(Clone)]
pub struct AdmissionLayer {
    cfg: AdmissionConfig,
}

impl AdmissionLayer {
    /// Build a new admission layer with the given config.
    pub fn new(cfg: AdmissionConfig) -> Self {
        Self { cfg }
    }
}

impl<S> Layer<S> for AdmissionLayer {
    type Service = AdmissionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AdmissionService::new(inner, self.cfg)
    }
}

/// Service wrapper that enforces timeout and inflight limits.
#[derive(Clone)]
pub struct AdmissionService<S> {
    inner: S,
    cfg: AdmissionConfig,
    inflight: Arc<Semaphore>,
}

impl<S> AdmissionService<S> {
    fn new(inner: S, cfg: AdmissionConfig) -> Self {
        Self {
            inner,
            cfg,
            inflight: Arc::new(Semaphore::new(cfg.max_inflight)),
        }
    }
}

impl<S, B> Service<Request<B>> for AdmissionService<S>
where
    S: Service<Request<B>, Error = Infallible> + Clone + Send + 'static,
    S::Response: IntoResponse + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    // Use axum::Response so we can build responses from any IntoResponse.
    type Response = Response;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Error is already Infallible; just forward it.
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let cfg = self.cfg;
        let mut inner = self.inner.clone();
        let inflight = Arc::clone(&self.inflight);

        Box::pin(async move {
            // Inflight cap → 503 when saturated.
            let _permit = match inflight.try_acquire() {
                Ok(p) => p,
                Err(_) => {
                    crate::metrics::inc_reject("busy");
                    let body = Json(json!({ "ok": false, "reason": "busy" }));
                    return Ok((StatusCode::SERVICE_UNAVAILABLE, body).into_response());
                }
            };

            // Timeout → 408 when exceeded.
            match tokio::time::timeout(cfg.timeout, inner.call(req)).await {
                Ok(Ok(resp)) => Ok(resp.into_response()),
                Ok(Err(_)) => {
                    // Inner service promised Infallible; this should be unreachable.
                    let body = Json(json!({ "ok": false, "reason": "inner_error" }));
                    Ok((StatusCode::INTERNAL_SERVER_ERROR, body).into_response())
                }
                Err(_) => {
                    crate::metrics::inc_reject("timeout");
                    let body = Json(json!({ "ok": false, "reason": "timeout" }));
                    Ok((StatusCode::REQUEST_TIMEOUT, body).into_response())
                }
            }
        })
    }
}
