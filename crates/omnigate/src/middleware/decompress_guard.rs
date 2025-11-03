//! RO:WHAT — Decompression guard for request bodies.
//! RO:WHY  — Stop risky encodings and cap potential decompression-bomb expansion at the edge.
//!
//! RO:BEHAVIOR —
//!   • Reject unsupported or (optionally) stacked encodings with 415 using our JSON envelope.
//!   • Allowed encodings come from config: `admission.decompression.allow` (e.g., ["identity","gzip"]).
//!   • If compressed (encoding != identity) and Content-Length is present, require:
//!         compressed_length * EXPANSION_CAP <= MAX_EXPANDED
//!     where MAX_EXPANDED = `admission.body.max_content_length` and EXPANSION_CAP = 10.
//!   • Streaming/unknown sizes are still protected by `DefaultBodyLimit` in `body_caps`.
//!
//! RO:INVARIANTS — Pure guard (no decompression). Budgets track body caps precisely.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    http::{HeaderValue, Request},
    response::{IntoResponse, Response},
    Router,
};
use tower::{Layer, Service};

use crate::errors::{http_map, Reason};
// IMPORTANT: use counters from metrics/gates so we're on the default registry.
use crate::metrics::gates::DECOMPRESS_REJECT_TOTAL;

/// Worst-case expansion factor budgeted for compressed bodies.
const EXPANSION_CAP: usize = 10;

/// Config-aware attach: add the guard with values pulled from Admission.
pub fn attach_with_cfg<S>(router: Router<S>, adm: &crate::config::Admission) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let allow = adm
        .decompression
        .allow
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let deny_stacked = adm.decompression.deny_stacked;
    let max_expanded = adm.body.max_content_length as usize;

    router.layer(DecompressGuardLayer {
        allow,
        deny_stacked,
        max_expanded,
    })
}

/// Layer carrying admission parameters.
#[derive(Clone)]
pub struct DecompressGuardLayer {
    allow: Vec<String>,
    deny_stacked: bool,
    max_expanded: usize,
}

impl<S> Layer<S> for DecompressGuardLayer {
    type Service = DecompressGuard<S>;
    fn layer(&self, inner: S) -> Self::Service {
        DecompressGuard {
            inner,
            allow: self.allow.clone(),
            deny_stacked: self.deny_stacked,
            max_expanded: self.max_expanded,
        }
    }
}

#[derive(Clone)]
pub struct DecompressGuard<S> {
    inner: S,
    allow: Vec<String>,
    deny_stacked: bool,
    max_expanded: usize,
}

impl<S, B> Service<Request<B>> for DecompressGuard<S>
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
        // Parse and normalize Content-Encoding(s)
        let enc_header = req.headers().get(axum::http::header::CONTENT_ENCODING);

        let encodings = enc_header
            .and_then(|hv: &HeaderValue| hv.to_str().ok())
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase())
            .collect::<Vec<_>>();

        // Stacked encodings?
        if self.deny_stacked && encodings.len() > 1 {
            DECOMPRESS_REJECT_TOTAL
                .with_label_values(&["stacked"])
                .inc();

            let resp = http_map::to_response(
                Reason::UnsupportedMediaType,
                "stacked Content-Encoding not allowed",
            );
            return Box::pin(async move { Ok(resp) });
        }

        // Validate the (single) encoding or absence thereof.
        let encoding_opt = encodings.first().map(|s| s.as_str());
        let is_identity_or_none = match encoding_opt {
            None => true, // no header = identity
            Some(enc) => enc == "identity",
        };

        if let Some(enc) = encoding_opt {
            if !self.allow.iter().any(|a| a == enc) {
                // Disallowed/unknown encoding.
                DECOMPRESS_REJECT_TOTAL
                    .with_label_values(&["unknown"])
                    .inc();

                let resp = http_map::to_response(
                    Reason::UnsupportedMediaType,
                    "Content-Encoding not allowed by policy",
                );
                return Box::pin(async move { Ok(resp) });
            }
        }

        // If compressed and length is known, enforce expansion budget.
        if !is_identity_or_none {
            if let Some(cl) = req
                .headers()
                .get(axum::http::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
            {
                let compressed = cl as usize;
                if compressed.saturating_mul(EXPANSION_CAP) > self.max_expanded {
                    DECOMPRESS_REJECT_TOTAL
                        .with_label_values(&["over_budget"])
                        .inc();

                    let resp = http_map::to_response(
                        Reason::PayloadTooLarge,
                        "compressed body exceeds allowed expansion budget",
                    );
                    return Box::pin(async move { Ok(resp) });
                }
            }
            // No Content-Length → streaming is still guarded by DefaultBodyLimit downstream.
        }

        let fut = self.inner.call(req);
        Box::pin(async move {
            let res = fut.await?.into_response();
            Ok(res)
        })
    }
}
