//! RO:WHAT — CN-4 HTTP acceptance tests for the constrained RegisterRoot trust-anchor and challenge routes.
//! RO:WHY — Prove that CrabNode's svc-passport composition exposes one public-safe local trust anchor and one real durable RegisterRoot challenge without widening into unreviewed KMS/full-router authority.
//! RO:INTERACTS — recovery-gated router, durable ron-kms adapter, canonical `PassportChallengeV1`, and strict Axum JSON extraction.
//! RO:INVARIANTS — purpose/trusted context/time/KID are server-owned; successful challenges are canonical and distinct; malformed or authority-injecting JSON fails closed; unreviewed full-router/KMS surfaces remain absent.
//! RO:SECURITY — temporary test custody only; no secret material is returned, printed, or asserted.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test crabnode_cn4_register_root_challenge_route.

#![cfg(feature = "native-passport")]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
    Router,
};
use ron_auth::native_passport::{
    verify_passport_challenge_v1_strict, PassportChallengeVerificationContextV1,
};
use ron_kms::DurableEd25519ServiceKey;
use ron_proto::{PassportChallengePurposeV1, PassportChallengeV1};
use serde_json::json;
use tower::ServiceExt;

use svc_passport::{
    http::{
        handlers::native_register_root_trust_anchor::{
            NativeRegisterRootTrustAnchorV1, NATIVE_REGISTER_ROOT_TRUST_ANCHOR_SCHEMA_V1,
        },
        router::build_native_profile_router_with_store_and_kms,
    },
    kms::DurableRonKmsClient,
    native::NativePassportServerRuntimeMountConfigV1,
    profile::UsernameClaimStore,
};

const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

struct TestDirectory {
    root: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();

        Self {
            root: std::env::temp_dir().join(format!(
                "svc-passport-cn4-register-root-challenge-{label}-{}-{stamp}",
                std::process::id(),
            )),
        }
    }

    fn path(&self) -> &Path {
        &self.root
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

async fn app(directory: &TestDirectory) -> Router {
    let key = DurableEd25519ServiceKey::open_or_create(
        directory.root.join("service-key"),
        "crabnode",
        "svc-passport",
    )
    .expect("durable service key");

    let kms = DurableRonKmsClient::new(Arc::new(key)).expect("durable KMS adapter");

    build_native_profile_router_with_store_and_kms(
        Arc::new(UsernameClaimStore::new()),
        Arc::new(kms),
        directory.config(),
    )
    .await
    .expect("recovery-gated router")
}

fn valid_body() -> serde_json::Value {
    json!({
        "passport_id":
            format!(
                "passport:v1:main:ed25519:b3:{HEX_A}"
            ),

        "requested_scopes": [
            "identity.read",
            "profile.read"
        ],

        "operation_body_hash":
            HEX_D
    })
}

fn request(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).expect("JSON body")))
        .expect("request")
}

async fn challenge_from_response(response: axum::response::Response) -> PassportChallengeV1 {
    assert_eq!(response.status(), StatusCode::OK,);

    let bytes = to_bytes(response.into_body(), 128 * 1024)
        .await
        .expect("response bytes");

    serde_json::from_slice(&bytes).expect("canonical challenge response")
}

#[tokio::test]
async fn register_root_route_returns_real_server_bound_challenge() {
    let directory = TestDirectory::new("green");

    let app = app(&directory).await;

    let first = challenge_from_response(
        app.clone()
            .oneshot(request("/v1/passport/register/challenge", valid_body()))
            .await
            .expect("first response"),
    )
    .await;

    first.validate().expect("canonical challenge");

    assert_eq!(first.purpose, PassportChallengePurposeV1::RegisterRoot,);

    assert_eq!(first.network_id.as_str(), "rustyonions-devnet",);

    assert_eq!(first.environment.as_str(), "private-beta",);

    assert_eq!(first.audience.as_str(), "svc-passport",);

    assert_eq!(first.issuing_service_id.as_str(), "svc-passport",);

    assert!(first
        .service_key_id
        .as_str()
        .starts_with("ed25519/crabnode/svc-passport/",),);

    assert_eq!(
        first
            .passport_id
            .as_ref()
            .expect("Passport binding")
            .as_str(),
        format!("passport:v1:main:ed25519:b3:{HEX_A}"),
    );

    assert!(
        first.device_id.is_none(),
        "RegisterRoot challenge must not acquire device binding",
    );

    assert_eq!(
        first
            .operation_body_hash
            .as_ref()
            .expect("operation hash")
            .as_str(),
        HEX_D,
    );

    assert_eq!(
        first
            .requested_scopes
            .iter()
            .map(|scope| scope.as_str())
            .collect::<Vec<_>>(),
        vec!["identity.read", "profile.read",],
    );

    assert_eq!(first.expires_at_ms - first.issued_at_ms, 60_000,);

    let second = challenge_from_response(
        app.oneshot(request("/v1/passport/register/challenge", valid_body()))
            .await
            .expect("second response"),
    )
    .await;

    assert_ne!(
        first.challenge_id, second.challenge_id,
        "each challenge requires fresh identity",
    );

    assert_ne!(
        first.nonce, second.nonce,
        "each challenge requires fresh nonce",
    );

    assert!(
        directory.path().join("challenges").exists(),
        "successful route must use durable challenge root",
    );
}

#[tokio::test]
async fn register_root_trust_anchor_strictly_verifies_real_challenge() {
    let directory = TestDirectory::new("trust-anchor");

    let app = app(&directory).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/v1/passport/register/trust-anchor")
                .body(Body::empty())
                .expect("trust-anchor request"),
        )
        .await
        .expect("trust-anchor response");

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), 16 * 1024)
        .await
        .expect("trust-anchor response bytes");

    let raw = std::str::from_utf8(&bytes).expect("trust-anchor response UTF-8");

    for forbidden in [
        "secret_seed",
        "private_key",
        "recovery_factor",
        "recovery_phrase",
        "\"pin\"",
        "\"vmk\"",
        "vault_bytes",
    ] {
        assert!(
            !raw.contains(forbidden),
            "public trust anchor must not expose secret field {forbidden}",
        );
    }

    let trust_anchor: NativeRegisterRootTrustAnchorV1 =
        serde_json::from_slice(&bytes).expect("strict trust-anchor response");

    assert_eq!(
        trust_anchor.schema,
        NATIVE_REGISTER_ROOT_TRUST_ANCHOR_SCHEMA_V1,
    );

    assert_eq!(trust_anchor.network_id.as_str(), "rustyonions-devnet",);

    assert_eq!(trust_anchor.environment.as_str(), "private-beta",);

    assert_eq!(trust_anchor.audience.as_str(), "svc-passport",);

    assert_eq!(trust_anchor.issuing_service_id.as_str(), "svc-passport",);

    assert_eq!(trust_anchor.challenge_ttl_ms, 60_000);
    assert_eq!(trust_anchor.trusted_initial_root_key_epoch, 0);

    let challenge = challenge_from_response(
        app.oneshot(request("/v1/passport/register/challenge", valid_body()))
            .await
            .expect("challenge response"),
    )
    .await;

    assert_eq!(
        challenge.service_key_id, trust_anchor.service_key_id,
        "challenge KID must match the explicitly provisionable trust anchor",
    );

    verify_passport_challenge_v1_strict(
        &challenge,
        PassportChallengeVerificationContextV1 {
            trusted_service_public_key: &trust_anchor.service_public_key,
            expected_network_id: &trust_anchor.network_id,
            expected_environment: &trust_anchor.environment,
            expected_audience: &trust_anchor.audience,
            expected_issuing_service_id: &trust_anchor.issuing_service_id,
            expected_service_key_id: &trust_anchor.service_key_id,
            now_ms: challenge.issued_at_ms,
            max_clock_skew_ms: 0,
        },
    )
    .expect("real challenge must verify against the provisioned trust anchor");
}

#[tokio::test]
async fn caller_cannot_inject_trusted_challenge_fields() {
    let directory = TestDirectory::new("authority-injection");

    let app = app(&directory).await;

    let mut body = valid_body();

    let object = body.as_object_mut().expect("object");

    object.insert("issued_at_ms".to_owned(), json!(1));

    object.insert("service_key_id".to_owned(), json!("ed25519/attacker/v1"));

    object.insert("network_id".to_owned(), json!("attacker-net"));

    let response = app
        .oneshot(request("/v1/passport/register/challenge", body))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST,);
}

#[tokio::test]
async fn noncanonical_scope_order_is_bad_request() {
    let directory = TestDirectory::new("scope-order");

    let app = app(&directory).await;

    let mut body = valid_body();

    body["requested_scopes"] = json!(["profile.read", "identity.read"]);

    let response = app
        .oneshot(request("/v1/passport/register/challenge", body))
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST,);
}

#[tokio::test]
async fn only_register_root_challenge_surface_is_added() {
    let directory = TestDirectory::new("surface");

    let app = app(&directory).await;

    for forbidden in [
        "/v1/passport/register",
        "/v1/passport/issue",
        "/v1/passport/verify",
        "/v1/passport/verify_batch",
        "/v1/keys",
        "/admin/rotate",
        "/admin/attest",
        "/metrics",
    ] {
        let response = app
            .clone()
            .oneshot(request(forbidden, json!({})))
            .await
            .expect("forbidden response");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{forbidden} must remain absent",
        );
    }
}
