use axum::body::to_bytes;
use axum::{body::Body, http, http::Request};
use serde_json::{json, Value};
use tower::ServiceExt;

use svc_passport::{health::Health, http::router::build_router};

#[path = "../src/test_support.rs"]
mod test_support;

use test_support::default_config;

async fn response_json(resp: axum::response::Response) -> (http::StatusCode, Value) {
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read response body");

    let value = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        panic!(
            "response body should be json: {err}; body={}",
            String::from_utf8_lossy(&bytes)
        )
    });

    (status, value)
}

fn claim_body(passport_subject: &str, username: &str) -> Value {
    json!({
        "passport_subject": passport_subject,
        "requested_username": username,
        "display_name": "Skinny Crabby",
        "bio": "Building the content-addressed creator web.",
        "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image"
    })
}

#[tokio::test]
async fn profile_debug_route_is_safe() {
    let app = build_router(default_config(), Health::default());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/passport/profile/_debug")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, json) = response_json(resp).await;

    assert!(status.is_success());
    assert_eq!(json["schema"], "svc-passport.profile-debug.v1");
    assert_eq!(json["wallet_mutation"], false);
    assert_eq!(json["ledger_mutation"], false);
    assert_eq!(json["private_keys"], false);
    assert_eq!(json["alt_linkage"], false);
}

#[tokio::test]
async fn claim_then_get_public_profile() {
    let app = build_router(default_config(), Health::default());

    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/profile/claim")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&claim_body("passport:main:skinnycrabby", "@SkinnyCrabby")).unwrap(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    let (status, json) = response_json(resp).await;

    assert_eq!(status, http::StatusCode::CREATED);
    assert_eq!(json["schema"], "svc-passport.public-profile.v1");
    assert_eq!(json["passport_subject"], "passport:main:skinnycrabby");
    assert_eq!(json["passport_kind"], "main");
    assert_eq!(json["username"], "skinnycrabby");
    assert_eq!(json["handle"], "@skinnycrabby");
    assert_eq!(json["username_status"], "confirmed");
    assert_eq!(json["profile_crab_url"], "crab://@skinnycrabby");
    assert!(json.get("reputation_score").is_none() || json["reputation_score"].is_null());
    assert!(json.get("moderator_score").is_none() || json["moderator_score"].is_null());
    assert!(serde_json::to_string(&json)
        .unwrap()
        .contains("reputation and moderation scores are not computed yet"));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/passport/profile/skinnycrabby")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, json) = response_json(resp).await;

    assert!(status.is_success());
    assert_eq!(json["username"], "skinnycrabby");
    assert_eq!(json["handle"], "@skinnycrabby");
    assert_eq!(json["profile_crab_url"], "crab://@skinnycrabby");

    let encoded = serde_json::to_string(&json).unwrap();
    assert!(!encoded.contains("private_key"));
    assert!(!encoded.contains("seed_phrase"));
    assert!(!encoded.contains("spend_authority"));
    assert!(!encoded.contains("wallet_spend_authority"));
    assert!(!encoded.contains("private_alt_mapping"));
    assert!(!encoded.contains("parent_passport"));
    assert!(!encoded.contains("main_passport_subject"));
}

#[tokio::test]
async fn duplicate_username_returns_conflict() {
    let app = build_router(default_config(), Health::default());

    for passport in ["passport:main:alice", "passport:main:alice2"] {
        let req = Request::builder()
            .method(http::Method::POST)
            .uri("/v1/passport/profile/claim")
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::to_vec(&claim_body(passport, "alice")).unwrap(),
            ))
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();
        let (status, json) = response_json(resp).await;

        if passport.ends_with("alice") {
            assert_eq!(status, http::StatusCode::CREATED);
        } else {
            assert_eq!(status, http::StatusCode::CONFLICT);
            assert_eq!(json["code"], "username_unavailable");
            assert_eq!(json["retryable"], false);
        }
    }
}

#[tokio::test]
async fn reserved_username_rejects() {
    let app = build_router(default_config(), Health::default());

    let req = Request::builder()
        .method(http::Method::POST)
        .uri("/v1/passport/profile/claim")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(&claim_body("passport:main:site", "@site")).unwrap(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let (status, json) = response_json(resp).await;

    assert_eq!(status, http::StatusCode::BAD_REQUEST);
    assert_eq!(json["code"], "reserved_username");
    assert_eq!(json["retryable"], false);
}

#[tokio::test]
async fn unknown_profile_returns_404() {
    let app = build_router(default_config(), Health::default());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/passport/profile/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let (status, json) = response_json(resp).await;

    assert_eq!(status, http::StatusCode::NOT_FOUND);
    assert_eq!(json["code"], "profile_not_found");
    assert_eq!(json["retryable"], false);
}
