//! RO:WHAT — Request body size caps.
//! RO:WHY  — Prevent DoS and enforce hard limits early.
//! RO:BEHAVIOR —
//!   * If `Content-Length` is present and > MAX, short-circuit with 413 JSON using our error map.
//!   * Otherwise, forward and rely on Axum's body limiter (`DefaultBodyLimit::max`) for streaming
//!     and methods without Content-Length.
//!
//! RO:INVARIANTS — Keep MAX aligned with OAP/HTTP caps (default: 1 MiB). Emit metrics for oversize rejects.
//!                 Do not emit 411 here; policy_gate tests rely on 405/403 from routing/policy layers.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    extract::DefaultBodyLimit,
    http::{self, Request},
    response::{IntoResponse, Response},
};
use tower::{Layer, Service};

use crate::errors::{http_map, Reason};
// IMPORTANT: use metrics from gates module so we hit the default-registry counters.
use crate::metrics::gates::BODY_REJECT_TOTAL;

/// Size constants (avoid clippy identity-op).
const KIB: usize = 1024;
const MIB: usize = KIB * KIB;
/// Default max body bytes (1 MiB). Keep in sync with service config later.
const MAX_BYTES: usize = MIB;

/// Public factory returning the composed guard as a tuple of layers,
/// which implements `Layer<Route>` (compatible with `Router::layer`).
pub fn layer() -> (PreflightContentLengthGuardLayer, DefaultBodyLimit) {
    (
        PreflightContentLengthGuardLayer { max: MAX_BYTES },
        // NOTE: DefaultBodyLimit::max takes a usize, and MAX_BYTES is already usize.
        DefaultBodyLimit::max(MAX_BYTES),
    )
}

/// Fast-path guard that inspects `Content-Length` and short-circuits with a 413 JSON
/// when the declared size is clearly over budget.
#[derive(Clone, Copy)]
pub struct PreflightContentLengthGuardLayer {
    pub(crate) max: usize,
}

impl<S> Layer<S> for PreflightContentLengthGuardLayer {
    type Service = PreflightContentLengthGuard<S>;
    fn layer(&self, inner: S) -> Self::Service {
        PreflightContentLengthGuard {
            inner,
            max: self.max,
        }
    }
}

#[derive(Clone)]
pub struct PreflightContentLengthGuard<S> {
    inner: S,
    max: usize,
}

impl<S, B> Service<Request<B>> for PreflightContentLengthGuard<S>
where
    S: Service<Request<B>>,
    S::Future: Send + 'static,
    S::Response: IntoResponse,
{
    type Response = Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        // If Content-Length is present and too big, reject immediately with our envelope.
        if let Some(len) = req
            .headers()
            .get(http::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            if len as usize > self.max {
                // Metrics: body oversize reject
                BODY_REJECT_TOTAL.with_label_values(&["oversize"]).inc();

                let resp = http_map::to_response(
                    Reason::PayloadTooLarge,
                    "request body exceeds configured limit",
                );
                return Box::pin(async move { Ok(resp) });
            }
        }

        // NOTE:
        // We *do not* emit 411 LengthRequired here anymore.
        //
        // For payload-carrying methods without a Content-Length header, and for
        // streaming/unknown sizes, we allow the request to proceed. Axum's
        // DefaultBodyLimit (wired via `layer()`) still enforces a hard cap on
        // the actual body size, while leaving routing/policy behavior (405/403)
        // intact for the policy_gate tests.
        let fut = self.inner.call(req);
        Box::pin(async move {
            let res = fut.await?.into_response();
            Ok(res)
        })
    }
}
