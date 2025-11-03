//! Verifies PolicyLayer wiring and shows the failure/success cases.
//!
//! Case A (broken): Extension<PolicyBundle> layered *before* `middleware::apply` →
//!                  PolicyLayer can’t see the bundle → PUT returns 405 (router method guard).
//! Case B (fixed):  Extension<PolicyBundle> layered *after*  `middleware::apply` →
//!                  PolicyLayer sees the bundle → PUT returns 403 (policy deny).

use std::sync::Arc;

use axum::{
    body::Body, extract::Request, http::StatusCode, response::IntoResponse, routing::get, Json,
    Router,
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
