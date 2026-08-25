//! RO:WHAT — CN-4 HTTP boundary acceptance for the fixed Native Passport IssueCapability challenge/proof pair.
//! RO:WHY — Capability issuance must be explicitly mounted without widening the existing ProveSession routes or creating generic capability lifecycle authority.
//! RO:INTERACTS — capability-enabled constrained router, durable ron-kms service identity, capability recovery preflight, strict JSON extraction, and bounded HTTP bodies.
//! RO:INVARIANTS — existing builder remains capability-free; new routes are POST-only and 16 KiB bounded; purpose/TTL/policy/context are not caller fields; valid-shaped unknown-device requests reach real durable authority lookup; refresh/revoke stay absent.
//! RO:METRICS — none.
//! RO:CONFIG — temporary Native Passport roots plus dedicated capability state/redo roots and one-hour capability TTL.
//! RO:SECURITY — no real Passport secret/DeviceKey, no fake capability success, no username/profile mutation, no wallet/ledger mutation, and no caller-selected proof purpose.
//! RO:TEST — `cargo test -p svc-passport --no-default-features --features native-passport --test crabnode_cn4_capability_route`.

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
use tower::ServiceExt;

use svc_passport::{
    http::router::{
        build_native_profile_router_with_store_and_kms,
        build_native_profile_router_with_store_kms_and_capability,
    },
    kms::{client::KmsClient, DurableRonKmsClient},
    native::{
        NativePassportServerCapabilityRuntimeConfigV1, NativePassportServerRuntimeMountConfigV1,
    },
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
            .expect("clock")
            .as_nanos();

        Self {
            root: std::env::temp_dir().join(format!(
                "svc-passport-cn4-capability-http-{label}-{}-{stamp}",
                std::process::id(),
            )),
        }
    }

    fn runtime_config(&self) -> NativePassportServerRuntimeMountConfigV1 {
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

    fn capability_config(&self) -> NativePassportServerCapabilityRuntimeConfigV1 {
        NativePassportServerCapabilityRuntimeConfigV1 {
            capability_root: self.root.join("capabilities"),
            transaction_root: self.root.join("capability-issuance-redo"),
            capability_ttl_ms: 3_600_000,
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

    Arc::new(DurableRonKmsClient::new(Arc::new(key)).expect("durable KMS adapter"))
}

fn request(method: Method, uri: &str, body: impl Into<Body>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.into())
        .expect("request")
}

async fn old_app(directory: &TestDirectory) -> Router {
    build_native_profile_router_with_store_and_kms(
        Arc::new(UsernameClaimStore::new()),
        durable_kms(directory),
        directory.runtime_config(),
    )
    .await
    .expect("existing constrained router")
}

async fn capability_app(directory: &TestDirectory) -> Router {
    build_native_profile_router_with_store_kms_and_capability(
        Arc::new(UsernameClaimStore::new()),
        durable_kms(directory),
        directory.runtime_config(),
        directory.capability_config(),
    )
    .await
    .expect("capability-enabled constrained router")
}

#[tokio::test]
async fn existing_native_builder_remains_capability_free() {
    let directory = TestDirectory::new("existing-builder");

    let app = old_app(&directory).await;

    for path in [
        "/v1/passport/capability/challenge",
        "/v1/passport/capability/prove",
    ] {
        let response = app
            .clone()
            .oneshot(request(Method::POST, path, "{}"))
            .await
            .expect("response");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} must remain absent from existing builder",
        );
    }
}

#[tokio::test]
async fn capability_builder_recovers_and_mounts_only_fixed_routes() {
    let directory = TestDirectory::new("capability-builder");

    let app = capability_app(&directory).await;

    assert!(
        directory.root.join("capabilities").is_dir(),
        "capability state must be opened during preflight",
    );

    assert!(
        directory.root.join("capability-issuance-redo").is_dir(),
        "capability redo journal must be opened during preflight",
    );

    let unknown_device_request = format!(
        r#"{{"passport_id":"{PASSPORT_ID}","device_id":"{DEVICE_ID}","requested_scopes":["identity.read"]}}"#,
    );

    let unknown_device = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/v1/passport/capability/challenge",
            unknown_device_request,
        ))
        .await
        .expect("unknown-device response");

    assert_eq!(
        unknown_device.status(),
        StatusCode::NOT_FOUND,
        "valid-shaped request must reach real durable device lookup",
    );

    let authority_injection = format!(
        r#"{{"passport_id":"{PASSPORT_ID}","device_id":"{DEVICE_ID}","requested_scopes":["identity.read"],"purpose":"issue_capability","capability_ttl_ms":1,"policy_version":999}}"#,
    );

    let injected = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/v1/passport/capability/challenge",
            authority_injection,
        ))
        .await
        .expect("injected authority response");

    assert_eq!(injected.status(), StatusCode::BAD_REQUEST,);

    let malformed_proof = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/v1/passport/capability/prove",
            r#"{"challenge":{},"proof_created_at_ms":1,"proof_signature":"bad"}"#,
        ))
        .await
        .expect("malformed proof response");

    assert_eq!(malformed_proof.status(), StatusCode::BAD_REQUEST,);

    for path in [
        "/v1/passport/capability/challenge",
        "/v1/passport/capability/prove",
    ] {
        let response = app
            .clone()
            .oneshot(request(Method::GET, path, "{}"))
            .await
            .expect("GET response");

        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{path} must remain POST-only",
        );
    }

    let oversized = format!(
        r#"{{"passport_id":"{PASSPORT_ID}","device_id":"{DEVICE_ID}","requested_scopes":["identity.read"],"padding":"{}"}}"#,
        "x".repeat(16_385),
    );

    let response = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/v1/passport/capability/challenge",
            oversized,
        ))
        .await
        .expect("oversized response");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE,);

    for path in [
        "/v1/passport/capability/refresh",
        "/v1/passport/capability/revoke",
    ] {
        let response = app
            .clone()
            .oneshot(request(Method::POST, path, "{}"))
            .await
            .expect("closed adjacent route");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} must remain unmounted",
        );
    }
}
