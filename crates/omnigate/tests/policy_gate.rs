//! Verifies PolicyLayer wiring and shows the failure/success cases.
//!
//! Case A (broken): Extension<PolicyBundle> layered *before* `middleware::apply` →
//!                  PolicyLayer can’t see the bundle → PUT returns 405 (router method guard).
//! Case B (fixed):  Extension<PolicyBundle> layered *after*  `middleware::apply` →
//!                  PolicyLayer sees the bundle → PUT returns 403 (policy deny).

use std::sync::Arc;

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use ron_policy::PolicyBundle;
use serde_json::json;
use tower::ServiceExt;

async fn ping() -> impl IntoResponse {
    Json(json!({ "ok": true }))
}

// Minimal strict-policy bundle:
// - default deny
// - allow only GET
fn test_bundle() -> PolicyBundle {
    let json = r#"
    {
      "version": 1,
      "defaults": { "default_action": "deny" },
      "rules": [
        { "id": "allow-gets", "when": { "method": "GET" }, "action": "allow" }
      ]
    }"#;

    serde_json::from_str::<PolicyBundle>(json).expect("strict bundle should parse")
}

#[tokio::test]
async fn policy_broken_layering_yields_405_put() {
    // Router with only GET /v1/ping
    let router = Router::new().route("/v1/ping", get(ping));

    // ❌ BROKEN ORDER: layer Extension first, then apply middleware stack.
    // In this order, the PolicyLayer (added by middleware::apply) sits OUTSIDE
    // the Extension layer and thus does NOT see the bundled policy.
    let router = router.layer(axum::Extension(Arc::new(test_bundle())));
    let router = omnigate::middleware::apply(router);

    // PUT should not be allowed by the route; without policy, this becomes 405.
    let req = Request::builder()
        .method("PUT")
        .uri("/v1/ping")
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "broken layering should yield 405 (policy unseen)"
    );
}

#[tokio::test]
async fn policy_correct_layering_yields_403_put() {
    // Router with only GET /v1/ping
    let base = Router::new().route("/v1/ping", get(ping));

    // ✅ CORRECT ORDER: build the middleware stack first (includes PolicyLayer),
    // then layer the Extension so it runs OUTSIDE and is visible to PolicyLayer.
    let router = omnigate::middleware::apply(base).layer(axum::Extension(Arc::new(test_bundle())));

    // PUT should be denied by policy (default deny; only GET is allowed).
    let req = Request::builder()
        .method("PUT")
        .uri("/v1/ping")
        .body(Body::empty())
        .unwrap();

    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "correct layering should yield 403 (policy deny)"
    );

    // GET should still pass (rule allows GET)
    let req_ok = Request::builder()
        .method("GET")
        .uri("/v1/ping")
        .body(Body::empty())
        .unwrap();

    let resp_ok = router.oneshot(req_ok).await.unwrap();
    assert_eq!(
        resp_ok.status(),
        StatusCode::OK,
        "GET should be allowed by policy"
    );
}

fn production_bundle() -> PolicyBundle {
    ron_policy::load_json(include_bytes!("../configs/policy.bundle.json"))
        .expect("production Omnigate policy bundle must parse and validate")
}

#[tokio::test]
async fn cn4_production_policy_allows_only_exact_fixed_identity_post() {
    let base = Router::new()
        .route("/v1/identity/passport/register/challenge", post(ping))
        .route("/v1/identity/passport/register/challenge-other", post(ping));

    let router =
        omnigate::middleware::apply(base).layer(axum::Extension(Arc::new(production_bundle())));

    let accepted = Request::builder()
        .method("POST")
        .uri("/v1/identity/passport/register/challenge")
        .header("content-length", "0")
        .body(Body::empty())
        .unwrap();

    let accepted_response = router.clone().oneshot(accepted).await.unwrap();

    assert_eq!(
        accepted_response.status(),
        StatusCode::OK,
        "the exact reviewed CN-4 fixed identity POST must pass policy",
    );

    let denied = Request::builder()
        .method("POST")
        .uri("/v1/identity/passport/register/challenge-other")
        .header("content-length", "0")
        // Deliberately attempt to inject the internal tag through a
        // caller-controlled header. Policy middleware must ignore it.
        .header("x-omnigate-policy-tag", "cn4-fixed-identity-admission")
        .body(Body::empty())
        .unwrap();

    let denied_response = router.clone().oneshot(denied).await.unwrap();

    assert_eq!(
        denied_response.status(),
        StatusCode::FORBIDDEN,
        "a different POST must remain default-denied even when a caller tries to inject the internal tag",
    );
}

#[tokio::test]
async fn cn4_production_policy_allows_exact_register_root_proof_and_denies_unreviewed_prove() {
    let base = Router::new()
        .route("/v1/identity/passport/register/proof", post(ping))
        .route("/v1/identity/passport/prove/extra", post(ping));

    let router =
        omnigate::middleware::apply(base).layer(axum::Extension(Arc::new(production_bundle())));

    let accepted = Request::builder()
        .method("POST")
        .uri("/v1/identity/passport/register/proof")
        .header("content-length", "0")
        .body(Body::empty())
        .unwrap();

    let accepted_response = router.clone().oneshot(accepted).await.unwrap();

    assert_eq!(
        accepted_response.status(),
        StatusCode::OK,
        "the exact reviewed CN-4 RegisterRoot proof POST must pass policy",
    );

    let denied = Request::builder()
        .method("POST")
        .uri("/v1/identity/passport/prove/extra")
        .header("content-length", "0")
        .header("x-omnigate-policy-tag", "cn4-fixed-identity-admission")
        .body(Body::empty())
        .unwrap();

    let denied_response = router.oneshot(denied).await.unwrap();

    assert_eq!(
        denied_response.status(),
        StatusCode::FORBIDDEN,
        "an unreviewed prove path must remain default-denied even when a caller tries to inject the internal tag",
    );

    #[tokio::test]
    async fn cn4_production_policy_allows_exact_capability_issue_pair_only() {
        let base = Router::new()
            .route("/v1/identity/passport/capability/challenge", post(ping))
            .route("/v1/identity/passport/capability/prove", post(ping))
            .route("/v1/identity/passport/capability/refresh", post(ping))
            .route("/v1/identity/passport/capability/prove/extra", post(ping));

        let router =
            omnigate::middleware::apply(base).layer(axum::Extension(Arc::new(production_bundle())));

        for path in [
            "/v1/identity/passport/capability/challenge",
            "/v1/identity/passport/capability/prove",
        ] {
            let request = Request::builder()
                .method("POST")
                .uri(path)
                .header("content-length", "0")
                .body(Body::empty())
                .unwrap();

            let response = router.clone().oneshot(request).await.unwrap();

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "{path} is an explicitly reviewed fixed CN-4 capability issuance route",
            );
        }

        for path in [
            "/v1/identity/passport/capability/refresh",
            "/v1/identity/passport/capability/prove/extra",
        ] {
            let request = Request::builder()
                .method("POST")
                .uri(path)
                .header("content-length", "0")
                .header("x-omnigate-policy-tag", "cn4-fixed-identity-admission")
                .body(Body::empty())
                .unwrap();

            let response = router.clone().oneshot(request).await.unwrap();

            assert_eq!(
                response.status(),
                StatusCode::FORBIDDEN,
                "{path} must remain default-denied even with caller tag injection",
            );
        }
    }
}
