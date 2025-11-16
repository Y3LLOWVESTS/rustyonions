//! RO:WHAT — Minimal HTTP metrics middleware (request count + latency).
//! RO:WHY  — Golden metrics parity across services, with stable metric names:
//!           `micronode_http_requests_total` and `micronode_request_latency_seconds`.
//!
//! RO:INVARIANTS —
//!   - Never propagates errors (Error = Infallible).
//!   - Records a 500 status in metrics if the inner service errors.
//!   - No locks held across `.await`.
//!
//! RO:METRICS —
//!   - micronode_http_requests_total{method,route,status}
//!   - micronode_request_latency_seconds
//!
//! RO:CONFIG — Transparent; router decides which routes are wrapped.

use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::OnceLock,
    task::{Context, Poll},
    time::Instant,
};

use axum::{body::Body, http::Request, response::Response};
use prometheus::{Histogram, HistogramOpts, IntCounterVec, Opts};
use tower::{Layer, Service};
use tracing::error;

// --- Static metric families (initialized on first use) ---

static REQS: OnceLock<IntCounterVec> = OnceLock::new();
static LAT: OnceLock<Histogram> = OnceLock::new();

fn reqs() -> &'static IntCounterVec {
    REQS.get_or_init(|| {
        let opts = Opts::new(
            "micronode_http_requests_total",
            "Total HTTP requests processed by Micronode",
        );
        let vec = IntCounterVec::new(opts, &["method", "route", "status"])
            .expect("construct micronode_http_requests_total");

        prometheus::default_registry()
            .register(Box::new(vec.clone()))
            .expect("register micronode_http_requests_total");

        vec
    })
}

fn lat() -> &'static Histogram {
    LAT.get_or_init(|| {
        let opts = HistogramOpts::new(
            "micronode_request_latency_seconds",
            "HTTP request latency observed by Micronode",
        )
        .buckets(prometheus::DEFAULT_BUCKETS.to_vec());

        let hist = Histogram::with_opts(opts).expect("construct micronode_request_latency_seconds");

        prometheus::default_registry()
            .register(Box::new(hist.clone()))
            .expect("register micronode_request_latency_seconds");

        hist
    })
}

// --- Tower Layer implementation ---

#[derive(Clone, Default)]
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
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.inner.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => {
                // For foundation cut, we log readiness errors but don't propagate them.
                error!("HttpMetrics inner not ready: {e}");
                Poll::Ready(Ok(()))
            }
        }
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let method = req.method().as_str().to_owned();
        let route = req.uri().path().to_owned();
        let start = Instant::now();

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let result = inner.call(req).await;
            let elapsed = start.elapsed().as_secs_f64();

            match result {
                Ok(resp) => {
                    let status = resp.status().as_u16().to_string();
                    reqs().with_label_values(&[&method, &route, &status]).inc();
                    lat().observe(elapsed);
                    Ok(resp)
                }
                Err(e) => {
                    error!("handler error in HttpMetrics: {e}");
                    // Record as a 500 in metrics.
                    let status = String::from("500");
                    reqs().with_label_values(&[&method, &route, &status]).inc();
                    lat().observe(elapsed);

                    let resp = Response::builder()
                        .status(500)
                        .body(Body::from("internal error"))
                        .expect("build 500 response in HttpMetrics");

                    Ok(resp)
                }
            }
        })
    }
}

/// Convenience constructor so app.rs can stay clean.
pub fn layer() -> HttpMetricsLayer {
    HttpMetricsLayer
}

/// Prewarm so the metric families are registered and visible at `/metrics`
/// even before the first real request hits.
pub fn prewarm() {
    let _ = reqs();
    let _ = lat();
}
