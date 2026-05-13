//! passport_profile_routes.rs — integration tests for Omnigate → svc-passport public profile proxy.
//!
//! RO:WHAT — Spin up dummy svc-passport and real Omnigate route; assert profile claim/read proxy contracts.
//! RO:WHY — NEXT_LEVEL Phase 4 must keep CrabLink gateway-only while exposing profile lookup through Omnigate.
//! RO:INTERACTS — omnigate::routes::v1::profile, svc-passport `/v1/passport/profile/*`, reqwest client.
//! RO:INVARIANTS — Omnigate proxies only; no wallet mutation; no ledger mutation; no private identity leakage.
//! RO:METRICS — route metrics are covered when mounted through full App::build.
//! RO:CONFIG — OMNIGATE_PASSPORT_BASE_URL, OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL.
//! RO:SECURITY — verifies selected headers reach svc-passport and hop-by-hop headers do not.
//! RO:TEST — cargo test -p omnigate --test passport_profile_routes.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    body::Bytes,
    extract::Path,
    http::{HeaderMap, StatusCode, Uri},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Serialize)]
struct ClaimEcho {
    schema: &'static str,
    passport_subject: String,
    passport_kind: &'static str,
    username: String,
    handle: String,
    username_status: &'static str,
    profile_crab_url: String,
    reputation_score: Option<u32>,
    moderator_score: Option<u32>,
    warnings: Vec<&'static str>,
    authorization: Option<String>,
    x_ron_passport: Option<String>,
    x_ron_wallet_account: Option<String>,
    idempotency_key: Option<String>,
    x_correlation_id: Option<String>,
    x_request_id: Option<String>,
    host: Option<String>,
    connection: Option<String>,
}

#[derive(Debug, Serialize)]
struct Problem {
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
}

async fn start_dummy_passport() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn claim_handler(headers: HeaderMap, body: Bytes) -> (StatusCode, Json<Value>) {
        let value: Value = match serde_json::from_slice(&body) {
            Ok(value) => value,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!(Problem {
                        code: "bad_json",
                        message: "request body was not valid JSON",
                        retryable: false,
                        reason: "bad_json",
                    })),
                );
            }
        };

        let username_raw = value
            .get("requested_username")
            .and_then(Value::as_str)
            .unwrap_or_default();

        let username = username_raw
            .trim()
            .trim_start_matches('@')
            .to_ascii_lowercase();

        if username == "site" {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!(Problem {
                    code: "reserved_username",
                    message: "username is reserved",
                    retryable: false,
                    reason: "reserved_username",
                })),
            );
        }

        if username == "taken" {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!(Problem {
                    code: "username_unavailable",
                    message: "username is unavailable",
                    retryable: false,
                    reason: "username_unavailable",
                })),
            );
        }

        let passport_subject = value
            .get("passport_subject")
            .and_then(Value::as_str)
            .unwrap_or("passport:main:dev")
            .to_owned();

        (
            StatusCode::CREATED,
            Json(serde_json::json!(ClaimEcho {
                schema: "svc-passport.public-profile.v1",
                passport_subject,
                passport_kind: "main",
                username: username.clone(),
                handle: format!("@{username}"),
                username_status: "confirmed",
                profile_crab_url: format!("crab://@{username}"),
                reputation_score: None,
                moderator_score: None,
                warnings: vec![
                    "public profile is read-only",
                    "reputation and moderation scores are not computed yet",
                ],
                authorization: grab(&headers, "authorization"),
                x_ron_passport: grab(&headers, "x-ron-passport"),
                x_ron_wallet_account: grab(&headers, "x-ron-wallet-account"),
                idempotency_key: grab(&headers, "idempotency-key"),
                x_correlation_id: grab(&headers, "x-correlation-id"),
                x_request_id: grab(&headers, "x-request-id"),
                host: grab(&headers, "host"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    async fn get_handler(
        Path(username): Path<String>,
        headers: HeaderMap,
        uri: Uri,
    ) -> (StatusCode, Json<Value>) {
        let _query = parse_query(&uri);

        if username == "missing" {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!(Problem {
                    code: "profile_not_found",
                    message: "public profile was not found",
                    retryable: false,
                    reason: "profile_not_found",
                })),
            );
        }

        (
            StatusCode::OK,
            Json(serde_json::json!(ClaimEcho {
                schema: "svc-passport.public-profile.v1",
                passport_subject: format!("passport:main:{username}"),
                passport_kind: "main",
                username: username.clone(),
                handle: format!("@{username}"),
                username_status: "confirmed",
                profile_crab_url: format!("crab://@{username}"),
                reputation_score: None,
                moderator_score: None,
                warnings: vec!["public profile is read-only"],
                authorization: grab(&headers, "authorization"),
                x_ron_passport: grab(&headers, "x-ron-passport"),
                x_ron_wallet_account: grab(&headers, "x-ron-wallet-account"),
                idempotency_key: grab(&headers, "idempotency-key"),
                x_correlation_id: grab(&headers, "x-correlation-id"),
                x_request_id: grab(&headers, "x-request-id"),
                host: grab(&headers, "host"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/passport/profile/claim", post(claim_handler))
        .route("/v1/passport/profile/:username", get(get_handler));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy passport");
    let addr = listener.local_addr().expect("dummy passport local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy passport serve");
    });

    wait_for_health(format!("http://{addr}/healthz")).await;
    addr
}

async fn start_omnigate_profile_route(passport_addr: SocketAddr) -> SocketAddr {
    let passport_base = format!("http://{passport_addr}");
    std::env::set_var("OMNIGATE_PASSPORT_BASE_URL", passport_base);

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate profile route");
    let addr = listener
        .local_addr()
        .expect("omnigate profile route local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate profile route serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    addr
}

#[tokio::test]
async fn profile_claim_proxies_to_svc_passport() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let passport_addr = start_dummy_passport().await;
    let omnigate_addr = start_omnigate_profile_route(passport_addr).await;

    let request = serde_json::json!({
        "passport_subject": "passport:main:skinnycrabby",
        "requested_username": "@SkinnyCrabby",
        "display_name": "Skinny Crabby",
        "bio": "Building the content-addressed creator web.",
        "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://{omnigate_addr}/v1/identity/passport/profile/claim"
        ))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:skinnycrabby")
        .header("x-ron-wallet-account", "acct_dev")
        .header("idempotency-key", "idem-profile-claim-1")
        .header("x-correlation-id", "corr-profile-claim")
        .header("x-request-id", "req-profile-claim")
        .header("connection", "close")
        .json(&request)
        .send()
        .await
        .expect("omnigate profile claim response");

    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: Value = resp.json().await.expect("parse profile claim JSON body");

    assert_eq!(body["schema"], "svc-passport.public-profile.v1");
    assert_eq!(body["passport_subject"], "passport:main:skinnycrabby");
    assert_eq!(body["username"], "skinnycrabby");
    assert_eq!(body["handle"], "@skinnycrabby");
    assert_eq!(body["username_status"], "confirmed");
    assert_eq!(body["profile_crab_url"], "crab://@skinnycrabby");
    assert!(body["reputation_score"].is_null());
    assert!(body["moderator_score"].is_null());

    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["x_ron_passport"], "passport:main:skinnycrabby");
    assert_eq!(body["x_ron_wallet_account"], "acct_dev");
    assert_eq!(body["idempotency_key"], "idem-profile-claim-1");
    assert_eq!(body["x_correlation_id"], "corr-profile-claim");
    assert_eq!(body["x_request_id"], "req-profile-claim");
    assert!(body["connection"].is_null());

    let encoded = serde_json::to_string(&body).unwrap();
    assert!(!encoded.contains("private_key"));
    assert!(!encoded.contains("seed_phrase"));
    assert!(!encoded.contains("spend_authority"));
    assert!(!encoded.contains("wallet_spend_authority"));
    assert!(!encoded.contains("private_alt_mapping"));
    assert!(!encoded.contains("parent_passport"));

    clear_env();
}

#[tokio::test]
async fn profile_lookup_proxies_to_svc_passport() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let passport_addr = start_dummy_passport().await;
    let omnigate_addr = start_omnigate_profile_route(passport_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "http://{omnigate_addr}/v1/identity/passport/profile/skinnycrabby"
        ))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:skinnycrabby")
        .header("x-ron-wallet-account", "acct_dev")
        .header("x-correlation-id", "corr-profile-get")
        .header("x-request-id", "req-profile-get")
        .send()
        .await
        .expect("omnigate profile get response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse profile get JSON body");

    assert_eq!(body["schema"], "svc-passport.public-profile.v1");
    assert_eq!(body["passport_subject"], "passport:main:skinnycrabby");
    assert_eq!(body["username"], "skinnycrabby");
    assert_eq!(body["handle"], "@skinnycrabby");
    assert_eq!(body["profile_crab_url"], "crab://@skinnycrabby");

    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["x_ron_passport"], "passport:main:skinnycrabby");
    assert_eq!(body["x_ron_wallet_account"], "acct_dev");
    assert_eq!(body["x_correlation_id"], "corr-profile-get");
    assert_eq!(body["x_request_id"], "req-profile-get");
    assert!(body["connection"].is_null());

    clear_env();
}

#[tokio::test]
async fn profile_claim_preserves_svc_passport_conflict() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let passport_addr = start_dummy_passport().await;
    let omnigate_addr = start_omnigate_profile_route(passport_addr).await;

    let request = serde_json::json!({
        "passport_subject": "passport:main:taken2",
        "requested_username": "taken"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://{omnigate_addr}/v1/identity/passport/profile/claim"
        ))
        .json(&request)
        .send()
        .await
        .expect("omnigate profile claim conflict response");

    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let body: Value = resp.json().await.expect("parse conflict JSON body");

    assert_eq!(body["code"], "username_unavailable");
    assert_eq!(body["retryable"], false);

    clear_env();
}

#[tokio::test]
async fn profile_lookup_preserves_svc_passport_404() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let passport_addr = start_dummy_passport().await;
    let omnigate_addr = start_omnigate_profile_route(passport_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "http://{omnigate_addr}/v1/identity/passport/profile/missing"
        ))
        .send()
        .await
        .expect("omnigate profile not-found response");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let body: Value = resp.json().await.expect("parse not found JSON body");

    assert_eq!(body["code"], "profile_not_found");
    assert_eq!(body["retryable"], false);

    clear_env();
}

#[tokio::test]
async fn profile_upstream_connect_failure_yields_problem_502() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    std::env::set_var("OMNIGATE_PASSPORT_BASE_URL", "http://127.0.0.1:9");

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate profile route");
    let omnigate_addr = listener
        .local_addr()
        .expect("omnigate profile route local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate profile route serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "http://{omnigate_addr}/v1/identity/passport/profile/skinnycrabby"
        ))
        .send()
        .await
        .expect("omnigate profile connect failure response");

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    let body: Value = resp.json().await.expect("parse 502 JSON body");

    assert_eq!(body["code"], "passport_upstream");
    assert_eq!(body["retryable"], true);

    clear_env();
}

fn parse_query(uri: &Uri) -> HashMap<String, String> {
    uri.query()
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

async fn wait_for_health(url: String) {
    let client = reqwest::Client::new();

    for _ in 0..40 {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    panic!("dummy passport did not become healthy at {url}");
}

fn clear_env() {
    std::env::remove_var("OMNIGATE_PASSPORT_BASE_URL");
    std::env::remove_var("OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL");
}
