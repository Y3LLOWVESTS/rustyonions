//! Identity route proxy tests for CrabLink passport bootstrap and public profiles.
//!
//! RO:WHAT — Spin up dummy `Omnigate` and real `svc-gateway`; assert identity, fixed RegisterRoot/device-session/IssueCapability ingress, wallet, and profile edge proxy behavior.
//! RO:WHY — `CrabLink` must use gateway-only paths, including the CN-4 fixed RegisterRoot, ProveSession, and IssueCapability challenge/proof ingress.
//! RO:INTERACTS — `svc_gateway::routes`, `Config`, `AppState`, `SVC_GATEWAY_OMNIGATE_BASE_URL`.
//! RO:INVARIANTS — gateway remains proxy-only; it forwards identity headers and idempotency keys.
//! RO:METRICS — exercises gateway correlation/HTTP metric layers.
//! RO:CONFIG — `SVC_GATEWAY_OMNIGATE_BASE_URL`, `SVC_GATEWAY_BIND_ADDR`.
//! RO:SECURITY — no direct `svc-passport`, `svc-wallet`, or ledger calls.
//! RO:TEST — `cargo test -p svc-gateway --test identity_routes_proxy`.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    body::Bytes,
    http::{HeaderMap, Method, StatusCode, Uri},
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::Value;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Serialize)]
struct IdentityEcho {
    method: String,
    path: String,
    query: HashMap<String, String>,
    body_len: usize,
    body: Option<Value>,
    authorization: Option<String>,
    passport: Option<String>,
    wallet_account: Option<String>,
    idempotency_key: Option<String>,
    x_request_id: Option<String>,
    x_correlation_id: Option<String>,
    connection: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    reason: &'static str,
}

fn test_metrics_handles() -> metrics::MetricsHandles {
    static CELL: OnceCell<metrics::MetricsHandles> = OnceCell::new();
    CELL.get_or_init(|| metrics::register().expect("register metrics once for test process"))
        .clone()
}

async fn start_dummy_omnigate() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn echo_handler(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
    ) -> (StatusCode, Json<Value>) {
        if uri.path().contains("problem400") {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!(ErrorBody {
                    error: "bad_request",
                    reason: "identity route rejected by omnigate",
                })),
            );
        }

        if uri.path().contains("missing") {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "code": "profile_not_found",
                    "message": "public profile was not found",
                    "retryable": false
                })),
            );
        }

        if uri.path().contains("taken") {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "code": "username_unavailable",
                    "message": "username is unavailable",
                    "retryable": false
                })),
            );
        }

        let parsed_body: Option<Value> = if body.is_empty() {
            None
        } else {
            Some(serde_json::from_slice(&body).expect("test request body should be JSON"))
        };

        if uri.path() == "/v1/identity/passport/profile/claim"
            && parsed_body
                .as_ref()
                .and_then(|body| body.get("requested_username"))
                .and_then(Value::as_str)
                .map(|username| {
                    username
                        .trim()
                        .trim_start_matches('@')
                        .eq_ignore_ascii_case("taken")
                })
                .unwrap_or(false)
        {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "code": "username_unavailable",
                    "message": "username is unavailable",
                    "retryable": false
                })),
            );
        }

        (
            StatusCode::OK,
            Json(serde_json::json!(IdentityEcho {
                method: method.to_string(),
                path: uri.path().to_owned(),
                query: parse_query(&uri),
                body_len: body.len(),
                body: parsed_body,
                authorization: grab(&headers, "authorization"),
                passport: grab(&headers, "x-ron-passport"),
                wallet_account: grab(&headers, "x-ron-wallet-account"),
                idempotency_key: grab(&headers, "idempotency-key"),
                x_request_id: grab(&headers, "x-request-id"),
                x_correlation_id: grab(&headers, "x-correlation-id"),
                connection: grab(&headers, "connection"),
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/identity/me", get(echo_handler))
        .route("/v1/identity/passport/bootstrap", post(echo_handler))
        .route(
            "/v1/identity/passport/register/challenge",
            post(echo_handler),
        )
        .route("/v1/identity/passport/register/proof", post(echo_handler))
        .route("/v1/identity/passport/device/authorize", post(echo_handler))
        .route("/v1/identity/passport/challenge", post(echo_handler))
        .route("/v1/identity/passport/prove", post(echo_handler))
        .route(
            "/v1/identity/passport/capability/challenge",
            post(echo_handler),
        )
        .route("/v1/identity/passport/capability/prove", post(echo_handler))
        .route("/v1/identity/passport/profile/claim", post(echo_handler))
        .route("/v1/identity/passport/profile/:username", get(echo_handler))
        .route("/v1/wallet/:account/balance", get(echo_handler));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy omnigate");
    let addr = listener.local_addr().expect("dummy omnigate local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy omnigate serve");
    });

    wait_for_health(format!("http://{addr}/healthz")).await;

    addr
}

async fn start_gateway(omnigate_addr: SocketAddr) -> SocketAddr {
    std::env::set_var(
        "SVC_GATEWAY_OMNIGATE_BASE_URL",
        format!("http://{omnigate_addr}"),
    );
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let cfg = Config::load().expect("load config with env overrides");
    let metrics_handles = test_metrics_handles();
    let state = AppState::new(cfg.clone(), metrics_handles);
    let router = routes::build_router(&state);

    let listener = TcpListener::bind(&cfg.server.bind_addr)
        .await
        .expect("bind gateway");
    let gateway_addr = listener.local_addr().expect("gateway local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("gateway serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    gateway_addr
}

#[tokio::test]
async fn identity_me_proxy_preserves_passport_headers() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{gateway_addr}/identity/me?debug=1"))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:dev")
        .header("x-ron-wallet-account", "acct_dev")
        .header("x-correlation-id", "corr-identity-me")
        .header("x-request-id", "req-identity-me")
        .header("connection", "close")
        .send()
        .await
        .expect("gateway identity response");

    assert!(resp.status().is_success());

    let body: Value = resp.json().await.expect("parse JSON body");

    assert_eq!(body["method"], "GET");
    assert_eq!(body["path"], "/v1/identity/me");
    assert_eq!(body["query"]["debug"], "1");
    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["passport"], "passport:main:dev");
    assert_eq!(body["wallet_account"], "acct_dev");
    assert_eq!(body["x_correlation_id"], "corr-identity-me");
    assert_eq!(body["x_request_id"], "req-identity-me");
    assert!(body["connection"].is_null());

    clear_env();
}

#[tokio::test]
async fn passport_bootstrap_proxy_preserves_body_and_idempotency() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let payload = serde_json::json!({
        "kind": "main",
        "display_name": "CrabLink main passport",
        "requested_username": "skinnycrabby",
        "starter_grant": true
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gateway_addr}/identity/passport/bootstrap"))
        .header("authorization", "Bearer dev")
        .header("content-type", "application/json")
        .header("idempotency-key", "idem-bootstrap-1")
        .header("x-ron-passport", "passport:main:dev")
        .header("x-ron-wallet-account", "acct_dev")
        .json(&payload)
        .send()
        .await
        .expect("gateway passport bootstrap response");

    assert!(resp.status().is_success());

    let body: Value = resp.json().await.expect("parse JSON body");

    assert_eq!(body["method"], "POST");
    assert_eq!(body["path"], "/v1/identity/passport/bootstrap");
    assert_eq!(body["body"], payload);
    assert_eq!(body["idempotency_key"], "idem-bootstrap-1");
    assert_eq!(body["passport"], "passport:main:dev");
    assert_eq!(body["wallet_account"], "acct_dev");

    clear_env();
}

#[tokio::test]
async fn passport_profile_claim_proxy_preserves_body_headers_and_idempotency() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let payload = serde_json::json!({
        "passport_subject": "passport:main:skinnycrabby",
        "requested_username": "@SkinnyCrabby",
        "display_name": "Skinny Crabby",
        "bio": "Building the content-addressed creator web.",
        "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://{gateway_addr}/identity/passport/profile/claim"
        ))
        .header("authorization", "Bearer dev")
        .header("content-type", "application/json")
        .header("idempotency-key", "idem-profile-claim-1")
        .header("x-ron-passport", "passport:main:skinnycrabby")
        .header("x-ron-wallet-account", "acct_dev")
        .header("x-correlation-id", "corr-profile-claim")
        .header("x-request-id", "req-profile-claim")
        .header("connection", "close")
        .json(&payload)
        .send()
        .await
        .expect("gateway profile claim response");

    assert!(resp.status().is_success());

    let body: Value = resp.json().await.expect("parse JSON body");

    assert_eq!(body["method"], "POST");
    assert_eq!(body["path"], "/v1/identity/passport/profile/claim");
    assert_eq!(body["body"], payload);
    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["passport"], "passport:main:skinnycrabby");
    assert_eq!(body["wallet_account"], "acct_dev");
    assert_eq!(body["idempotency_key"], "idem-profile-claim-1");
    assert_eq!(body["x_correlation_id"], "corr-profile-claim");
    assert_eq!(body["x_request_id"], "req-profile-claim");
    assert!(body["connection"].is_null());

    clear_env();
}

#[tokio::test]
async fn passport_profile_lookup_proxy_targets_omnigate_profile_route() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "http://{gateway_addr}/identity/passport/profile/skinnycrabby"
        ))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:skinnycrabby")
        .header("x-ron-wallet-account", "acct_dev")
        .header("x-correlation-id", "corr-profile-get")
        .header("x-request-id", "req-profile-get")
        .send()
        .await
        .expect("gateway profile lookup response");

    assert!(resp.status().is_success());

    let body: Value = resp.json().await.expect("parse JSON body");

    assert_eq!(body["method"], "GET");
    assert_eq!(body["path"], "/v1/identity/passport/profile/skinnycrabby");
    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["passport"], "passport:main:skinnycrabby");
    assert_eq!(body["wallet_account"], "acct_dev");
    assert_eq!(body["x_correlation_id"], "corr-profile-get");
    assert_eq!(body["x_request_id"], "req-profile-get");
    assert!(body["connection"].is_null());

    clear_env();
}

#[tokio::test]
async fn passport_profile_upstream_errors_pass_through() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();

    let resp = client
        .get(format!(
            "http://{gateway_addr}/identity/passport/profile/missing"
        ))
        .send()
        .await
        .expect("gateway profile missing response");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.expect("parse profile missing body");
    assert_eq!(body["code"], "profile_not_found");
    assert_eq!(body["retryable"], false);

    let payload = serde_json::json!({
        "passport_subject": "passport:main:taken2",
        "requested_username": "taken"
    });

    let resp = client
        .post(format!(
            "http://{gateway_addr}/identity/passport/profile/claim"
        ))
        .json(&payload)
        .send()
        .await
        .expect("gateway profile conflict response");

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = resp.json().await.expect("parse profile conflict body");
    assert_eq!(body["code"], "username_unavailable");
    assert_eq!(body["retryable"], false);

    clear_env();
}

#[tokio::test]
async fn wallet_balance_proxy_targets_omnigate_wallet_balance() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{gateway_addr}/wallet/acct_dev/balance"))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:dev")
        .header("x-ron-wallet-account", "acct_dev")
        .send()
        .await
        .expect("gateway wallet balance response");

    assert!(resp.status().is_success());

    let body: Value = resp.json().await.expect("parse JSON body");

    assert_eq!(body["method"], "GET");
    assert_eq!(body["path"], "/v1/wallet/acct_dev/balance");
    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["passport"], "passport:main:dev");
    assert_eq!(body["wallet_account"], "acct_dev");

    clear_env();
}

#[tokio::test]
async fn identity_route_upstream_connect_failure_yields_problem_502() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    std::env::set_var("SVC_GATEWAY_OMNIGATE_BASE_URL", "http://127.0.0.1:1");
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let cfg = Config::load().expect("load config with env overrides");
    let metrics_handles = test_metrics_handles();
    let state = AppState::new(cfg.clone(), metrics_handles);
    let router = routes::build_router(&state);

    let listener = TcpListener::bind(&cfg.server.bind_addr)
        .await
        .expect("bind gateway");
    let gateway_addr = listener.local_addr().expect("gateway local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("gateway serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "http://{gateway_addr}/identity/passport/profile/skinnycrabby"
        ))
        .send()
        .await
        .expect("gateway profile upstream failure response");

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    let body: Value = resp.json().await.expect("parse JSON Problem body");
    assert_eq!(body["code"], "upstream_unavailable");
    assert_eq!(body["retryable"], true);

    clear_env();
}

fn parse_query(uri: &Uri) -> HashMap<String, String> {
    uri.query()
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key.to_owned(), value.to_owned())
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

    panic!("dummy omnigate did not become healthy at {url}");
}

fn clear_env() {
    std::env::remove_var("SVC_GATEWAY_OMNIGATE_BASE_URL");
    std::env::remove_var("SVC_GATEWAY_BIND_ADDR");
}

#[tokio::test]
async fn cn4_register_root_challenge_proxies_to_fixed_omnigate_path() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;

    let gateway_addr = start_gateway(omnigate_addr).await;

    let payload = serde_json::json!({
        "passport_id":
            "passport:v1:main:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",

        "requested_scopes": [
            "identity.read",
            "profile.read"
        ],

        "operation_body_hash":
            "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
    });

    let response = reqwest::Client::new()
        .post(format!(
            "http://{gateway_addr}/identity/passport/register/challenge"
        ))
        .header("authorization", "Bearer cn4-test")
        .header("content-type", "application/json")
        .header("idempotency-key", "cn4-register-root-1")
        .header("x-correlation-id", "corr-cn4-register-root")
        .header("x-request-id", "req-cn4-register-root")
        .header("connection", "close")
        .json(&payload)
        .send()
        .await
        .expect("gateway RegisterRoot challenge response");

    assert_eq!(response.status(), StatusCode::OK,);

    let body: Value = response.json().await.expect("gateway proxy response JSON");

    assert_eq!(body["method"], "POST",);

    assert_eq!(body["path"], "/v1/identity/passport/register/challenge",);

    assert_eq!(body["body"], payload,);

    assert_eq!(body["authorization"], "Bearer cn4-test",);

    assert_eq!(body["idempotency_key"], "cn4-register-root-1",);

    assert_eq!(body["x_correlation_id"], "corr-cn4-register-root",);

    assert_eq!(body["x_request_id"], "req-cn4-register-root",);

    assert!(
        body["connection"].is_null(),
        "gateway must not forward the caller's hop-by-hop Connection header",
    );

    clear_env();
}

#[tokio::test]
async fn cn4_register_root_proof_proxies_to_fixed_omnigate_path() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let payload = serde_json::json!({
        "challenge_id":
            "challenge:v1:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "root_public_key":
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "root_signature":
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
    });

    let response = reqwest::Client::new()
        .post(format!(
            "http://{gateway_addr}/identity/passport/register/proof"
        ))
        .header("authorization", "Bearer cn4-proof-test")
        .header("content-type", "application/json")
        .header("idempotency-key", "cn4-register-root-proof-1")
        .header("x-correlation-id", "corr-cn4-register-root-proof")
        .header("x-request-id", "req-cn4-register-root-proof")
        .header("connection", "close")
        .json(&payload)
        .send()
        .await
        .expect("gateway RegisterRoot proof response");

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response
        .json()
        .await
        .expect("gateway RegisterRoot proof proxy JSON");

    assert_eq!(body["method"], "POST");
    assert_eq!(body["path"], "/v1/identity/passport/register/proof");
    assert_eq!(body["body"], payload);
    assert_eq!(body["authorization"], "Bearer cn4-proof-test");
    assert_eq!(body["idempotency_key"], "cn4-register-root-proof-1");
    assert_eq!(body["x_correlation_id"], "corr-cn4-register-root-proof");
    assert_eq!(body["x_request_id"], "req-cn4-register-root-proof");
    assert!(
        body["connection"].is_null(),
        "gateway must not forward the caller's hop-by-hop Connection header",
    );

    clear_env();
}

#[tokio::test]
async fn cn4_device_authorize_gateway_route_proxies_exactly() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;

    let gateway_addr = start_gateway(omnigate_addr).await;

    let payload = serde_json::json!({
        "authorization": {
            "opaque_test_marker":
                "cn4-device-authorize",
        },
    });

    let response = reqwest::Client::new()
        .post(format!(
            "http://{gateway_addr}/identity/passport/device/authorize"
        ))
        .header("authorization", "Bearer cn4-device-test")
        .header("content-type", "application/json")
        .header("idempotency-key", "cn4-device-authorize-1")
        .header("x-correlation-id", "corr-cn4-device-authorize")
        .header("x-request-id", "req-cn4-device-authorize")
        .header("connection", "close")
        .json(&payload)
        .send()
        .await
        .expect("gateway DeviceAuthorize response");

    assert_eq!(response.status(), StatusCode::OK,);

    let body: Value = response
        .json()
        .await
        .expect("gateway DeviceAuthorize proxy JSON");

    assert_eq!(body["method"], "POST");
    assert_eq!(body["path"], "/v1/identity/passport/device/authorize",);
    assert_eq!(body["body"], payload);
    assert_eq!(body["authorization"], "Bearer cn4-device-test",);
    assert_eq!(body["idempotency_key"], "cn4-device-authorize-1",);
    assert_eq!(body["x_correlation_id"], "corr-cn4-device-authorize",);
    assert_eq!(body["x_request_id"], "req-cn4-device-authorize",);
    assert!(
        body["connection"].is_null(),
        "gateway must filter caller hop-by-hop Connection header",
    );

    clear_env();
}

#[tokio::test]
async fn cn4_device_session_gateway_pair_proxies_exactly() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    for (public_path, upstream_path, marker) in [
        (
            "/identity/passport/challenge",
            "/v1/identity/passport/challenge",
            "device-session-challenge",
        ),
        (
            "/identity/passport/prove",
            "/v1/identity/passport/prove",
            "device-session-proof",
        ),
    ] {
        let payload = serde_json::json!({
            "opaque_test_marker": marker,
        });

        let response = reqwest::Client::new()
            .post(format!("http://{gateway_addr}{public_path}"))
            .header("authorization", "Bearer cn4-device-session")
            .header("content-type", "application/json")
            .header("idempotency-key", format!("idem-{marker}"))
            .header("x-correlation-id", format!("corr-{marker}"))
            .header("x-request-id", format!("req-{marker}"))
            .header("connection", "close")
            .json(&payload)
            .send()
            .await
            .expect("gateway ProveSession response");

        assert_eq!(response.status(), StatusCode::OK,);

        let body: Value = response.json().await.expect("gateway ProveSession JSON");

        assert_eq!(body["method"], "POST");
        assert_eq!(body["path"], upstream_path);
        assert_eq!(body["body"], payload);

        assert_eq!(body["authorization"], "Bearer cn4-device-session",);

        assert_eq!(body["idempotency_key"], format!("idem-{marker}"),);

        assert_eq!(body["x_correlation_id"], format!("corr-{marker}"),);

        assert_eq!(body["x_request_id"], format!("req-{marker}"),);

        assert!(
            body["connection"].is_null(),
            "gateway must filter caller hop-by-hop Connection header",
        );
    }

    clear_env();
}

#[tokio::test]
async fn cn4_capability_gateway_pair_proxies_exactly() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    for (public_path, upstream_path, marker) in [
        (
            "/identity/passport/capability/challenge",
            "/v1/identity/passport/capability/challenge",
            "capability-challenge",
        ),
        (
            "/identity/passport/capability/prove",
            "/v1/identity/passport/capability/prove",
            "capability-proof",
        ),
    ] {
        let payload = serde_json::json!({
            "opaque_test_marker": marker,
        });

        let response = reqwest::Client::new()
            .post(format!("http://{gateway_addr}{public_path}"))
            .header("authorization", "Bearer cn4-capability")
            .header("content-type", "application/json")
            .header("idempotency-key", format!("idem-{marker}"))
            .header("x-correlation-id", format!("corr-{marker}"))
            .header("x-request-id", format!("req-{marker}"))
            .header("connection", "close")
            .json(&payload)
            .send()
            .await
            .expect("gateway IssueCapability response");

        assert_eq!(response.status(), StatusCode::OK);

        let body: Value = response.json().await.expect("gateway IssueCapability JSON");

        assert_eq!(body["method"], "POST");
        assert_eq!(body["path"], upstream_path);
        assert_eq!(body["body"], payload);

        assert_eq!(body["authorization"], "Bearer cn4-capability",);

        assert_eq!(body["idempotency_key"], format!("idem-{marker}"),);

        assert_eq!(body["x_correlation_id"], format!("corr-{marker}"),);

        assert_eq!(body["x_request_id"], format!("req-{marker}"),);

        assert!(
            body["connection"].is_null(),
            "gateway must filter caller hop-by-hop Connection header",
        );
    }

    clear_env();
}

#[tokio::test]
async fn cn4_unreviewed_gateway_identity_routes_remain_absent() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;

    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();

    for public_path in [
        "/identity/passport/challenge/extra",
        "/identity/passport/prove/extra",
        "/identity/passport/device/register",
        "/identity/passport/capability/challenge/extra",
        "/identity/passport/capability/prove/extra",
        "/identity/passport/capability/refresh",
        "/identity/passport/capability/revoke",
    ] {
        let response = client
            .post(format!("http://{gateway_addr}{public_path}"))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .expect("generic Native Passport route response");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{public_path} must remain locally unmounted",
        );
    }

    let trust_anchor = client
        .get(format!(
            "http://{gateway_addr}/identity/passport/register/trust-anchor"
        ))
        .send()
        .await
        .expect("public trust-anchor route response");

    assert_eq!(
        trust_anchor.status(),
        StatusCode::NOT_FOUND,
        "CN-4 challenge trust anchor must remain an explicit local provisioning surface and must not be product-gateway proxied",
    );

    clear_env();
}

#[tokio::test]
async fn cn4_register_root_gateway_route_enforces_16_kib_body_cap() {
    let _guard = ENV_LOCK.lock().await;
    clear_env();

    let omnigate_addr = start_dummy_omnigate().await;

    let gateway_addr = start_gateway(omnigate_addr).await;

    for public_path in [
        "/identity/passport/register/challenge",
        "/identity/passport/register/proof",
        "/identity/passport/device/authorize",
        "/identity/passport/challenge",
        "/identity/passport/prove",
        "/identity/passport/capability/challenge",
        "/identity/passport/capability/prove",
    ] {
        let response = reqwest::Client::new()
            .post(format!("http://{gateway_addr}{public_path}"))
            .header("content-type", "application/octet-stream")
            .body(vec![b'a'; 16_385])
            .send()
            .await
            .expect("oversized fixed RegisterRoot response");

        assert_eq!(
            response.status(),
            StatusCode::PAYLOAD_TOO_LARGE,
            "{public_path} must enforce the 16 KiB body cap",
        );
    }

    clear_env();
}
