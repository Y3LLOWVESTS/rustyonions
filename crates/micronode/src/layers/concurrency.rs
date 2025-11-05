//! RO:WHAT — Per-route non-blocking concurrency cap (429 when saturated).
//! RO:WHY  — Shed load early without stalling worker threads.
//! RO:AXUM — Tower Layer to satisfy Axum 0.7 trait bounds.

use axum::{
    body::Body,
    http::Request,
    response::{IntoResponse, Response},
};
use http::StatusCode;
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::Semaphore;
use tower::{Layer, Service};

type BoxFut = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send + 'static>>;

#[derive(Clone)]
pub struct ConcurrencyLayer {
    sema: Arc<Semaphore>,
}

impl ConcurrencyLayer {
    pub fn new(sema: Arc<Semaphore>) -> Self {
        Self { sema }
    }
}

impl<S> Layer<S> for ConcurrencyLayer {
    type Service = ConcurrencyService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ConcurrencyService { inner, sema: self.sema.clone() }
    }
}

#[derive(Clone)]
pub struct ConcurrencyService<S> {
    inner: S,
    sema: Arc<Semaphore>,
}

impl<S> Service<Request<Body>> for ConcurrencyService<S>
where
    S: Service<Request<Body>, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = BoxFut;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.inner.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(_)) => Poll::Ready(Ok(())),
        }
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();

        // Use OWNED permit so it is 'static-safe inside the async block.
        match self.sema.clone().try_acquire_owned() {
            Ok(permit) => Box::pin(async move {
                let resp = inner.call(req).await?;
                drop(permit); // explicit for clarity
                Ok(resp)
            }),
            Err(_) => Box::pin(async move {
                Ok((StatusCode::TOO_MANY_REQUESTS, "concurrency limit exceeded").into_response())
            }),
        }
    }
}
