//! RO:WHAT — Decompression guard for request bodies.
//! RO:WHY  — Stop risky encodings and cap potential decompression-bomb expansion at the edge.
//!
//! RO:BEHAVIOR —
//!   • Reject unsupported or stacked encodings with 415 (Unsupported Media Type) using our envelope.
//!   • Allow only: identity (or none), gzip, deflate, br.
//!   • If compressed (gzip/deflate/br) and Content-Length is present, require:
//!         content_length <= MAX_EXPANDED / EXPANSION_CAP
//!     so a worst-case expansion <= MAX_EXPANDED (default 1 MiB).
//!   • Streaming / unknown sizes are still protected by body caps (DefaultBodyLimit) in `body_caps`.
//!
//! RO:INVARIANTS — No decompression here (pure guard). Keep budgets aligned with body caps.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    http::Request,
    response::{IntoResponse, Response},
};
use tower::{Layer, Service};

use crate::errors::{http_map, Reason};

const KIB: usize = 1024;
const MIB: usize = KIB * KIB;

/// Max allowed post-inflate size (should match body cap).
const MAX_EXPANDED: usize = MIB; // 1 MiB

/// Worst-case expansion factor we budget for compressed bodies.
const EXPANSION_CAP: usize = 10;

/// Encodings we accept. Order matters when stacked (we deny stacks for now).
const ENC_IDENTITY: &str = "identity";
const ENC_GZIP: &str = "gzip";
const ENC_DEFLATE: &str = "deflate";
const ENC_BR: &str = "br";

#[derive(Clone, Copy, Default)]
pub struct DecompressGuardLayer;

pub fn layer() -> DecompressGuardLayer {
    DecompressGuardLayer
}

impl<S> Layer<S> for DecompressGuardLayer {
    type Service = DecompressGuard<S>;
    fn layer(&self, inner: S) -> Self::Service {
        DecompressGuard { inner }
    }
}

#[derive(Clone)]
pub struct DecompressGuard<S> {
    inner: S,
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
        // Parse Content-Encoding (may be comma-separated per RFC). Keep owned Strings locally.
        let enc_header = req
            .headers()
            .get(axum::http::header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .trim();

        // Normalize to lowercase, trim, drop empties.
        let encodings: Vec<String> = if enc_header.is_empty() {
            Vec::new()
        } else {
            enc_header
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_ascii_lowercase())
                .collect()
        };

        // Reject stacked encodings for now (complex, rare, riskier).
        if encodings.len() > 1 {
            // Metrics: stacked encoding reject
            crate::metrics::DECOMPRESS_REJECT_TOTAL
                .with_label_values(&["stacked"])
                .inc();

            let resp = http_map::to_response(
                Reason::UnsupportedMediaType,
                "stacked Content-Encoding not supported",
            )
            .into_response();
            return Box::pin(async move { Ok(resp) });
        }

        // Validate single (or none).
        let is_compressed = if let Some(enc) = encodings.first().map(String::as_str) {
            match enc {
                ENC_IDENTITY => false,
                ENC_GZIP | ENC_DEFLATE | ENC_BR => true,
                // Disallow everything else (e.g., compress, zstd (not negotiated here), etc.)
                _ => {
                    // Metrics: unknown/unsupported encoding reject
                    crate::metrics::DECOMPRESS_REJECT_TOTAL
                        .with_label_values(&["unknown"])
                        .inc();

                    let resp = http_map::to_response(
                        Reason::UnsupportedMediaType,
                        "unsupported Content-Encoding",
                    )
                    .into_response();
                    return Box::pin(async move { Ok(resp) });
                }
            }
        } else {
            false
        };

        // If compressed: enforce conservative expansion budget using Content-Length if present.
        if is_compressed {
            if let Some(cl) = req
                .headers()
                .get(axum::http::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
            {
                // require compressed_length * EXPANSION_CAP <= MAX_EXPANDED
                if (cl as usize).saturating_mul(EXPANSION_CAP) > MAX_EXPANDED {
                    // Metrics: expansion budget reject
                    crate::metrics::DECOMPRESS_REJECT_TOTAL
                        .with_label_values(&["over_budget"])
                        .inc();

                    let resp = http_map::to_response(
                        Reason::PayloadTooLarge,
                        "compressed body exceeds allowed expansion budget",
                    )
                    .into_response();
                    return Box::pin(async move { Ok(resp) });
                }
            }
            // If no Content-Length, runtime streaming limit in `body_caps` still protects us.
        }

        let fut = self.inner.call(req);
        Box::pin(async move {
            let res = fut.await?.into_response();
            Ok(res)
        })
    }
}
