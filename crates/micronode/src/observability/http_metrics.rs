//! RO:WHAT — Minimal HTTP metrics middleware (request count + latency).
//! RO:WHY  — Golden metrics parity across services.
//! RO:INVARIANTS — Prewarm labels later; stable names: micronode_http_*.

use axum::{body::Body, http::Request};
use once_cell::sync::Lazy;
use prometheus::{Histogram, HistogramOpts, IntCounterVec, Opts, Registry};
use std::time::Instant;
use tower::Layer;
use tower::{BoxError, Service};
use tracing::error;

static REGISTRY: Lazy<Registry> = Lazy::new(prometheus::default_registry);
static REQS: Lazy<IntCounterVec> = Lazy::new(|| {
    let o = Opts::new("micronode_http_requests_total", "HTTP requests");
    let v = IntCounterVec::new(o, &["method", "route", "status"]).unwrap();
    REGISTRY.register(Box::new(v.clone())).ok();
    v
});
static LAT: Lazy<Histogram> = Lazy::new(|| {
    let o = HistogramOpts::new("micronode_request_latency_seconds", "Request latency");
    let h = Histogram::with_opts(o).unwrap();
    REGISTRY.register(Box::new(h.clone())).ok();
    h
});

#[derive(Clone)]
pub struct HttpMetricsLayer;

impl<S> Layer<S> for HttpMetricsLayer {
    type Service = HttpMetrics<S>;
    fn layer(&self, inner: S) -> Self::Service {
        HttpMetrics { inner }
    }
}

#[derive(Clone)]
pub struct HttpMetrics<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for HttpMetrics<S>
where
    S: Service<Request<Body>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<BoxError>,
{
    type Response = axum::response::Response;
    type Error = BoxError;
    type Future = tokio::task::JoinHandle<Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        futures::ready!(tokio::task::block_in_place(|| std::task::Poll::Ready(Ok(()))));
        // Delegate readiness directly if inner has it:
        // But to avoid trait bounds complexity here, we assume OK (foundation).
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let method = req.method().as_str().to_owned();
        // Route label best-effort: use URI path; real router labels later.
        let route = req.uri().path().to_owned();
        let start = Instant::now();

        let mut inner = self.inner.clone();
        tokio::spawn(async move {
            let resp = inner.call(req).await.map_err(|e| e.into()).map_err(|e: BoxError| {
                error!("handler error: {e}");
                e
            })?;
            let status = resp.status().as_u16().to_string();
            REQS.with_label_values(&[&method, &route, &status]).inc();
            LAT.observe(start.elapsed().as_secs_f64());
            Ok(resp)
        })
    }
}

pub fn layer() -> HttpMetricsLayer {
    HttpMetricsLayer
}
