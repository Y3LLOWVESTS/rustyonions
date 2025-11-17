// crates/micronode/src/layers/security.rs
//! RO:WHAT — Security ingress layers.
//!
//! 1) `SecurityLayer`: extract raw macaroons into request extensions.
//! 2) `RequireAuthLayer`: enforce a simple, config-driven policy (MVP).
//!
//! RO:WHY — Keep handlers simple; centralize capability plumbing and gating.
//!
//! RO:INTERACTS — Uses `security::auth_macaroon::extract_raw_macaroon` and `RawMacaroon`.
//!
//! RO:INVARIANTS —
//! - Extraction never rejects and never logs token contents.
//! - Enforcement is deny-by-default unless `security.mode = "dev_allow"`.
//! - For now, `external` behaves like `deny_all` (until ron-auth wiring).
//!
//! RO:TEST — Covered by `tests/auth_gate.rs` and existing HTTP integration tests.

use crate::config::schema::SecurityMode;
use crate::security::auth_macaroon::{extract_raw_macaroon, RawMacaroon};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    response::{IntoResponse, Response},
};
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

type BoxFut = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send + 'static>>;

// ===============================
// 1) Extraction: SecurityLayer
// ===============================

/// Layer type used in app.rs for capability extraction.
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
            // Hide inner readiness errors behind 500 in call()
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

// ===============================
// 2) Enforcement: RequireAuthLayer
// ===============================

/// Policy enforcement layer. Deny-by-default unless `DevAllow` is set.
#[derive(Clone, Copy)]
pub struct RequireAuthLayer {
    mode: SecurityMode,
}

impl RequireAuthLayer {
    pub fn new(mode: SecurityMode) -> Self {
        Self { mode }
    }
}

impl<S> Layer<S> for RequireAuthLayer {
    type Service = RequireAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequireAuthService { inner, mode: self.mode }
    }
}

#[derive(Clone)]
pub struct RequireAuthService<S> {
    inner: S,
    mode: SecurityMode,
}

impl<S> Service<Request<Body>> for RequireAuthService<S>
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
            // Hide inner readiness errors behind 500 in call()
            Poll::Ready(Err(_)) => Poll::Ready(Ok(())),
        }
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let mode = self.mode;

        Box::pin(async move {
            match mode {
                SecurityMode::DevAllow => {
                    // DX-friendly: allow without header.
                    Ok(inner.call(req).await.unwrap_or_else(|_| {
                        (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
                    }))
                }
                SecurityMode::DenyAll | SecurityMode::External => {
                    // External behaves like deny_all until wired to ron-auth.
                    let has_mac = req.extensions().get::<RawMacaroon>().is_some();

                    if !has_mac {
                        // 401 + WWW-Authenticate: Macro realm="micronode"
                        let resp = (
                            StatusCode::UNAUTHORIZED,
                            [(header::WWW_AUTHENTICATE, r#"Macro realm="micronode""#)],
                            "missing capability macaroon",
                        )
                            .into_response();
                        Ok(resp)
                    } else {
                        // Present but not allowed by current policy.
                        Ok((StatusCode::FORBIDDEN, "capability not allowed by policy")
                            .into_response())
                    }
                }
            }
        })
    }
}
