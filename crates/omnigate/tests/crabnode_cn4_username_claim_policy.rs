//! RO:WHAT — Proves production Omnigate policy admits only the exact reviewed CN-4 protected username claim POST.
//! RO:WHY — A DeviceKey-signed username claim must reach svc-passport while neighboring write routes remain default-denied.
//! RO:INTERACTS — Omnigate policy middleware and the production policy bundle.
//! RO:INVARIANTS — exact POST only; default deny preserved; caller headers cannot manufacture trusted policy classification.
//! RO:METRICS — none.
//! RO:CONFIG — uses the checked-in production Omnigate policy bundle.
//! RO:SECURITY — no wildcard POST admission, no client-selected policy tag, no Passport/device/capability authority added here.
//! RO:TEST — cargo test -p omnigate --test crabnode_cn4_username_claim_policy.

use std::sync::Arc;

use axum::{body::Body, extract::Request, http::StatusCode, routing::post, Router};
use ron_policy::PolicyBundle;
use tower::ServiceExt;

async fn accepted() -> &'static str {
    "accepted"
}

fn production_bundle() -> PolicyBundle {
    ron_policy::load_json(include_bytes!("../configs/policy.bundle.json"))
        .expect("production Omnigate policy bundle must parse and validate")
}

#[tokio::test]
async fn exact_protected_username_claim_post_is_admitted() {
    let base = Router::new().route("/v1/identity/passport/profile/claim", post(accepted));

    let router =
        omnigate::middleware::apply(base).layer(axum::Extension(Arc::new(production_bundle())));

    let request = Request::builder()
        .method("POST")
        .uri("/v1/identity/passport/profile/claim")
        .header("content-length", "0")
        .body(Body::empty())
        .expect("exact username claim request");

    let response = router.oneshot(request).await.expect("policy response");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "exact reviewed protected username claim must pass production policy",
    );
}

#[tokio::test]
async fn nearby_username_claim_post_remains_default_denied() {
    let base = Router::new().route("/v1/identity/passport/profile/claim-other", post(accepted));

    let router =
        omnigate::middleware::apply(base).layer(axum::Extension(Arc::new(production_bundle())));

    let request = Request::builder()
        .method("POST")
        .uri("/v1/identity/passport/profile/claim-other")
        .header("content-length", "0")
        .header("x-omnigate-policy-tag", "cn4-fixed-identity-admission")
        .body(Body::empty())
        .expect("nearby username claim request");

    let response = router.oneshot(request).await.expect("policy response");

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "nearby write must remain default-denied even with caller tag injection",
    );
}
