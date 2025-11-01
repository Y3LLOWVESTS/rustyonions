//! RO:WHAT   Correlation/request IDs middleware.
//! RO:WHY    Stable per-request IDs for logs/metrics/traces.
//! RO:INVARS Low cardinality; always attach request_id; optional correlation chain.

use axum::{
    extract::Request,
    http::header::{HeaderName, HeaderValue},
    response::{IntoResponse, Response},
};
use futures_util::future::BoxFuture;
use tower::{Layer, Service};

static HDR_REQ_ID: HeaderName = HeaderName::from_static("x-request-id");
static HDR_CORR_ID: HeaderName = HeaderName::from_static("x-correlation-id");

#[allow(dead_code)] // will be consumed by logging/observe once wired
#[derive(Debug, Clone)]
pub struct CorrelationIds {
    pub request_id: String,
    pub correlation_id: String,
}

#[derive(Clone)]
pub struct CorrIdLayer;

pub fn layer() -> CorrIdLayer {
    CorrIdLayer
}

impl<S> Layer<S> for CorrIdLayer {
    type Service = CorrIdService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        CorrIdService { inner }
    }
}

#[derive(Clone)]
pub struct CorrIdService<S> {
    inner: S,
}

// 128-bit random hex via fastrand 2.x (requires an explicit range).
#[inline]
fn gen_request_id() -> String {
    // full 64-bit range on each half
    let hi = fastrand::u64(0..=u64::MAX);
    let lo = fastrand::u64(0..=u64::MAX);
    format!("{:016x}{:016x}", hi, lo)
}

impl<S> Service<Request> for CorrIdService<S>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Response: IntoResponse + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let mut inner = self.inner.clone();

        // Request ID (always)
        let req_id = gen_request_id();

        // Correlation ID (propagate if present, else req_id). Keep ASCII-only to ensure header validity.
        let corr_id = req
            .headers()
            .get(&HDR_CORR_ID)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
            .filter(|s| s.is_ascii())
            .unwrap_or_else(|| req_id.clone());

        // Stash into request headers for downstream visibility (insert only if HeaderValue parses).
        if let Ok(v) = HeaderValue::from_str(&req_id) {
            req.headers_mut().insert(HDR_REQ_ID.clone(), v);
        }
        if let Ok(v) = HeaderValue::from_str(&corr_id) {
            req.headers_mut().insert(HDR_CORR_ID.clone(), v);
        }

        Box::pin(async move {
            let mut res = inner.call(req).await?.into_response();

            // Reflect IDs back to the client (again, only if HeaderValue parses).
            if let Ok(v) = HeaderValue::from_str(&req_id) {
                res.headers_mut().insert(HDR_REQ_ID.clone(), v);
            }
            if let Ok(v) = HeaderValue::from_str(&corr_id) {
                res.headers_mut().insert(HDR_CORR_ID.clone(), v);
            }

            Ok(res)
        })
    }
}
