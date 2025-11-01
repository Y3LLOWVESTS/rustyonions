//! RO:WHAT   Thin policy middleware that consults ron-policy Evaluator.
//! RO:WHY    Keep policy out of business handlers; centralize allow/deny.
//! RO:INVARS  Respect config.fail_mode when evaluator unavailable; bounded context; low-cardinality labels.

use std::sync::Arc;

use axum::{extract::Request, http::StatusCode, response::IntoResponse};
use futures_util::future::BoxFuture;
use tower::{Layer, Service};

use crate::errors::GateError;
use crate::metrics::POLICY_SHORTCIRCUITS_TOTAL;
use crate::state::AppState;

#[derive(Clone)]
pub struct PolicyLayer;

impl<S> Layer<S> for PolicyLayer {
    type Service = PolicyService<S>;
    fn layer(&self, inner: S) -> Self::Service { PolicyService { inner } }
}

#[derive(Clone)]
pub struct PolicyService<S> {
    inner: S,
}

impl<S> Service<Request> for PolicyService<S>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Response: IntoResponse + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        futures_util::ready!(futures_util::future::poll_fn(|cx| self.inner.poll_ready(cx)).poll_unpin(cx));
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut inner = self.inner.clone();

        BoxFuture::from(async move {
            let state = req.extensions().get::<Arc<AppState>>().cloned();

            if let Some(state) = state {
                if let Some(eval) = state.policy.clone() {
                    // Minimal ron-policy input
                    let method = req.method().as_str().to_owned();
                    let path = req.uri().path().to_owned();
                    let tenant = state.tenant.clone().unwrap_or_else(|| "default".to_string());

                    let ctx = ron_policy::EvalContext {
                        tenant,
                        method,
                        path,
                        region: state.region.clone().unwrap_or_default(),
                        tags: state.tags_for(&req),
                        ..Default::default()
                    };

                    match eval.evaluate(&ctx) {
                        Ok(dec) if dec.effect.is_allow() => inner.call(req).await,
                        Ok(dec) => {
                            let status = if dec.reason.as_deref() == Some("LEGAL") {
                                StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS
                            } else {
                                StatusCode::FORBIDDEN
                            };
                            POLICY_SHORTCIRCUITS_TOTAL.with_label_values(&[status.as_str()]).inc();
                            let resp = GateError::PolicyDeny {
                                reason: dec.reason.as_deref().unwrap_or("DENY"),
                                status,
                            }
                            .into_response();
                            Ok(axum::response::IntoResponse::into_response(resp).into())
                        }
                        Err(_) => {
                            POLICY_SHORTCIRCUITS_TOTAL.with_label_values(&["503"]).inc();
                            let resp = GateError::PolicyError.into_response();
                            Ok(axum::response::IntoResponse::into_response(resp).into())
                        }
                    }
                } else if state.config.policy.fail_deny() {
                    POLICY_SHORTCIRCUITS_TOTAL.with_label_values(&["403"]).inc();
                    let resp = GateError::PolicyDeny {
                        reason: "NO_EVALUATOR",
                        status: StatusCode::FORBIDDEN,
                    }
                    .into_response();
                    return Ok(axum::response::IntoResponse::into_response(resp).into());
                }
            }

            inner.call(req).await
        })
    }
}
