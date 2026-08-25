//! RO:WHAT — CN-4 HTTP composition acceptance for the DeviceKey-protected public username claim builder.
//! RO:WHY — The public CrabNode profile mutation must not inherit the historical caller-supplied Passport subject route while capability and Native Passport routes remain available.
//! RO:INTERACTS — protected svc-passport router builder, durable ron-kms identity, capability recovery, request-replay preflight, strict username intent DTO, and fixed Native Passport routes.
//! RO:INVARIANTS — missing request proof rejects; caller Passport ownership rejects; request replay state opens before readiness; capability and RegisterRoot surfaces remain mounted.
//! RO:METRICS — none.
//! RO:CONFIG — isolated temporary Native Passport/capability/request-replay roots.
//! RO:SECURITY — no physical Passport/DeviceKey secret, fake claim, wallet/ledger mutation, or caller-owned Passport authority.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test crabnode_cn4_protected_username_route.

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
    http::router::build_native_profile_router_with_store_kms_capability_and_request_proof,
    kms::{client::KmsClient, DurableRonKmsClient},
    native::{
        NativePassportServerCapabilityRuntimeConfigV1,
        NativePassportServerRequestProofRuntimeConfigV1, NativePassportServerRuntimeMountConfigV1,
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
                "svc-passport-cn4-protected-username-{label}-{}-{stamp}",
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

    fn request_config(&self) -> NativePassportServerRequestProofRuntimeConfigV1 {
        NativePassportServerRequestProofRuntimeConfigV1 {
            request_replay_root: self.root.join("request-proof-replay"),
            request_replay_retention_ms: 120_000,
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

async fn protected_app(directory: &TestDirectory) -> Router {
    build_native_profile_router_with_store_kms_capability_and_request_proof(
        Arc::new(UsernameClaimStore::new()),
        durable_kms(directory),
        directory.runtime_config(),
        directory.capability_config(),
        directory.request_config(),
    )
    .await
    .expect("protected CrabNode identity router")
}

#[tokio::test]
async fn protected_builder_removes_caller_owned_profile_claim_authority() {
    let directory = TestDirectory::new("caller-authority");

    let app = protected_app(&directory).await;

    let missing_proof = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/v1/passport/profile/claim",
            r#"{"requested_username":"testmac"}"#,
        ))
        .await
        .expect("missing-proof response");

    assert_eq!(
        missing_proof.status(),
        StatusCode::UNAUTHORIZED,
        "public username mutation must require request proof",
    );

    let forged_subject = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/v1/passport/profile/claim",
            format!(r#"{{"passport_subject":"{PASSPORT_ID}","requested_username":"testmac"}}"#,),
        ))
        .await
        .expect("forged-subject response");

    assert_eq!(
        forged_subject.status(),
        StatusCode::BAD_REQUEST,
        "caller-supplied Passport ownership must fail closed",
    );
}

#[tokio::test]
async fn protected_builder_recovers_request_replay_and_preserves_fixed_routes() {
    let directory = TestDirectory::new("recovery-and-routes");

    let app = protected_app(&directory).await;

    assert!(
        directory.root.join("request-proof-replay").is_dir(),
        "request-proof replay state must open during builder preflight",
    );

    assert!(
        directory.root.join("capabilities").is_dir(),
        "capability state must remain recovery-gated",
    );

    assert!(
        directory.root.join("capability-issuance-redo").is_dir(),
        "capability redo state must remain recovery-gated",
    );

    let trust_anchor = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/v1/passport/register/trust-anchor",
            "",
        ))
        .await
        .expect("trust-anchor response");

    assert_eq!(
        trust_anchor.status(),
        StatusCode::OK,
        "RegisterRoot trust anchor must remain mounted",
    );

    let capability_challenge = app
        .clone()
        .oneshot(request(
            Method::POST,
            "/v1/passport/capability/challenge",
            format!(
                concat!(
                    r#"{{"passport_id":"{}","#,
                    r#""device_id":"{}","#,
                    r#""requested_scopes":["identity.read","identity.username.claim"]}}"#
                ),
                PASSPORT_ID, DEVICE_ID,
            ),
        ))
        .await
        .expect("capability challenge response");

    assert_eq!(
        capability_challenge.status(),
        StatusCode::NOT_FOUND,
        "fixed capability route must reach real durable device lookup",
    );
}
