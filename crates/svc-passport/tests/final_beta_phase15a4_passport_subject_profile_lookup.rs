//! RO:WHAT — FINAL_BETA Phase 15A4A2A Passport-subject public-profile lookup.
//! RO:WHY — Omnigate needs svc-passport-confirmed creator identity before writing public publication projections.
//! RO:INTERACTS — UsernameClaimStore reverse claim index and svc-passport profile HTTP routes.
//! RO:INVARIANTS — username comes from the existing claim store only; no username derivation from Passport text or request headers.
//! RO:SECURITY — read-only public identity projection; no wallet, ledger, secret, PIN, recovery, signing, or capability authority.

use axum::{
    body::{
        to_bytes,
        Body,
    },
    http::{
        self,
        Request,
    },
};

use serde_json::{
    json,
    Value,
};

use tower::ServiceExt;

use svc_passport::{
    health::Health,

    http::router::build_router,

    profile::{
        UsernameClaimRequest,
        UsernameClaimStore,
        UsernameClaimStatus,
    },
};

#[path = "../src/test_support.rs"]
mod test_support;

use test_support::default_config;

const SUBJECT: &str =
    "passport:main:forum_creator";

const USERNAME: &str =
    "forum_creator";

fn claim_request() -> UsernameClaimRequest {
    UsernameClaimRequest {
        passport_subject:
            SUBJECT.to_owned(),

        requested_username:
            USERNAME.to_owned(),

        display_name:
            Some(
                "Forum Creator"
                    .to_owned(),
            ),

        bio:
            Some(
                "Forum publication identity."
                    .to_owned(),
            ),

        avatar_image:
            None,
    }
}

async fn response_json(
    response: axum::response::Response,
) -> Value {
    let body =
        to_bytes(
            response.into_body(),
            usize::MAX,
        )
        .await
        .expect(
            "response body",
        );

    serde_json::from_slice(
        &body,
    )
    .expect(
        "response JSON",
    )
}

#[test]
fn phase15a4a2a_core_reverse_lookup_returns_confirmed_claimed_profile() {
    let store =
        UsernameClaimStore::new();

    store
        .claim_main_username(
            claim_request(),
            1_776_000_000_000,
        )
        .expect(
            "claim succeeds",
        );

    let profile =
        store
            .public_profile_for_passport_subject(
                SUBJECT,
            )
            .expect(
                "subject lookup succeeds",
            )
            .expect(
                "profile exists",
            );

    assert_eq!(
        profile.passport_subject,
        SUBJECT,
    );

    assert_eq!(
        profile.username,
        USERNAME,
    );

    assert_eq!(
        profile.profile_crab_url,
        "crab://@forum_creator",
    );

    assert_eq!(
        profile.username_status,
        UsernameClaimStatus::Confirmed,
    );
}

#[test]
fn phase15a4a2a_unknown_passport_subject_returns_no_profile_without_guessing_username() {
    let store =
        UsernameClaimStore::new();

    let profile =
        store
            .public_profile_for_passport_subject(
                "passport:main:unknown_creator",
            )
            .expect(
                "unknown lookup is valid",
            );

    assert!(
        profile.is_none(),
    );
}

#[tokio::test]
async fn phase15a4a2a_http_claim_then_subject_lookup_returns_same_confirmed_identity() {
    let app =
        build_router(
            default_config(),
            Health::default(),
        );

    let claim =
        Request::builder()
            .method(
                http::Method::POST,
            )
            .uri(
                "/v1/passport/profile/claim",
            )
            .header(
                http::header::CONTENT_TYPE,
                "application/json",
            )
            .body(
                Body::from(
                    serde_json::to_vec(
                        &json!({
                            "passport_subject": SUBJECT,
                            "requested_username": USERNAME,
                            "display_name": "Forum Creator",
                            "bio": "Forum publication identity."
                        }),
                    )
                    .expect(
                        "claim body",
                    ),
                ),
            )
            .expect(
                "claim request",
            );

    let claim_response =
        app
            .clone()
            .oneshot(
                claim,
            )
            .await
            .expect(
                "claim response",
            );

    assert_eq!(
        claim_response.status(),
        http::StatusCode::CREATED,
    );

    let lookup =
        Request::builder()
            .method(
                http::Method::GET,
            )
            .uri(
                "/v1/passport/profile/by-subject/passport%3Amain%3Aforum_creator",
            )
            .body(
                Body::empty(),
            )
            .expect(
                "lookup request",
            );

    let lookup_response =
        app
            .clone()
            .oneshot(
                lookup,
            )
            .await
            .expect(
                "lookup response",
            );

    assert_eq!(
        lookup_response.status(),
        http::StatusCode::OK,
    );

    let body =
        response_json(
            lookup_response,
        )
        .await;

    assert_eq!(
        body["passport_subject"],
        SUBJECT,
    );

    assert_eq!(
        body["username"],
        USERNAME,
    );

    assert_eq!(
        body["profile_crab_url"],
        "crab://@forum_creator",
    );

    assert_eq!(
        body["username_status"],
        "confirmed",
    );
}

#[tokio::test]
async fn phase15a4a2a_http_unknown_subject_returns_not_found() {
    let app =
        build_router(
            default_config(),
            Health::default(),
        );

    let request =
        Request::builder()
            .method(
                http::Method::GET,
            )
            .uri(
                "/v1/passport/profile/by-subject/passport%3Amain%3Amissing_creator",
            )
            .body(
                Body::empty(),
            )
            .expect(
                "lookup request",
            );

    let response =
        app
            .oneshot(
                request,
            )
            .await
            .expect(
                "lookup response",
            );

    assert_eq!(
        response.status(),
        http::StatusCode::NOT_FOUND,
    );

    let body =
        response_json(
            response,
        )
        .await;

    assert_eq!(
        body["code"],
        "profile_not_found",
    );
}

#[test]
fn phase15a4a2a_source_boundary_keeps_subject_lookup_read_only_and_store_derived() {
    let core_source =
        include_str!(
            "../src/profile.rs",
        );

    let handler_source =
        include_str!(
            "../src/http/handlers/profile.rs",
        );

    let router_source =
        include_str!(
            "../src/http/router.rs",
        );

    for required in [
        "public_profile_for_passport_subject",
        "by_passport_subject",
        "by_username",
        "passport and username indexes disagree",
    ] {
        assert!(
            core_source.contains(
                required,
            ),
            "missing reverse identity marker: {required}",
        );
    }

    for required in [
        "get_profile_by_passport_subject",
        "/v1/passport/profile/by-subject/:passport_subject",
    ] {
        assert!(
            handler_source.contains(
                required,
            )
                || router_source.contains(
                    required,
                ),
            "missing subject-profile route marker: {required}",
        );
    }

    for forbidden in [
        "x-ron-username",
        "x-ron-handle",
        "requested_username_from_header",
        "username_from_passport_subject",
        "split(':').last()",
        "wallet.spend",
        "ledger.write",
        "mint_roc",
        "burn_roc",
        "seed_phrase",
        "private_key",
    ] {
        assert!(
            !core_source.contains(
                forbidden,
            ),
            "subject lookup core crossed authority boundary: {forbidden}",
        );
    }
}
