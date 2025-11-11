//! Per-route Prometheus metrics middleware.
//! Labels: method, route, status. Also records a per-route latency histogram.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use axum::{http::Request, response::Response};
use tower::{Layer, Service};

use crate::observability::metrics::RegistryMetrics;

#[derive(Clone)]
pub struct MetricsLayer {
    metrics: RegistryMetrics,
}

impl MetricsLayer {
    pub fn new(metrics: RegistryMetrics) -> Self {
        Self { metrics }
    }
}

#[derive(Clone)]
pub struct MetricsSvc<S> {
    inner: S,
    metrics: RegistryMetrics,
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsSvc<S>;
    fn layer(&self, inner: S) -> Self::Service {
        MetricsSvc {
            inner,
            metrics: self.metrics.clone(),
        }
    }
}

impl<S, B> Service<Request<B>> for MetricsSvc<S>
where
    S: Service<Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        // Capture route and method now; status is read from the response.
        let method = req.method().as_str().to_string();
        // Axum puts the matched path in an extension; fall back to "unmatched".
        let route = req
            .extensions()
            .get::<axum::extract::MatchedPath>()
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| "unmatched".to_string());

        let metrics = self.metrics.clone();
        let start = Instant::now();

        let fut = self.inner.call(req);
        Box::pin(async move {
            let resp = fut.await?;
            let status_s = resp.status().as_u16().to_string();

            // Counter (labels: method, route, status) — matches NOTES and RegistryMetrics
            metrics
                .requests_total
                .with_label_values(&[&method, &route, &status_s])
                .inc();

            // Histogram (per route)
            metrics
                .request_latency_seconds
                .with_label_values(&[&route])
                .observe(start.elapsed().as_secs_f64());

            Ok(resp)
        })
    }
}
