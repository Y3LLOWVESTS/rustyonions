//! RO:WHAT — CN-4 HTTP boundary acceptance for the fixed Native Passport `ProveSession` challenge/proof pair.
//! RO:WHY — An already root-authorized device must pass the reviewed possession seam before capability issuance or username/profile mutation.
//! RO:INTERACTS — constrained Native Passport router, durable ron-kms service identity, fixed device-session handlers, strict JSON extraction, and private possession runtime.
//! RO:INVARIANTS — both routes are POST-only and 16 KiB bounded; challenge purpose/trusted context/time remain server-owned; device public key is never caller authority; malformed or authority-injecting JSON fails closed; unrelated routes remain absent.
//! RO:METRICS — none.
//! RO:CONFIG — temporary durable test roots and private-beta trusted context.
//! RO:SECURITY — no physical Passport, RecoveryRoot, device private key, capability issuance, username/profile mutation, wallet mutation, or ledger mutation.
//! RO:TEST — `cargo test -p svc-passport --no-default-features --features native-passport --test crabnode_cn4_device_session_route`.

#![cfg(feature = "native-passport")]
#![forbid(unsafe_code)]

use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use ron_kms::DurableEd25519ServiceKey;
use serde_json::{json, Value};
use tower::ServiceExt;

use svc_passport::{
    http::router::build_native_profile_router_with_store_and_kms,
    kms::{client::KmsClient, DurableRonKmsClient},
    native::NativePassportServerRuntimeMountConfigV1,
    profile::UsernameClaimStore,
};

const PASSPORT_ID: &str =
    "passport:v1:main:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const DEVICE_ID: &str =
    "device:v1:ed25519:b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct TestDirectory {
    root: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();

        Self {
            root: std::env::temp_dir().join(format!(
                "svc-passport-cn4-device-session-route-{label}-{}-{stamp}",
                std::process::id(),
            )),
        }
    }

    fn config(&self) -> NativePassportServerRuntimeMountConfigV1 {
        NativePassportServerRuntimeMountConfigV1 {
            challenge_root: self.root.join("challenges"),
            registry_root: self.root.join("registry"),
            transaction_root: self.root.join("root-registration-redo"),
            network_id: "rustyonions-devnet".to_owned(),
            environment: "private-beta".to_owned(),
            audience: "svc-passport".to_owned(),
            issuing_service_id: "svc-passport".to_owned(),
            challenge_ttl_ms: 60_000,
            replay_retention_ms: 120_000,
            trusted_initial_root_key_epoch: 0,
        }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn durable_kms(directory: &TestDirectory) -> Arc<dyn KmsClient> {
    let key = DurableEd25519ServiceKey::open_or_create(
        directory.root.join("service-key"),
        "crabnode",
        "svc-passport",
    )
    .expect("durable test service key");

    Arc::new(DurableRonKmsClient::new(Arc::new(key)).expect("durable svc-passport KMS adapter"))
}

async fn app(directory: &TestDirectory) -> Router {
    build_native_profile_router_with_store_and_kms(
        Arc::new(UsernameClaimStore::new()),
        durable_kms(directory),
        directory.config(),
    )
    .await
    .expect("Native Passport router")
}

fn request(method: Method, path: &str, body: impl Into<Body>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(body.into())
        .expect("request")
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("bounded response body");

    serde_json::from_slice(&bytes).expect("JSON response")
}

#[tokio::test]
async fn exact_device_session_routes_exist_and_are_post_only() {
    let directory = TestDirectory::new("methods");
    let app = app(&directory).await;

    for path in ["/v1/passport/challenge", "/v1/passport/prove"] {
        let response = app
            .clone()
            .oneshot(request(Method::GET, path, Body::empty()))
            .await
            .expect("method response");

        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{path} must exist but remain POST-only",
        );
    }
}

#[tokio::test]
async fn challenge_rejects_caller_selected_purpose_and_trusted_context() {
    let directory = TestDirectory::new("challenge-authority");

    let app = app(&directory).await;

    let response = app
        .oneshot(request(
            Method::POST,
            "/v1/passport/challenge",
            json!({
                "passport_id": PASSPORT_ID,
                "device_id": DEVICE_ID,
                "requested_scopes": [
                    "catalog.read",
                    "identity.read"
                ],
                "purpose": "issue_capability",
                "network_id": "attacker-net",
                "issued_at_ms": 1,
                "service_key_id":
                    "ed25519/attacker/v1"
            })
            .to_string(),
        ))
        .await
        .expect("challenge authority response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST,);
}

#[tokio::test]
async fn unknown_registered_device_fails_closed() {
    let directory = TestDirectory::new("unknown-device");

    let app = app(&directory).await;

    let response = app
        .oneshot(request(
            Method::POST,
            "/v1/passport/challenge",
            json!({
                "passport_id": PASSPORT_ID,
                "device_id": DEVICE_ID,
                "requested_scopes": [
                    "catalog.read",
                    "identity.read"
                ]
            })
            .to_string(),
        ))
        .await
        .expect("unknown device response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND,);

    let problem = body_json(response).await;

    assert_eq!(
        problem["schema"],
        "svc-passport.native-device-session-problem.v1",
    );

    assert_eq!(problem["code"], "device_not_registered",);

    assert_eq!(problem["retryable"], false,);
}

#[tokio::test]
async fn malformed_and_authority_injecting_proof_bodies_fail_closed() {
    let directory = TestDirectory::new("proof-authority");

    let app = app(&directory).await;

    let malformed = app
        .clone()
        .oneshot(request(Method::POST, "/v1/passport/prove", "{}"))
        .await
        .expect("malformed proof response");

    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST,);

    let injected = app
        .oneshot(request(
            Method::POST,
            "/v1/passport/prove",
            json!({
                "challenge": {},
                "proof_created_at_ms": 1,
                "proof_signature":
                    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                "device_public_key":
                    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
            })
            .to_string(),
        ))
        .await
        .expect("proof authority response");

    assert_eq!(injected.status(), StatusCode::BAD_REQUEST,);
}

#[tokio::test]
async fn body_caps_and_adjacent_routes_fail_closed() {
    let directory = TestDirectory::new("surface");
    let app = app(&directory).await;

    let oversized = format!(r#"{{"padding":"{}"}}"#, "x".repeat(17_000),);

    for path in ["/v1/passport/challenge", "/v1/passport/prove"] {
        let response = app
            .clone()
            .oneshot(request(Method::POST, path, oversized.clone()))
            .await
            .expect("oversized response");

        assert_eq!(
            response.status(),
            StatusCode::PAYLOAD_TOO_LARGE,
            "{path} must enforce the 16 KiB body cap",
        );
    }

    for path in [
        "/v1/passport/session/challenge",
        "/v1/passport/session/prove",
        "/v1/passport/device/prove",
    ] {
        let response = app
            .clone()
            .oneshot(request(Method::POST, path, "{}"))
            .await
            .expect("adjacent route response");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} must remain absent",
        );
    }
}
