// crates/micronode/src/layers/security.rs
//! RO:WHAT — Security ingress layer that extracts raw macaroons into request extensions.
//! RO:WHY  — Give handlers and facets a cheap way to see caller capabilities later.
//! RO:INTERACTS — Uses `security::auth_macaroon::extract_raw_macaroon` and `RawMacaroon`.
//! RO:INVARIANTS — Never rejects requests on its own and at most one RawMacaroon per request.
//! RO:TEST — Indirectly exercised by HTTP integration tests once auth is enforced.

use crate::security::auth_macaroon::{extract_raw_macaroon, RawMacaroon};
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
    task::{Context, Poll},
};
use tower::{Layer, Service};

type BoxFut = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send + 'static>>;

/// Layer type used in app.rs.
#[derive(Clone, Default)]
pub struct SecurityLayer;

impl SecurityLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for SecurityLayer {
    type Service = SecurityService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityService { inner }
    }
}

#[derive(Clone)]
pub struct SecurityService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for SecurityService<S>
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

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();

        // Extract a RawMacaroon (if present) and park it in extensions.
        if let Some(mac) = extract_raw_macaroon(&req) {
            req.extensions_mut().insert::<RawMacaroon>(mac);
        }

        Box::pin(async move {
            Ok(inner.call(req).await.unwrap_or_else(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
            }))
        })
    }
}
