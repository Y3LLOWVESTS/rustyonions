//! RO:WHAT — CN-4 contract tests for recovery-gated Native Passport composition behind CrabNode's constrained svc-passport router.
//! RO:WHY — A service must not become ready until service KMS and durable RegisterRoot recovery are usable, while CN-3's no-admin/no-full-router boundary remains intact.
//! RO:INTERACTS — build_native_profile_router_with_store_and_kms, DurableRonKmsClient test fixture, UsernameClaimStore, and Native Passport durable startup preflight.
//! RO:INVARIANTS — valid runtime preflight returns the existing constrained profile surface; malformed trusted context fails closed; unreviewed full KMS/admin surfaces remain absent.
//! RO:SECURITY — durable ron-kms custody is test-local and explicitly injected; no DevKms, key export, KMS admin, capability issue, username authority change, wallet, or ledger mutation.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test crabnode_cn4_native_runtime_mount.

#![cfg(feature = "native-passport")]

use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};

use ron_kms::DurableEd25519ServiceKey;

use svc_passport::{
    http::router::build_native_profile_router_with_store_and_kms,
    kms::{client::KmsClient, DurableRonKmsClient},
    native::NativePassportServerRuntimeMountConfigV1,
    profile::UsernameClaimStore,
};

use tower::ServiceExt;

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
                "svc-passport-cn4-runtime-mount-{label}-{}-{stamp}",
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
    .expect("durable CN-4 test service key");

    let adapter =
        DurableRonKmsClient::new(Arc::new(key)).expect("durable svc-passport KMS adapter");

    Arc::new(adapter)
}

fn request(method: Method, path: &str, body: &'static str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .expect("request")
}

#[tokio::test]
async fn native_runtime_preflight_preserves_constrained_crabnode_surface() {
    let directory = TestDirectory::new("green");

    let profile_store = Arc::new(UsernameClaimStore::new());

    let app = build_native_profile_router_with_store_and_kms(
        profile_store,
        durable_kms(&directory),
        directory.config(),
    )
    .await
    .expect("valid KMS and durable runtime must mount");

    let health = app
        .clone()
        .oneshot(request(Method::GET, "/healthz", ""))
        .await
        .expect("health");

    assert_eq!(health.status(), StatusCode::OK);

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
            "{path} must remain absent before the reviewed CN-4 route slice",
        );
    }
}

#[tokio::test]
async fn malformed_trusted_context_fails_before_router_becomes_available() {
    let directory = TestDirectory::new("bad-context");

    let mut config = directory.config();
    config.network_id.clear();

    let result = build_native_profile_router_with_store_and_kms(
        Arc::new(UsernameClaimStore::new()),
        durable_kms(&directory),
        config,
    )
    .await;

    assert!(
        result.is_err(),
        "invalid trusted context must fail closed before router availability",
    );
}
