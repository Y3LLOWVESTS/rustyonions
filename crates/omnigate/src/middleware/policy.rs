//! RO:WHAT   Thin policy middleware that consults a ron-policy Evaluator (if provided).
//! RO:WHY    Centralize allow/deny; keep business handlers policy-agnostic.
//! RO:INVARS If no evaluator is present, act as a no-op (safe pass-through).
//!           When denying, emit stable JSON envelopes and bounded-label metrics.

use std::{
    collections::BTreeSet,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::future::BoxFuture;
use tower::{Layer, Service};

use crate::errors::GateError;
// IMPORTANT: pull counters from the gates module to match registration on the default registry.
use crate::metrics::gates::POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL;

#[derive(Clone)]
pub struct PolicyLayer;

/// Public constructor used by the top-level middleware::apply.
pub fn layer() -> PolicyLayer {
    PolicyLayer
}

impl<S> Layer<S> for PolicyLayer {
    type Service = PolicyService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        PolicyService { inner }
    }
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
    type Response = Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut inner = self.inner.clone();

        // Pull an optional policy bundle from request extensions; build an Evaluator per request.
        let maybe_bundle = req
            .extensions()
            .get::<Arc<ron_policy::PolicyBundle>>()
            .cloned();

        Box::pin(async move {
            if let Some(bundle) = maybe_bundle {
                // Build Evaluator borrowing the bundle (lives for the request via Arc).
                match ron_policy::Evaluator::new(&bundle) {
                    Ok(eval) => {
                        // ron-policy Context (current API): populate minimally & safely.
                        let now_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;

                        // Tags: keep cardinality low and deterministic.
                        let mut tags: BTreeSet<String> = BTreeSet::new();
                        tags.insert("omnigate".to_string());

                        let method = req.method().as_str().to_owned();
                        // Region/tenant may be wired later via AppState; keep safe defaults.
                        let region = String::new();
                        let tenant = "default".to_string();

                        let ctx = ron_policy::Context {
                            now_ms,
                            body_bytes: 0, // unknown at admission time
                            method,
                            region,
                            tags,
                            tenant,
                        };

                        match eval.evaluate(&ctx) {
                            Ok(dec) => {
                                // DecisionEffect doesn’t expose is_allow(); match the variant.
                                match dec.effect {
                                    ron_policy::DecisionEffect::Allow => {
                                        let res = inner.call(req).await?;
                                        return Ok(res.into_response());
                                    }
                                    _ => {
                                        let status = if dec.reason.as_deref() == Some("LEGAL") {
                                            StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS
                                        } else {
                                            StatusCode::FORBIDDEN
                                        };

                                        // Metrics increment for deny/short-circuit.
                                        POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL
                                            .with_label_values(&[status.as_str()])
                                            .inc();

                                        let resp = GateError::PolicyDeny {
                                            reason: dec.reason.as_deref().unwrap_or("DENY"),
                                            status,
                                        }
                                        .into_response();
                                        return Ok(resp);
                                    }
                                }
                            }
                            Err(_e) => {
                                // Evaluator error → 503
                                POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL
                                    .with_label_values(&["503"])
                                    .inc();
                                let resp = GateError::PolicyError.into_response();
                                return Ok(resp);
                            }
                        }
                    }
                    Err(_e) => {
                        // If Evaluator construction fails, treat as transient policy error.
                        POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL
                            .with_label_values(&["503"])
                            .inc();
                        let resp = GateError::PolicyError.into_response();
                        return Ok(resp);
                    }
                }
            }

            // No bundle present → no-op pass-through.
            let res = inner.call(req).await?;
            Ok(res.into_response())
        })
    }
}
