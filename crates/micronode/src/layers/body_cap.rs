// crates/micronode/src/layers/body_cap.rs
//! RO:WHAT  — Header-level body cap + content-length policy.
//! RO:WHY   — Enforce explicit Content-Length only on methods that typically
//!            carry bodies (POST/PUT/PATCH). Don't force it for GET/DELETE.
//! RO:NOTE  — Still reject if a Content-Length is present but exceeds the cap.
//! RO:HTTP  — 411 Length Required; 413 Payload Too Large.

use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone, Copy)]
pub struct BodyCapLayer {
    cap_bytes: usize,
}

impl BodyCapLayer {
    pub fn new(cap_bytes: usize) -> Self {
        Self { cap_bytes }
    }
}

impl<S> Layer<S> for BodyCapLayer {
    type Service = BodyCapService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BodyCapService { inner, cap_bytes: self.cap_bytes }
    }
}

#[derive(Clone)]
pub struct BodyCapService<S> {
    inner: S,
    cap_bytes: usize,
}

impl<S, B> Service<Request<B>> for BodyCapService<S>
where
    S: Service<Request<B>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Delegate to inner; if inner errors, we still return Ready(Ok(())) and
        // map to 500 in call() to avoid bubbling error types here.
        match self.inner.poll_ready(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(_)) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();
        let cap = self.cap_bytes;

        // Methods that typically *carry* a body and must declare Content-Length explicitly.
        let requires_len = matches!(
            *req.method(),
            axum::http::Method::POST | axum::http::Method::PUT | axum::http::Method::PATCH
        );

        // Inspect Content-Length if present.
        let len_opt = req
            .headers()
            .get(axum::http::header::CONTENT_LENGTH)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok());

        // If a Content-Length is present on ANY method, enforce the cap.
        if let Some(len) = len_opt {
            if len > cap {
                return Box::pin(async move {
                    Ok((StatusCode::PAYLOAD_TOO_LARGE, "payload too large").into_response())
                });
            }
        }

        // For methods that *require* an explicit Content-Length, enforce presence.
        if requires_len && len_opt.is_none() {
            return Box::pin(async move {
                Ok((StatusCode::LENGTH_REQUIRED, "length required").into_response())
            });
        }

        Box::pin(async move {
            Ok(inner.call(req).await.unwrap_or_else(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
            }))
        })
    }
}
