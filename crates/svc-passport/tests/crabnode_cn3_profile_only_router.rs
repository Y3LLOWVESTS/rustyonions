//! RO:WHAT — Live contract test for CrabNode's svc-passport profile-only router.
//! RO:WHY — CN-3 must expose real public identity without introducing DevKms or service signing/admin authority.
//! RO:INTERACTS — svc_passport::http::router::build_profile_router and canonical profile handlers.
//! RO:INVARIANTS — profile claim/read works; issue/verify/keys/KMS-admin surfaces are absent.
//! RO:SECURITY — test is compiled and run with default features disabled, proving the router needs no DevKms.
//! RO:TEST — cargo test -p svc-passport --no-default-features --test crabnode_cn3_profile_only_router.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};

use svc_passport::http::router::build_profile_router;
use tower::ServiceExt;

fn request(method: Method, path: &str, body: &'static str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .expect("build request")
}

#[tokio::test]
async fn profile_only_router_has_identity_without_kms_authority() {
    let app = build_profile_router();

    let health = app
        .clone()
        .oneshot(request(Method::GET, "/healthz", ""))
        .await
        .expect("health response");

    assert_eq!(health.status(), StatusCode::OK,);

    let claim = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/v1/passport/profile/claim",
            r#"{
                        "passport_subject":"passport:main:cn3profile",
                        "requested_username":"cn3profile",
                        "display_name":"CN3 Profile",
                        "bio":"CrabNode CN-3 identity route",
                        "avatar_image":null
                    }"#,
        ))
        .await
        .expect("claim response");

    assert_eq!(claim.status(), StatusCode::CREATED,);

    let profile = app
        .clone()
        .oneshot(request(Method::GET, "/v1/passport/profile/cn3profile", ""))
        .await
        .expect("profile response");

    assert_eq!(profile.status(), StatusCode::OK,);

    for (method, path) in [
        (Method::POST, "/v1/passport/issue"),
        (Method::POST, "/v1/passport/verify"),
        (Method::POST, "/v1/passport/verify_batch"),
        (Method::GET, "/v1/keys"),
        (Method::POST, "/admin/rotate"),
        (Method::GET, "/admin/attest"),
        (Method::GET, "/metrics"),
    ] {
        let response = app
            .clone()
            .oneshot(request(method, path, "{}"))
            .await
            .expect("forbidden surface response");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} must not exist on CrabNode profile-only svc-passport",
        );
    }
}
