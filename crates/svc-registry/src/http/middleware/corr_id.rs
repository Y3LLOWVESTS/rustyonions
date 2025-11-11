//! Correlation ID middleware (generates if missing).
use axum::{http::Request, response::Response};
use std::task::{Context, Poll};
use tower::{Layer, Service};
use ulid::Ulid;

#[derive(Clone, Default)]
pub struct CorrLayer;

impl CorrLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for CorrLayer {
    type Service = CorrSvc<S>;
    fn layer(&self, inner: S) -> Self::Service {
        CorrSvc { inner }
    }
}

#[derive(Clone)]
pub struct CorrSvc<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for CorrSvc<S>
where
    S: Service<Request<B>, Response = Response> + Clone,
{
    type Response = Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // delegate readiness directly; no pinning needed
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        const HDR: &str = "X-Corr-ID";
        if !req.headers().contains_key(HDR) {
            let id = Ulid::new().to_string();
            if let Ok(val) = http::HeaderValue::from_str(&id) {
                req.headers_mut().insert(HDR, val);
            }
        }
        self.inner.call(req)
    }
}
