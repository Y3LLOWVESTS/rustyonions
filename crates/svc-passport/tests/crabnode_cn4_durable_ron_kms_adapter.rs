//! RO:WHAT — CN-4 integration tests for adapting restart-stable `ron-kms` service custody into `svc-passport::KmsClient`.
//! RO:WHY — CrabNode challenge signing must preserve the same service KID/public key and verification capability after process restart.
//! RO:INTERACTS — `DurableEd25519ServiceKey`, `DurableRonKmsClient`, and `KmsClient` exact-KID operations.
//! RO:INVARIANTS — wire KID is protocol-valid and restart-stable; wrong KID never signs or verifies; a pre-restart signature verifies after reopening the durable key.
//! RO:SECURITY — test temporary directories only; no secret material is printed or returned.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test crabnode_cn4_durable_ron_kms_adapter.

#![cfg(feature = "native-passport")]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use ron_kms::DurableEd25519ServiceKey;
use ron_proto::ServiceKeyIdV1;

use svc_passport::kms::{client::KmsClient, DurableRonKmsClient};

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
                "svc-passport-cn4-ron-kms-adapter-{label}-{}-{stamp}",
                std::process::id(),
            )),
        }
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn open_client(directory: &TestDirectory) -> DurableRonKmsClient {
    let key =
        DurableEd25519ServiceKey::open_or_create(directory.path(), "crabnode", "svc-passport")
            .expect("durable service key");

    DurableRonKmsClient::new(Arc::new(key)).expect("svc-passport durable KMS adapter")
}

#[tokio::test]
async fn durable_adapter_exposes_protocol_valid_restart_stable_identity() {
    let directory = TestDirectory::new("identity");

    let first = open_client(&directory);

    let first_identity = first
        .active_signing_identity()
        .await
        .expect("first identity");

    ServiceKeyIdV1::parse(first_identity.kid.clone()).expect("wire KID must be canonical");

    assert!(first_identity
        .kid
        .starts_with("ed25519/crabnode/svc-passport/"),);

    drop(first);

    let reopened = open_client(&directory);

    let reopened_identity = reopened
        .active_signing_identity()
        .await
        .expect("reopened identity");

    assert_eq!(
        reopened_identity, first_identity,
        "reopen must preserve exact wire KID and public key",
    );
}

#[tokio::test]
async fn pre_restart_signature_verifies_after_adapter_reopen() {
    let directory = TestDirectory::new("restart-signature");

    let first = open_client(&directory);

    let identity = first.active_signing_identity().await.expect("identity");

    let message = b"cn4-svc-passport-durable-kms-adapter-vector";

    let signature = first
        .sign_with_kid(&identity.kid, message)
        .await
        .expect("exact-KID signature");

    assert!(first
        .verify(&identity.kid, message, &signature,)
        .await
        .expect("first verify"),);

    drop(first);

    let reopened = open_client(&directory);

    assert!(
        reopened
            .verify(&identity.kid, message, &signature,)
            .await
            .expect("restart verify"),
        "pre-restart signature must remain verifiable after reopen",
    );
}

#[tokio::test]
async fn wrong_kid_and_rotation_fail_closed() {
    let directory = TestDirectory::new("fail-closed");

    let client = open_client(&directory);

    let message = b"cn4-wrong-kid";

    assert!(client
        .sign_with_kid(
            "ed25519/crabnode/svc-passport/not-the-active-key/v1",
            message,
        )
        .await
        .is_err(),);

    assert!(client
        .verify(
            "ed25519/crabnode/svc-passport/not-the-active-key/v1",
            message,
            &[0_u8; 64],
        )
        .await
        .is_err(),);

    assert!(
        client.rotate().await.is_err(),
        "single durable CN-4 key must not pretend rotation exists",
    );
}
