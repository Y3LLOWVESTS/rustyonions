//! RO:WHAT — Omnigate admission middleware over `ron-policy`, including trusted classification for reviewed fixed routes.
//! RO:WHY — Centralize allow/deny while keeping product handlers policy-agnostic and preserving default-deny writes.
//! RO:INTERACTS — `ron-policy`, Omnigate middleware, and the reviewed CN-4 fixed identity registration, device, capability, and protected username/profile ingress.
//! RO:INVARIANTS — policy remains declarative; fixed-route tags derive only from local method/URI; caller headers cannot create policy tags; unmatched writes remain denied.
//! RO:METRICS — policy short-circuit counters retain bounded status labels.
//! RO:CONFIG — consumes the operator-loaded `PolicyBundle`; no hidden allow-mode switch is added.
//! RO:SECURITY — no Passport, capability, wallet, ledger, signing, or identity authority moves into Omnigate.
//! RO:TEST — `tests/policy_gate.rs` and `tests/crabnode_cn4_register_root_challenge_proxy.rs`.

use std::{
    collections::BTreeSet,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::Request,
    http::{Method, StatusCode},
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

const CN4_FIXED_IDENTITY_ADMISSION_PATHS: &[&str] = &[
    "/v1/identity/passport/register/challenge",
    "/v1/identity/passport/register/proof",
    "/v1/identity/passport/device/authorize",
    "/v1/identity/passport/challenge",
    "/v1/identity/passport/prove",
    "/v1/identity/passport/capability/challenge",
    "/v1/identity/passport/capability/prove",
    "/v1/identity/passport/profile/claim",
];

const CN4_FIXED_IDENTITY_ADMISSION_TAG: &str = "cn4-fixed-identity-admission";

fn trusted_policy_tags(req: &Request) -> BTreeSet<String> {
    let mut tags = BTreeSet::new();

    tags.insert("omnigate".to_owned());

    if req.method() == Method::POST
        && CN4_FIXED_IDENTITY_ADMISSION_PATHS
            .iter()
            .any(|path| req.uri().path() == *path)
    {
        tags.insert(CN4_FIXED_IDENTITY_ADMISSION_TAG.to_owned());
    }

    tags
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

                        // Tags are low-cardinality and derived from trusted
                        // local request properties, never caller-selected headers.
                        let tags = trusted_policy_tags(&req);

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

#[cfg(test)]
mod cn4_device_authorize_policy_tests {
    use super::{trusted_policy_tags, CN4_FIXED_IDENTITY_ADMISSION_TAG};
    use axum::{body::Body, extract::Request, http::Method};

    fn has_fixed_identity_tag(method: Method, path: &str) -> bool {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .expect("policy test request");

        trusted_policy_tags(&request).contains(CN4_FIXED_IDENTITY_ADMISSION_TAG)
    }

    #[test]
    fn exact_device_authorize_post_receives_trusted_tag() {
        assert!(has_fixed_identity_tag(
            Method::POST,
            "/v1/identity/passport/device/authorize",
        ),);

        assert!(has_fixed_identity_tag(
            Method::POST,
            "/v1/identity/passport/register/challenge",
        ),);

        assert!(has_fixed_identity_tag(
            Method::POST,
            "/v1/identity/passport/register/proof",
        ),);

        assert!(has_fixed_identity_tag(
            Method::POST,
            "/v1/identity/passport/challenge",
        ),);

        assert!(has_fixed_identity_tag(
            Method::POST,
            "/v1/identity/passport/prove",
        ),);

        assert!(has_fixed_identity_tag(
            Method::POST,
            "/v1/identity/passport/capability/challenge",
        ),);

        assert!(has_fixed_identity_tag(
            Method::POST,
            "/v1/identity/passport/capability/prove",
        ),);

        assert!(has_fixed_identity_tag(
            Method::POST,
            "/v1/identity/passport/profile/claim",
        ),);
    }

    #[test]
    fn nearby_device_paths_and_wrong_method_remain_untrusted() {
        for path in [
            "/v1/identity/passport/device/register",
            "/v1/identity/passport/device/authorize/extra",
            "/v1/identity/passport/challenge/extra",
            "/v1/identity/passport/prove/extra",
            "/v1/identity/passport/capability/challenge/extra",
            "/v1/identity/passport/capability/prove/extra",
            "/v1/identity/passport/capability/refresh",
            "/v1/identity/passport/capability/revoke",
            "/v1/identity/passport/profile/claim/extra",
            "/v1/identity/passport/profile/claim-other",
        ] {
            assert!(
                !has_fixed_identity_tag(Method::POST, path,),
                "nearby path received trusted tag: {path}",
            );
        }

        assert!(!has_fixed_identity_tag(
            Method::GET,
            "/v1/identity/passport/device/authorize",
        ),);

        assert!(!has_fixed_identity_tag(
            Method::GET,
            "/v1/identity/passport/profile/claim",
        ),);
    }
}
