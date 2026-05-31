//! RO:WHAT — Request body size caps for Omnigate HTTP ingress.
//! RO:WHY — Prevent DoS while allowing bounded CrabLink media/image HTTP uploads.
//! RO:BEHAVIOR —
//!   * If `Content-Length` is present and > cap, short-circuit with 413 JSON.
//!   * Otherwise, forward and rely on Axum `DefaultBodyLimit::max(cap)`.
//! RO:INVARIANTS — OAP max_frame remains separate at 1 MiB; this file controls HTTP body caps only.
//! RO:METRICS — increments `body_reject_total` via gates metrics on oversize rejects.
//! RO:CONFIG — OMNIGATE_MAX_BODY_BYTES, OMNIGATE_MAX_CONTENT_LENGTH, CRABLINK_DEV_IMAGE_BODY_BYTES.
//! RO:SECURITY — no body bytes are read in preflight; malformed/oversized requests fail closed.

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
use tracing::info;

use crate::errors::{http_map, Reason};
// IMPORTANT: use metrics from gates module so we hit the default-registry counters.
use crate::metrics::gates::BODY_REJECT_TOTAL;

/// Size constants.
const KIB: usize = 1024;
const MIB: usize = KIB * KIB;

/// Default max HTTP body bytes for the current CrabLink image/media proof.
///
/// This intentionally differs from OAP framing. OAP frames remain capped at
/// 1 MiB; HTTP upload routes may accept larger bounded bodies before handing
/// bytes to storage/gateway product flows.
const DEFAULT_MAX_BYTES: usize = 64 * MIB;

/// Hard ceiling for this HTTP middleware guard.
///
/// Do not raise this casually. Larger production media should move toward
/// chunked/ranged upload paths instead of pushing huge buffers through one
/// request.
const MAX_CONFIGURABLE_BYTES: usize = 64 * MIB;

const ENV_BODY_CAP_KEYS: &[&str] = &[
    "OMNIGATE_MAX_BODY_BYTES",
    "OMNIGATE_MAX_CONTENT_LENGTH",
    "CRABLINK_DEV_IMAGE_BODY_BYTES",
];

/// Return the configured HTTP request-body cap.
///
/// Invalid, zero, or excessive values fall back/clamp safely because this is a
/// middleware factory, not the config validator. The typed config/env loader is
/// responsible for failing closed on malformed operator config.
#[must_use]
pub fn configured_max_bytes() -> usize {
    for key in ENV_BODY_CAP_KEYS {
        let Some(value) = std::env::var(key)
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok())
            .filter(|value| *value > 0)
        else {
            continue;
        };

        return value.min(MAX_CONFIGURABLE_BYTES);
    }

    DEFAULT_MAX_BYTES
}

/// Public factory returning the composed guard as a tuple of layers,
/// which implements `Layer<Route>` and is compatible with `Router::layer`.
pub fn layer() -> (PreflightContentLengthGuardLayer, DefaultBodyLimit) {
    let max = configured_max_bytes();

    info!(
        max_body_bytes = max,
        "omnigate HTTP request body cap active"
    );

    (
        PreflightContentLengthGuardLayer { max },
        DefaultBodyLimit::max(max),
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
        if let Some(len) = req
            .headers()
            .get(http::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
        {
            if len as usize > self.max {
                BODY_REJECT_TOTAL.with_label_values(&["oversize"]).inc();

                let resp = http_map::to_response(Reason::PayloadTooLarge, "request body too large");

                return Box::pin(async move { Ok(resp) });
            }
        }

        let fut = self.inner.call(req);

        Box::pin(async move {
            let response = fut.await?.into_response();
            Ok(response)
        })
    }
}
