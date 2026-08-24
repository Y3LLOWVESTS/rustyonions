//! RO:WHAT — CN-4 HTTP acceptance tests for the fixed crash-recoverable RegisterRoot proof route.
//! RO:WHY — Prove that a real root-control proof consumes exactly one durable challenge and registers root authority without exposing unrelated proof authority or adjacent identity surfaces.
//! RO:INTERACTS — constrained svc-passport router, durable ron-kms adapter, `ron-auth` canonical challenge/root-proof transcripts, recovery-root test signer, challenge store, registry store, and redo coordinator.
//! RO:INVARIANTS — proof fields derive from the signed challenge and trusted service policy; forged proof does not consume the challenge; successful replay rejects; unrelated identity surfaces remain absent.
//! RO:METRICS — none.
//! RO:CONFIG — temporary test-local durable roots and private-beta trusted context.
//! RO:SECURITY — recovery factor exists only in temporary test memory; no secret is serialized, printed, returned, or persisted; no capability, username, wallet, or ledger mutation.
//! RO:TEST — `cargo test -p svc-passport --no-default-features --features native-passport --test crabnode_cn4_register_root_proof_route`.

#![cfg(feature = "native-passport")]

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
use ron_auth::native_passport::{
    passport_challenge_v1_transcript_b3_hex, RootRegistrationProofTranscriptV1,
};
use ron_kms::DurableEd25519ServiceKey;
use ron_proto::{
    B3DigestHex, Ed25519PublicKeyHex as ProtoEd25519PublicKeyHex, PassportChallengeV1,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use svc_passport::{
    http::router::build_native_profile_router_with_store_and_kms,
    kms::{client::KmsClient, DurableRonKmsClient},
    native::{
        derive_native_recovery_public_identity_v1, sign_native_recovery_root_registration_proof_v1,
        NativePassportServerRuntimeMountConfigV1, NativeSecretBytes,
        PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
        PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
    },
    profile::UsernameClaimStore,
};

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
                "svc-passport-cn4-register-root-proof-{label}-{}-{stamp}",
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

    let adapter = DurableRonKmsClient::new(Arc::new(key)).expect("durable KMS adapter");

    Arc::new(adapter)
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

fn request(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).expect("JSON request")))
        .expect("request")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 128 * 1024)
        .await
        .expect("response bytes");

    serde_json::from_slice(&bytes).expect("JSON response")
}

async fn issue_challenge(app: &Router, passport_id: &str) -> PassportChallengeV1 {
    let response = app
        .clone()
        .oneshot(request(
            "/v1/passport/register/challenge",
            json!({
                "passport_id": passport_id,
                "requested_scopes": [
                    "identity.read",
                    "profile.read"
                ],
                "operation_body_hash": HEX_D
            }),
        ))
        .await
        .expect("challenge response");

    assert_eq!(response.status(), StatusCode::OK,);

    let bytes = to_bytes(response.into_body(), 128 * 1024)
        .await
        .expect("challenge bytes");

    serde_json::from_slice(&bytes).expect("challenge JSON")
}

fn proof_body(
    recovery_factor: &NativeSecretBytes,
    root_public_key: &str,
    challenge: &PassportChallengeV1,
) -> Value {
    let challenge_hash = passport_challenge_v1_transcript_b3_hex(&challenge.signing_payload())
        .expect("challenge transcript hash");

    let challenge_transcript_hash = B3DigestHex::parse("challenge_transcript_hash", challenge_hash)
        .expect("typed challenge hash");

    let root_public_key =
        ProtoEd25519PublicKeyHex::parse(root_public_key).expect("typed root public key");

    let passport_id = challenge
        .passport_id
        .as_ref()
        .expect("RegisterRoot Passport binding");

    let operation_body_hash = challenge
        .operation_body_hash
        .as_ref()
        .expect("RegisterRoot operation binding");

    let scopes: Vec<&str> = challenge
        .requested_scopes
        .iter()
        .map(|scope| scope.as_str())
        .collect();

    /*
     * The challenge issue instant is valid inside the challenge window and
     * avoids test dependence on scheduler delay. Production clients use their
     * actual native signing time.
     */
    let proof_created_at_ms = challenge.issued_at_ms;

    let transcript = RootRegistrationProofTranscriptV1 {
        challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,

        challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,

        proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,

        proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,

        challenge_id: &challenge.challenge_id,

        network_id: challenge.network_id.as_str(),

        environment: challenge.environment.as_str(),

        audience: challenge.audience.as_str(),

        passport_id,

        root_public_key: &root_public_key,

        root_key_epoch: 0,

        device_id: None,

        operation_body_hash,

        challenge_transcript_hash: &challenge_transcript_hash,

        requested_scopes: &scopes,

        challenge_issued_at_ms: challenge.issued_at_ms,

        challenge_expires_at_ms: challenge.expires_at_ms,

        proof_created_at_ms,
    };

    let signed = sign_native_recovery_root_registration_proof_v1(recovery_factor, &transcript)
        .expect("root registration proof");

    json!({
        "challenge": challenge,
        "root_public_key":
            root_public_key.as_str(),
        "proof_created_at_ms":
            proof_created_at_ms,
        "proof_signed_payload_hex":
            signed.signed_payload_hex.as_str()
    })
}

#[tokio::test]
async fn valid_proof_registers_once_and_same_challenge_replay_rejects() {
    let directory = TestDirectory::new("green");

    let app = app(&directory).await;

    let recovery_factor = NativeSecretBytes::new(vec![0x71; 32]).expect("test recovery factor");

    let identity =
        derive_native_recovery_public_identity_v1(&recovery_factor).expect("test root identity");

    let challenge = issue_challenge(&app, identity.passport_id.as_str()).await;

    let body = proof_body(
        &recovery_factor,
        identity.root_public_key.as_str(),
        &challenge,
    );

    let response = app
        .clone()
        .oneshot(request("/v1/passport/register/proof", body.clone()))
        .await
        .expect("proof response");

    assert_eq!(response.status(), StatusCode::OK,);

    let result = response_json(response).await;

    assert_eq!(
        result["schema"],
        "svc-passport.native-register-root-proof-result.v1",
    );

    assert_eq!(result["status"], "registered",);

    assert_eq!(result["durable_generation"], 1,);

    assert!(
        directory.root.join("registry").exists(),
        "successful proof must create durable registry state",
    );

    let replay = app
        .clone()
        .oneshot(request("/v1/passport/register/proof", body))
        .await
        .expect("replay response");

    assert_eq!(
        replay.status(),
        StatusCode::CONFLICT,
        "same one-time challenge must never register twice",
    );
}

#[tokio::test]
async fn forged_proof_does_not_consume_challenge_before_valid_retry() {
    let directory = TestDirectory::new("forged");

    let app = app(&directory).await;

    let recovery_factor = NativeSecretBytes::new(vec![0x72; 32]).expect("test recovery factor");

    let identity =
        derive_native_recovery_public_identity_v1(&recovery_factor).expect("test root identity");

    let challenge = issue_challenge(&app, identity.passport_id.as_str()).await;

    let valid = proof_body(
        &recovery_factor,
        identity.root_public_key.as_str(),
        &challenge,
    );

    let mut forged = valid.clone();

    forged["proof_signed_payload_hex"] = json!("00".repeat(64));

    let rejected = app
        .clone()
        .oneshot(request("/v1/passport/register/proof", forged))
        .await
        .expect("forged response");

    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST,);

    /*
     * Critical transaction ordering proof: a bad root signature fails before
     * redo prepare and before one-time challenge consumption, so the genuine
     * proof can still use the same challenge.
     */
    let accepted = app
        .clone()
        .oneshot(request("/v1/passport/register/proof", valid))
        .await
        .expect("valid retry response");

    assert_eq!(accepted.status(), StatusCode::OK,);

    let result = response_json(accepted).await;

    assert_eq!(result["status"], "registered",);
}

#[tokio::test]
async fn trusted_fields_cannot_be_injected_into_register_root_proof() {
    let directory = TestDirectory::new("surface");

    let app = app(&directory).await;

    let recovery_factor = NativeSecretBytes::new(vec![0x73; 32]).expect("test recovery factor");

    let identity =
        derive_native_recovery_public_identity_v1(&recovery_factor).expect("test root identity");

    let challenge = issue_challenge(&app, identity.passport_id.as_str()).await;

    let mut body = proof_body(
        &recovery_factor,
        identity.root_public_key.as_str(),
        &challenge,
    );

    let object = body.as_object_mut().expect("proof object");

    object.insert("accepted_at_ms".to_owned(), json!(challenge.issued_at_ms));

    object.insert("root_key_epoch".to_owned(), json!(77));

    object.insert(
        "challenge_transcript_hash".to_owned(),
        json!("aa".repeat(32)),
    );

    let injected = app
        .clone()
        .oneshot(request("/v1/passport/register/proof", body))
        .await
        .expect("injected response");

    assert_eq!(
        injected.status(),
        StatusCode::BAD_REQUEST,
        "strict DTO must reject caller attempts to inject trusted bindings",
    );
}
