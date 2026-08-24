//! RO:WHAT — Locks the exact CN-4 internal svc-passport device-authorization HTTP admission surface.
//! RO:WHY — DeviceAuthorizationV1 must enter durable server truth through one bounded reviewed route before Omnigate/gateway exposure or possession/capability work.
//! RO:INTERACTS — constrained native router, durable KMS fixture, device authorization handler, and Native Passport runtime mount.
//! RO:INVARIANTS — exact route is mounted; malformed/unknown/oversized bodies fail closed; nearby generic device/register and unreviewed adjacent identity surfaces remain absent.
//! RO:METRICS — none.
//! RO:CONFIG — test-only durable Native Passport directory and fixed private-beta context.
//! RO:SECURITY — no physical Passport, root/device secret, real authorization mutation, capability, username, wallet, or ledger authority.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test crabnode_cn4_device_authorize_route.

#![cfg(feature = "native-passport")]
#![forbid(unsafe_code)]

use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
    Router,
};

use ron_kms::DurableEd25519ServiceKey;
use serde_json::Value;
use tower::ServiceExt;

use svc_passport::{
    http::{
        handlers::native_device_authorize::NATIVE_DEVICE_AUTHORIZATION_PROBLEM_SCHEMA_V1,
        router::build_native_profile_router_with_store_and_kms,
    },
    kms::{client::KmsClient, DurableRonKmsClient},
    native::NativePassportServerRuntimeMountConfigV1,
    profile::UsernameClaimStore,
};

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
                "svc-passport-cn4-device-authorize-route-{label}-{}-{stamp}",
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

    Arc::new(DurableRonKmsClient::new(Arc::new(key)).expect("durable KMS adapter"))
}

async fn app(directory: &TestDirectory) -> Router {
    build_native_profile_router_with_store_and_kms(
        Arc::new(UsernameClaimStore::new()),
        durable_kms(directory),
        directory.config(),
    )
    .await
    .expect("CN-4 constrained router")
}

fn request(method: Method, uri: &str, body: impl Into<Body>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.into())
        .expect("request")
}

#[tokio::test]
async fn exact_device_authorize_route_is_bounded_and_fail_closed() {
    let directory = TestDirectory::new("surface");

    let app = app(&directory).await;

    let malformed = app
        .clone()
        .oneshot(request(Method::POST, "/v1/passport/device/authorize", "{}"))
        .await
        .expect("malformed response");

    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST,);

    let bytes = to_bytes(malformed.into_body(), 16 * 1024)
        .await
        .expect("problem body");

    let value: Value = serde_json::from_slice(&bytes).expect("problem JSON");

    assert_eq!(
        value.get("schema").and_then(Value::as_str),
        Some(NATIVE_DEVICE_AUTHORIZATION_PROBLEM_SCHEMA_V1),
    );

    assert_eq!(
        value.get("code").and_then(Value::as_str),
        Some("invalid_request"),
    );

    assert_eq!(value.get("retryable").and_then(Value::as_bool), Some(false),);

    let unknown = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/v1/passport/device/authorize",
            r#"{"authorization":{},"unexpected":true}"#,
        ))
        .await
        .expect("unknown-field response");

    assert_eq!(unknown.status(), StatusCode::BAD_REQUEST,);

    let oversized_body = format!(
        r#"{{"authorization":{{}},"padding":"{}"}}"#,
        "x".repeat(16_385),
    );

    let oversized = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/v1/passport/device/authorize",
            oversized_body,
        ))
        .await
        .expect("oversized response");

    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE,);

    for path in ["/v1/passport/device/register"] {
        let response = app
            .clone()
            .oneshot(request(Method::POST, path, "{}"))
            .await
            .expect("closed route response");

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} must remain unmounted",
        );
    }
}
