// crates/svc-passport/tests/jwks_roundtrip.rs
// RO:WHAT  Issue → fetch /v1/keys → assert the returned JWKS contains the KID we signed with.
// RO:WHY   Ensures JWKS correctness + rotation compatibility (current KID is advertised).
// RO:INVARS  No secret leakage; stable, hermetic (uses embedded default config via test_support).

use axum::body::to_bytes;
use axum::{body::Body, http, http::Request};
use tower::ServiceExt;

use svc_passport::{health::Health, http::router::build_router};

#[path = "../src/test_support.rs"]
mod test_support;

use serde_json::Value;
use test_support::default_config;

#[tokio::test]
async fn jwks_contains_current_kid_used_by_issue() {
    let app = build_router(default_config(), Health::default());

    // 1) Issue a passport to obtain an envelope (alg,kid,msg_b64,sig_b64)
    let issue_body = Body::from(serde_json::to_vec(&serde_json::json!({"hello":"world"})).unwrap());
    let issue_req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/issue")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(issue_body)
        .unwrap();

    let issue_resp = app.clone().oneshot(issue_req).await.unwrap();
    assert!(issue_resp.status().is_success());

    let env_json: Value =
        serde_json::from_slice(&to_bytes(issue_resp.into_body(), usize::MAX).await.unwrap())
            .unwrap();
    let kid_issued = env_json["kid"]
        .as_str()
        .expect("kid in issue response")
        .to_string();

    // 2) Fetch JWKS and assert the KID is advertised with OKP/Ed25519 shape
    let jwks_req = Request::builder()
        .method(http::Method::GET)
        .uri("/v1/keys")
        .body(Body::empty())
        .unwrap();

    let jwks_resp = app.clone().oneshot(jwks_req).await.unwrap();
    assert!(jwks_resp.status().is_success());

    let jwks: Value =
        serde_json::from_slice(&to_bytes(jwks_resp.into_body(), usize::MAX).await.unwrap())
            .unwrap();

    let keys = jwks["keys"].as_array().expect("JWKS keys array");
    assert!(
        !keys.is_empty(),
        "JWKS should contain at least one verifying key"
    );

    let mut found = false;
    for k in keys {
        let kty = k.get("kty").and_then(|v| v.as_str()).unwrap_or_default();
        let crv = k.get("crv").and_then(|v| v.as_str()).unwrap_or_default();
        let kid = k.get("kid").and_then(|v| v.as_str()).unwrap_or_default();
        let x = k.get("x").and_then(|v| v.as_str()).unwrap_or_default();
        if kid == kid_issued {
            assert_eq!(kty, "OKP", "JWKS kty must be OKP for Ed25519");
            assert_eq!(crv, "Ed25519", "JWKS crv must be Ed25519");
            assert!(
                !x.is_empty(),
                "JWKS 'x' (public key) must be non-empty base64url"
            );
            found = true;
            break;
        }
    }

    assert!(
        found,
        "JWKS should advertise the KID used by /issue (kid={})",
        kid_issued
    );
}
