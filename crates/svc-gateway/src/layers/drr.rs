//! RO:WHAT   DRR/fair-queue placeholder layer.
//! RO:WHY    Slot-in for future dispatcher; currently pass-through.

use axum::http::Request;
use axum::response::Response;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use crate::observability::metrics::MetricsHandles;

#[derive(Clone)]
pub struct DrrLayer {
    pub max_inflight: usize,
    pub metrics: MetricsHandles,
}

impl<S> Layer<S> for DrrLayer {
    type Service = Drr<S>;
    fn layer(&self, inner: S) -> Self::Service {
        Drr {
            inner,
            max_inflight: self.max_inflight,
            metrics: self.metrics.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Drr<S> {
    inner: S,
    #[allow(dead_code)]
    max_inflight: usize,
    #[allow(dead_code)]
    metrics: MetricsHandles,
}

impl<S, B> Service<Request<B>> for Drr<S>
where
    S: Service<Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        // TODO: implement fair queueing; for now pass-through.
        self.inner.call(req)
    }
}
