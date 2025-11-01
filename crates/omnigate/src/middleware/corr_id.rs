//! RO:WHAT — Correlation-ID middleware.
//! RO:WHY  — Ensure every request/response is traceable across services.
//! RO:BEHAVIOR — reads `X-Request-Id` / `X-Correlation-Id`, generates if missing,
//!               stores in request extensions, echoes on response.

use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering::Relaxed},
    task::{Context, Poll},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::{HeaderMap, HeaderValue, Request};
use axum::response::{IntoResponse, Response};
use http::header::HeaderName;
use tower::{layer::Layer, Service};
use tracing::debug;

static NEXT_SEQ: AtomicU64 = AtomicU64::new(1);

const H_REQUEST_ID: &str = "x-request-id";
const H_CORR_ID: &str = "x-correlation-id";

#[derive(Clone, Copy, Default)]
pub struct CorrIdLayer;

pub fn layer() -> CorrIdLayer {
    CorrIdLayer
}

/// Values available to downstream handlers via `req.extensions()`.
#[derive(Clone, Debug)]
pub struct CorrelationIds {
    pub request_id: String,
    pub correlation_id: String,
}

impl<S> Layer<S> for CorrIdLayer {
    type Service = CorrId<S>;
    fn layer(&self, inner: S) -> Self::Service {
        CorrId { inner }
    }
}

#[derive(Clone)]
pub struct CorrId<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for CorrId<S>
where
    S: Service<Request<B>>,
    S::Future: Send + 'static,
    S::Response: IntoResponse, // allow any axum response type
{
    type Response = Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        // Extract or generate IDs, then stash into extensions.
        let (req_id, corr_id) = extract_or_generate(req.headers());
        req.extensions_mut().insert(CorrelationIds {
            request_id: req_id.clone(),
            correlation_id: corr_id.clone(),
        });

        debug!(request_id = %req_id, correlation_id = %corr_id, "corr_id assigned");

        let add_headers = move |headers: &mut HeaderMap| {
            if !headers.contains_key(H_REQUEST_ID) {
                if let Ok(v) = HeaderValue::from_str(&req_id) {
                    headers.insert(HeaderName::from_static(H_REQUEST_ID), v);
                }
            }
            if !headers.contains_key(H_CORR_ID) {
                if let Ok(v) = HeaderValue::from_str(&corr_id) {
                    headers.insert(HeaderName::from_static(H_CORR_ID), v);
                }
            }
        };

        let fut = self.inner.call(req);
        Box::pin(async move {
            // Convert to a concrete axum Response so we can mutate headers.
            let mut res: Response = fut.await?.into_response();
            add_headers(res.headers_mut());
            Ok(res)
        })
    }
}

/// Pull IDs from headers (case-insensitive); generate if absent.
fn extract_or_generate(headers: &HeaderMap) -> (String, String) {
    let rid = get_header(headers, H_REQUEST_ID);
    let cid = get_header(headers, H_CORR_ID);

    match (rid, cid) {
        (Some(r), Some(c)) => (r, c),
        (Some(r), None) => (r.clone(), r),
        (None, Some(c)) => (generate_id(), c),
        (None, None) => {
            let r = generate_id();
            (r.clone(), r)
        }
    }
}

fn get_header(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok().map(|s| s.to_owned()))
}

/// Generate a compact 16-hex ID without extra deps.
fn generate_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = NEXT_SEQ.fetch_add(1, Relaxed) as u128;

    let combined = (millis << 16) ^ (seq & 0xFFFF);
    let mut s = format!("{combined:x}");
    if s.len() > 16 {
        s.truncate(16);
    } else {
        while s.len() < 16 {
            s.insert(0, '0');
        }
    }
    s
}
