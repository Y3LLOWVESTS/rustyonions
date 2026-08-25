//! RO:WHAT — Private CN-4 server admission runtime for DeviceKey-signed username-claim request proofs.
//! RO:WHY — A durable device capability must not authorize namespace mutation by itself; the exact protected request must also prove current capability authority, current root-signed device authority, DeviceKey possession, freshness, and one-time replay state.
//! RO:INTERACTS — ron-auth PassportRequestProofV1 verifier, ron-policy private-beta device policy, durable capability state, durable device registry state, and durable request-nonce replay storage.
//! RO:INVARIANTS — POST /identity/passport/profile/claim and identity.username.claim are fixed server purpose; Passport/Device authority comes only from durable capability and registry state; root epoch/current policy are revalidated; bad proofs never consume replay state; accepted capability+nonce pairs are consumed durably once.
//! RO:METRICS — none in this private slice; a later HTTP adapter may expose bounded outcome counters without identity-bearing labels.
//! RO:CONFIG — caller supplies a dedicated request-replay root with retention covering the full two-sided V1 freshness window; all authority/replay roots must remain distinct.
//! RO:SECURITY — verifies public authority metadata and DeviceKey signatures only; no DeviceKey/root secret, PIN, RecoveryRoot, capability issuance, username/profile mutation, wallet, ledger, HTTP, or Tauri authority.
//! RO:TEST — focused unit tests below cover exact admission, restart-safe replay rejection, bad-proof no-consume, capability expiry/root-epoch rejection, config isolation, and forbidden authority.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use ron_auth::native_passport::{
    verify_passport_request_proof_v1_strict, PassportRequestProofVerificationContextV1,
    PASSPORT_REQUEST_PROOF_V1_MAX_CLOCK_SKEW_MS,
};
use ron_policy::NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION;
use ron_proto::{
    B3DigestHex, DeviceIdV1, NativePassportScopeV1, PassportIdV1, PassportRequestProofV1,
};
use thiserror::Error;

use super::{
    server_capability_runtime::NativePassportServerCapabilityRuntimeConfigV1,
    server_capability_store::{
        NativePassportServerCapabilityRecordV1, NativePassportServerCapabilitySnapshotStore,
        NativePassportServerCapabilityStatusV1, NativePassportServerCapabilityStoreError,
    },
    server_device_session_runtime::{
        load_live_device_authority, parse_trusted_context, LiveDeviceAuthorityV1,
        TrustedDeviceSessionContextV1,
    },
    server_request_replay_store::{
        NativePassportRequestReplayStore, NativePassportRequestReplayStoreError,
    },
    NativePassportServerRuntimeMountConfigV1,
};

const USERNAME_CLAIM_REQUEST_METHOD: &str = "POST";
const USERNAME_CLAIM_CANONICAL_PATH: &str = "/identity/passport/profile/claim";
const USERNAME_CLAIM_SCOPE: &str = "identity.username.claim";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportServerRequestProofRuntimeConfigV1 {
    pub request_replay_root: PathBuf,
    pub request_replay_retention_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativePassportUsernameClaimAdmissionV1 {
    pub(crate) passport_id: PassportIdV1,
    pub(crate) device_id: DeviceIdV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum NativePassportServerRequestProofRuntimeError {
    #[error("Native Passport request-proof runtime configuration is invalid")]
    InvalidConfig,

    #[error("Native Passport protected request is invalid")]
    InvalidRequest,

    #[error("Native Passport protected-request capability was not found")]
    CapabilityNotFound,

    #[error("Native Passport protected-request capability was rejected")]
    CapabilityRejected,

    #[error("Native Passport current device authority rejected the protected request")]
    DeviceAuthorityRejected,

    #[error("Native Passport protected-request DeviceKey proof was rejected")]
    ProofRejected,

    #[error("Native Passport protected-request authority changed during verification")]
    AuthorityChanged,

    #[error("Native Passport protected-request replay was rejected")]
    ReplayRejected,

    #[error("Native Passport protected-request replay state is full")]
    StateFull,

    #[error("Native Passport protected-request durable state is unavailable")]
    StorageUnavailable,

    #[error("Native Passport protected-request durable state is corrupt")]
    StorageCorrupt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveCapabilityAuthorityV1 {
    generation: u64,
    record: NativePassportServerCapabilityRecordV1,
}

pub(crate) fn preflight_native_passport_request_proof_runtime(
    base_config: &NativePassportServerRuntimeMountConfigV1,
    capability_config: &NativePassportServerCapabilityRuntimeConfigV1,
    request_config: &NativePassportServerRequestProofRuntimeConfigV1,
    now_ms: u64,
) -> Result<(), NativePassportServerRequestProofRuntimeError> {
    validate_runtime_config(base_config, capability_config, request_config)?;

    if now_ms == 0 {
        return Err(NativePassportServerRequestProofRuntimeError::InvalidRequest);
    }

    NativePassportRequestReplayStore::open(
        &request_config.request_replay_root,
        request_config.request_replay_retention_ms,
        now_ms,
    )
    .map_err(map_replay_store_error)?;

    Ok(())
}

pub(crate) fn admit_username_claim_request_proof_v1(
    base_config: &NativePassportServerRuntimeMountConfigV1,
    capability_config: &NativePassportServerCapabilityRuntimeConfigV1,
    request_config: &NativePassportServerRequestProofRuntimeConfigV1,
    proof: &PassportRequestProofV1,
    expected_canonical_query_hash: &B3DigestHex,
    expected_body_hash: &B3DigestHex,
    accepted_at_ms: u64,
) -> Result<NativePassportUsernameClaimAdmissionV1, NativePassportServerRequestProofRuntimeError> {
    validate_runtime_config(base_config, capability_config, request_config)?;

    if accepted_at_ms == 0 {
        return Err(NativePassportServerRequestProofRuntimeError::InvalidRequest);
    }

    proof
        .validate()
        .map_err(|_| NativePassportServerRequestProofRuntimeError::InvalidRequest)?;

    let trusted = parse_trusted_context(base_config)
        .map_err(|_| NativePassportServerRequestProofRuntimeError::InvalidConfig)?;

    let required_scope = NativePassportScopeV1::parse(USERNAME_CLAIM_SCOPE)
        .map_err(|_| NativePassportServerRequestProofRuntimeError::InvalidConfig)?;

    let capability_before = load_capability_authority(
        capability_config,
        &trusted,
        proof,
        accepted_at_ms,
        &required_scope,
    )?;

    let capability = &capability_before.record.capability;
    let required_scopes = [required_scope.clone()];

    let device_before = load_live_device_authority(
        base_config,
        &trusted,
        &capability.passport_id,
        &capability.device_id,
        &required_scopes,
        accepted_at_ms,
    )
    .map_err(|_| NativePassportServerRequestProofRuntimeError::DeviceAuthorityRejected)?;

    validate_capability_device_binding(&capability_before, &device_before)?;

    verify_passport_request_proof_v1_strict(
        proof,
        PassportRequestProofVerificationContextV1 {
            trusted_device_public_key: &device_before.authorization.device_public_key,
            expected_capability_id: &capability.capability_id,
            expected_device_id: &capability.device_id,
            expected_request_method: USERNAME_CLAIM_REQUEST_METHOD,
            expected_canonical_path: USERNAME_CLAIM_CANONICAL_PATH,
            expected_canonical_query_hash,
            expected_body_hash,
            now_ms: accepted_at_ms,
            max_clock_skew_ms: PASSPORT_REQUEST_PROOF_V1_MAX_CLOCK_SKEW_MS,
        },
    )
    .map_err(|_| NativePassportServerRequestProofRuntimeError::ProofRejected)?;

    let capability_before_consume = load_capability_authority(
        capability_config,
        &trusted,
        proof,
        accepted_at_ms,
        &required_scope,
    )?;

    let device_before_consume = load_live_device_authority(
        base_config,
        &trusted,
        &capability.passport_id,
        &capability.device_id,
        &required_scopes,
        accepted_at_ms,
    )
    .map_err(|_| NativePassportServerRequestProofRuntimeError::DeviceAuthorityRejected)?;

    validate_capability_device_binding(&capability_before_consume, &device_before_consume)?;

    if capability_before_consume != capability_before || device_before_consume != device_before {
        return Err(NativePassportServerRequestProofRuntimeError::AuthorityChanged);
    }

    let replay_store = NativePassportRequestReplayStore::open(
        &request_config.request_replay_root,
        request_config.request_replay_retention_ms,
        accepted_at_ms,
    )
    .map_err(map_replay_store_error)?;

    replay_store
        .consume(
            &capability.capability_id,
            &proof.request_nonce,
            accepted_at_ms,
        )
        .map_err(map_replay_store_error)?;

    Ok(NativePassportUsernameClaimAdmissionV1 {
        passport_id: capability.passport_id.clone(),
        device_id: capability.device_id.clone(),
    })
}

fn load_capability_authority(
    capability_config: &NativePassportServerCapabilityRuntimeConfigV1,
    trusted: &TrustedDeviceSessionContextV1,
    proof: &PassportRequestProofV1,
    accepted_at_ms: u64,
    required_scope: &NativePassportScopeV1,
) -> Result<LiveCapabilityAuthorityV1, NativePassportServerRequestProofRuntimeError> {
    let (_, loaded) = NativePassportServerCapabilitySnapshotStore::open(
        &capability_config.capability_root,
        trusted.audience.clone(),
        trusted.environment.clone(),
    )
    .map_err(map_capability_store_error)?;

    let record = loaded
        .snapshot
        .capabilities
        .iter()
        .find(|record| record.capability.capability_id == proof.capability_id)
        .cloned()
        .ok_or(NativePassportServerRequestProofRuntimeError::CapabilityNotFound)?;

    validate_capability_authority(&record, trusted, accepted_at_ms, required_scope)?;

    Ok(LiveCapabilityAuthorityV1 {
        generation: loaded.generation,
        record,
    })
}

fn validate_capability_authority(
    record: &NativePassportServerCapabilityRecordV1,
    trusted: &TrustedDeviceSessionContextV1,
    accepted_at_ms: u64,
    required_scope: &NativePassportScopeV1,
) -> Result<(), NativePassportServerRequestProofRuntimeError> {
    let capability = &record.capability;

    capability
        .validate()
        .map_err(|_| NativePassportServerRequestProofRuntimeError::CapabilityRejected)?;

    if record.status != NativePassportServerCapabilityStatusV1::Active
        || record.revoked_at_ms.is_some()
    {
        return Err(NativePassportServerRequestProofRuntimeError::CapabilityRejected);
    }

    if capability.audience != trusted.audience || capability.environment != trusted.environment {
        return Err(NativePassportServerRequestProofRuntimeError::CapabilityRejected);
    }

    if capability.issued_at_ms > accepted_at_ms || accepted_at_ms > capability.expires_at_ms {
        return Err(NativePassportServerRequestProofRuntimeError::CapabilityRejected);
    }

    if capability.policy_version != NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION {
        return Err(NativePassportServerRequestProofRuntimeError::CapabilityRejected);
    }

    if !capability
        .scopes
        .iter()
        .any(|scope| scope == required_scope)
    {
        return Err(NativePassportServerRequestProofRuntimeError::CapabilityRejected);
    }

    Ok(())
}

fn validate_capability_device_binding(
    capability_authority: &LiveCapabilityAuthorityV1,
    device_authority: &LiveDeviceAuthorityV1,
) -> Result<(), NativePassportServerRequestProofRuntimeError> {
    let capability = &capability_authority.record.capability;
    let authorization = &device_authority.authorization;

    if capability.passport_id != authorization.passport_id
        || capability.device_id != authorization.device_id
        || capability.root_key_epoch != Some(authorization.root_key_epoch)
    {
        return Err(NativePassportServerRequestProofRuntimeError::CapabilityRejected);
    }

    Ok(())
}

fn validate_runtime_config(
    base_config: &NativePassportServerRuntimeMountConfigV1,
    capability_config: &NativePassportServerCapabilityRuntimeConfigV1,
    request_config: &NativePassportServerRequestProofRuntimeConfigV1,
) -> Result<(), NativePassportServerRequestProofRuntimeError> {
    let request_root = &request_config.request_replay_root;
    let capability_root = &capability_config.capability_root;
    let capability_txn_root = &capability_config.transaction_root;

    if request_root.as_os_str().is_empty()
        || capability_root.as_os_str().is_empty()
        || capability_txn_root.as_os_str().is_empty()
    {
        return Err(NativePassportServerRequestProofRuntimeError::InvalidConfig);
    }

    let minimum_request_replay_retention_ms = PASSPORT_REQUEST_PROOF_V1_MAX_CLOCK_SKEW_MS
        .checked_mul(2)
        .ok_or(NativePassportServerRequestProofRuntimeError::InvalidConfig)?;

    if request_config.request_replay_retention_ms < minimum_request_replay_retention_ms {
        return Err(NativePassportServerRequestProofRuntimeError::InvalidConfig);
    }

    if capability_root == capability_txn_root
        || request_root == capability_root
        || request_root == capability_txn_root
        || request_root == &base_config.challenge_root
        || request_root == &base_config.registry_root
        || request_root == &base_config.transaction_root
        || capability_root == &base_config.challenge_root
        || capability_root == &base_config.registry_root
        || capability_root == &base_config.transaction_root
        || capability_txn_root == &base_config.challenge_root
        || capability_txn_root == &base_config.registry_root
        || capability_txn_root == &base_config.transaction_root
    {
        return Err(NativePassportServerRequestProofRuntimeError::InvalidConfig);
    }

    Ok(())
}

fn map_capability_store_error(
    error: NativePassportServerCapabilityStoreError,
) -> NativePassportServerRequestProofRuntimeError {
    match error {
        NativePassportServerCapabilityStoreError::Unavailable(_) => {
            NativePassportServerRequestProofRuntimeError::StorageUnavailable
        }
        NativePassportServerCapabilityStoreError::Corrupt(_) => {
            NativePassportServerRequestProofRuntimeError::StorageCorrupt
        }
    }
}

fn map_replay_store_error(
    error: NativePassportRequestReplayStoreError,
) -> NativePassportServerRequestProofRuntimeError {
    match error {
        NativePassportRequestReplayStoreError::InvalidConfig => {
            NativePassportServerRequestProofRuntimeError::InvalidConfig
        }
        NativePassportRequestReplayStoreError::InvalidTrustedTime => {
            NativePassportServerRequestProofRuntimeError::InvalidRequest
        }
        NativePassportRequestReplayStoreError::AlreadyConsumed => {
            NativePassportServerRequestProofRuntimeError::ReplayRejected
        }
        NativePassportRequestReplayStoreError::StateFull => {
            NativePassportServerRequestProofRuntimeError::StateFull
        }
        NativePassportRequestReplayStoreError::Unavailable(_) => {
            NativePassportServerRequestProofRuntimeError::StorageUnavailable
        }
        NativePassportRequestReplayStoreError::Corrupt(_) => {
            NativePassportServerRequestProofRuntimeError::StorageCorrupt
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use ed25519_dalek::{Signer as _, SigningKey};
    use ron_auth::native_passport::canonical_passport_request_proof_v1_transcript;
    use ron_policy::private_beta_device_authorization_scope_ceiling_v1;
    use ron_proto::{
        CapabilityIdV1, ChallengeIdV1, DeviceAuthorizationNonceV1,
        DeviceAuthorizationSigningPayloadV1, DeviceAuthorizationV1, DeviceClassV1,
        Ed25519PublicKeyHex, Ed25519SignatureV1, NativePassportContextLabelV1,
        NativePassportDeviceBoundCapabilityV1, PassportRequestProofV1,
        DEVICE_AUTHORIZATION_V1_VERSION, NATIVE_PASSPORT_DEVICE_BOUND_CAPABILITY_V1_VERSION,
        PASSPORT_REQUEST_PROOF_V1_VERSION,
    };

    use crate::native::{
        derive_native_device_public_identity_v1, derive_native_recovery_public_identity_v1,
        sign_native_recovery_device_authorization_v1, NativeSecretBytes,
    };

    use super::super::{
        server_capability_store::{
            NativePassportServerCapabilityRecordV1, NativePassportServerCapabilitySnapshotStore,
            NativePassportServerCapabilitySnapshotV1, NativePassportServerCapabilityStatusV1,
        },
        server_registry_store::{
            NativePassportServerDeviceRecordV1, NativePassportServerDeviceStatusV1,
            NativePassportServerRegistrySnapshotStore, NativePassportServerRegistrySnapshotV1,
            NativePassportServerRootRecordV1,
        },
    };

    use super::*;

    const NOW_MS: u64 = 1_787_600_100_000;

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HEX_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

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
                    "svc-passport-request-proof-runtime-{label}-{}-{stamp}",
                    std::process::id(),
                )),
            }
        }

        fn child(&self, name: &str) -> PathBuf {
            self.root.join(name)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    struct Fixture {
        device_seed_bytes: [u8; 32],
        passport_id: PassportIdV1,
        root_public_key: Ed25519PublicKeyHex,
        device_id: DeviceIdV1,
        authorization: DeviceAuthorizationV1,
    }

    impl Fixture {
        fn new() -> Self {
            let recovery_factor =
                NativeSecretBytes::new(vec![0x22; 32]).expect("test recovery factor");

            let root_identity = derive_native_recovery_public_identity_v1(&recovery_factor)
                .expect("root public identity");

            let device_seed_bytes = [0x42; 32];

            let device_secret =
                NativeSecretBytes::new(device_seed_bytes.to_vec()).expect("test DeviceKey seed");

            let device_identity =
                derive_native_device_public_identity_v1(&device_secret).expect("device identity");

            let passport_id =
                PassportIdV1::parse(root_identity.passport_id.as_str()).expect("Passport ID");

            let root_public_key =
                Ed25519PublicKeyHex::parse(root_identity.root_public_key.as_str())
                    .expect("root public key");

            let device_id =
                DeviceIdV1::parse(device_identity.device_id.as_str()).expect("Device ID");

            let device_public_key =
                Ed25519PublicKeyHex::parse(device_identity.device_public_key.as_str())
                    .expect("device public key");

            let ceiling =
                private_beta_device_authorization_scope_ceiling_v1(DeviceClassV1::RootAdminDesktop)
                    .expect("root-admin policy ceiling");

            let payload = DeviceAuthorizationSigningPayloadV1 {
                version: DEVICE_AUTHORIZATION_V1_VERSION,
                network_id: context("rustyonions-devnet"),
                environment: context("private-beta"),
                passport_id: passport_id.clone(),
                root_key_epoch: 0,
                device_id: device_id.clone(),
                device_public_key,
                device_class: DeviceClassV1::RootAdminDesktop,
                authorized_scope_ceiling: ceiling,
                authorization_nonce: DeviceAuthorizationNonceV1::from_bytes([0x11; 16]),
                issued_at_ms: NOW_MS - 10_000,
                expires_at_ms: Some(NOW_MS + 3_600_000),
            };

            let authorization =
                sign_native_recovery_device_authorization_v1(&recovery_factor, payload)
                    .expect("signed DeviceAuthorization");

            Self {
                device_seed_bytes,
                passport_id,
                root_public_key,
                device_id,
                authorization,
            }
        }
    }

    fn context(value: &str) -> NativePassportContextLabelV1 {
        NativePassportContextLabelV1::parse(value).expect("context")
    }

    fn scope(value: &str) -> NativePassportScopeV1 {
        NativePassportScopeV1::parse(value).expect("scope")
    }

    fn digest(field: &'static str, value: &str) -> B3DigestHex {
        B3DigestHex::parse(field, value).expect("B3 digest")
    }

    fn base_config(directory: &TestDirectory) -> NativePassportServerRuntimeMountConfigV1 {
        NativePassportServerRuntimeMountConfigV1 {
            challenge_root: directory.child("challenges"),
            registry_root: directory.child("registry"),
            transaction_root: directory.child("root-registration-redo"),
            network_id: "rustyonions-devnet".to_owned(),
            environment: "private-beta".to_owned(),
            audience: "svc-passport".to_owned(),
            issuing_service_id: "svc-passport".to_owned(),
            challenge_ttl_ms: 60_000,
            replay_retention_ms: 120_000,
            trusted_initial_root_key_epoch: 0,
        }
    }

    fn capability_config(
        directory: &TestDirectory,
    ) -> NativePassportServerCapabilityRuntimeConfigV1 {
        NativePassportServerCapabilityRuntimeConfigV1 {
            capability_root: directory.child("capabilities"),
            transaction_root: directory.child("capability-issuance-redo"),
            capability_ttl_ms: 3_600_000,
        }
    }

    fn request_config(
        directory: &TestDirectory,
    ) -> NativePassportServerRequestProofRuntimeConfigV1 {
        NativePassportServerRequestProofRuntimeConfigV1 {
            request_replay_root: directory.child("request-proof-replay"),
            request_replay_retention_ms: 60_000,
        }
    }

    fn write_registry(directory: &TestDirectory, fixture: &Fixture) {
        let config = base_config(directory);

        let (store, loaded) = NativePassportServerRegistrySnapshotStore::open(
            &config.registry_root,
            context("rustyonions-devnet"),
            context("private-beta"),
        )
        .expect("open registry");

        assert_eq!(loaded.generation, 0);

        let next = NativePassportServerRegistrySnapshotV1 {
            passports: vec![NativePassportServerRootRecordV1 {
                passport_id: fixture.passport_id.clone(),
                root_public_key: fixture.root_public_key.clone(),
                root_key_epoch: 0,
                registered_at_ms: NOW_MS - 20_000,
            }],
            devices: vec![NativePassportServerDeviceRecordV1 {
                passport_id: fixture.passport_id.clone(),
                device_id: fixture.device_id.clone(),
                authorization: fixture.authorization.clone(),
                status: NativePassportServerDeviceStatusV1::Authorized,
                registered_at_ms: NOW_MS - 9_000,
                revoked_at_ms: None,
            }],
        };

        store
            .persist(loaded.generation, &loaded.snapshot, 1, &next)
            .expect("persist registry");
    }

    fn capability(
        fixture: &Fixture,
        expires_at_ms: u64,
        root_key_epoch: Option<u64>,
    ) -> NativePassportDeviceBoundCapabilityV1 {
        NativePassportDeviceBoundCapabilityV1 {
            version: NATIVE_PASSPORT_DEVICE_BOUND_CAPABILITY_V1_VERSION,
            capability_id: CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_A}"))
                .expect("capability ID"),
            passport_id: fixture.passport_id.clone(),
            device_id: fixture.device_id.clone(),
            audience: context("svc-passport"),
            environment: context("private-beta"),
            scopes: vec![scope("identity.read"), scope("identity.username.claim")],
            issued_at_ms: NOW_MS - 1_000,
            expires_at_ms,
            policy_version: NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION,
            root_key_epoch,
        }
    }

    fn write_capability(
        directory: &TestDirectory,
        fixture: &Fixture,
        expires_at_ms: u64,
        root_key_epoch: Option<u64>,
    ) {
        let config = capability_config(directory);

        let (store, loaded) = NativePassportServerCapabilitySnapshotStore::open(
            &config.capability_root,
            context("svc-passport"),
            context("private-beta"),
        )
        .expect("open capability store");

        assert_eq!(loaded.generation, 0);

        let capability = capability(fixture, expires_at_ms, root_key_epoch);

        capability.validate().expect("valid capability fixture");

        let next = NativePassportServerCapabilitySnapshotV1 {
            capabilities: vec![NativePassportServerCapabilityRecordV1 {
                capability,
                issued_from_challenge_id: ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_B}"))
                    .expect("challenge ID"),
                status: NativePassportServerCapabilityStatusV1::Active,
                status_changed_at_ms: NOW_MS - 1_000,
                revoked_at_ms: None,
            }],
        };

        store
            .persist(loaded.generation, &loaded.snapshot, 1, &next)
            .expect("persist capability");
    }

    fn signed_proof(
        fixture: &Fixture,
        body_hash: B3DigestHex,
        request_nonce: B3DigestHex,
    ) -> PassportRequestProofV1 {
        let mut proof = PassportRequestProofV1 {
            version: PASSPORT_REQUEST_PROOF_V1_VERSION,
            capability_id: CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_A}"))
                .expect("capability ID"),
            request_method: USERNAME_CLAIM_REQUEST_METHOD.to_owned(),
            canonical_path: USERNAME_CLAIM_CANONICAL_PATH.to_owned(),
            canonical_query_hash: digest("canonical_query_hash", HEX_C),
            body_hash,
            timestamp_ms: NOW_MS,
            request_nonce,
            device_id: fixture.device_id.clone(),
            device_signature: Ed25519SignatureV1::from_bytes([0_u8; 64]),
        };

        let transcript = canonical_passport_request_proof_v1_transcript(&proof)
            .expect("canonical request-proof transcript");

        let signing_key = SigningKey::from_bytes(&fixture.device_seed_bytes);

        proof.device_signature =
            Ed25519SignatureV1::from_bytes(signing_key.sign(&transcript).to_bytes());

        proof
    }

    fn prepare_valid_authority(directory: &TestDirectory, fixture: &Fixture) {
        write_registry(directory, fixture);

        write_capability(directory, fixture, NOW_MS + 3_600_000, Some(0));
    }

    #[test]
    fn exact_username_claim_proof_admits_once_and_replay_rejects() {
        let directory = TestDirectory::new("exact");
        let fixture = Fixture::new();

        prepare_valid_authority(&directory, &fixture);

        let query_hash = digest("canonical_query_hash", HEX_C);
        let body_hash = digest("body_hash", HEX_D);

        let proof = signed_proof(&fixture, body_hash.clone(), digest("request_nonce", HEX_E));

        let admitted = admit_username_claim_request_proof_v1(
            &base_config(&directory),
            &capability_config(&directory),
            &request_config(&directory),
            &proof,
            &query_hash,
            &body_hash,
            NOW_MS,
        )
        .expect("exact request proof admitted");

        assert_eq!(admitted.passport_id, fixture.passport_id);
        assert_eq!(admitted.device_id, fixture.device_id);

        assert!(matches!(
            admit_username_claim_request_proof_v1(
                &base_config(&directory),
                &capability_config(&directory),
                &request_config(&directory),
                &proof,
                &query_hash,
                &body_hash,
                NOW_MS + 1,
            ),
            Err(NativePassportServerRequestProofRuntimeError::ReplayRejected),
        ));
    }

    #[test]
    fn rejected_proof_does_not_consume_request_nonce() {
        let directory = TestDirectory::new("bad-proof-no-consume");
        let fixture = Fixture::new();

        prepare_valid_authority(&directory, &fixture);

        let query_hash = digest("canonical_query_hash", HEX_C);
        let expected_body_hash = digest("body_hash", HEX_D);
        let nonce = digest("request_nonce", HEX_E);

        let wrong_proof = signed_proof(&fixture, digest("body_hash", HEX_F), nonce.clone());

        assert!(matches!(
            admit_username_claim_request_proof_v1(
                &base_config(&directory),
                &capability_config(&directory),
                &request_config(&directory),
                &wrong_proof,
                &query_hash,
                &expected_body_hash,
                NOW_MS,
            ),
            Err(NativePassportServerRequestProofRuntimeError::ProofRejected),
        ));

        let corrected_proof = signed_proof(&fixture, expected_body_hash.clone(), nonce);

        admit_username_claim_request_proof_v1(
            &base_config(&directory),
            &capability_config(&directory),
            &request_config(&directory),
            &corrected_proof,
            &query_hash,
            &expected_body_hash,
            NOW_MS,
        )
        .expect("same nonce remains usable after rejected proof");
    }

    #[test]
    fn expired_capability_rejects_before_replay_consumption() {
        let directory = TestDirectory::new("expired");
        let fixture = Fixture::new();

        write_registry(&directory, &fixture);

        write_capability(&directory, &fixture, NOW_MS - 1, Some(0));

        let query_hash = digest("canonical_query_hash", HEX_C);
        let body_hash = digest("body_hash", HEX_D);

        let proof = signed_proof(&fixture, body_hash.clone(), digest("request_nonce", HEX_E));

        assert!(matches!(
            admit_username_claim_request_proof_v1(
                &base_config(&directory),
                &capability_config(&directory),
                &request_config(&directory),
                &proof,
                &query_hash,
                &body_hash,
                NOW_MS,
            ),
            Err(NativePassportServerRequestProofRuntimeError::CapabilityRejected),
        ));
    }

    #[test]
    fn capability_root_epoch_must_match_current_root_signed_device_authority() {
        let directory = TestDirectory::new("root-epoch");
        let fixture = Fixture::new();

        write_registry(&directory, &fixture);

        write_capability(&directory, &fixture, NOW_MS + 3_600_000, Some(1));

        let query_hash = digest("canonical_query_hash", HEX_C);
        let body_hash = digest("body_hash", HEX_D);

        let proof = signed_proof(&fixture, body_hash.clone(), digest("request_nonce", HEX_E));

        assert!(matches!(
            admit_username_claim_request_proof_v1(
                &base_config(&directory),
                &capability_config(&directory),
                &request_config(&directory),
                &proof,
                &query_hash,
                &body_hash,
                NOW_MS,
            ),
            Err(NativePassportServerRequestProofRuntimeError::CapabilityRejected),
        ));
    }

    #[test]
    fn request_replay_state_must_be_isolated_from_capability_state() {
        let directory = TestDirectory::new("path-isolation");
        let fixture = Fixture::new();

        let base = base_config(&directory);
        let capability = capability_config(&directory);

        let request = NativePassportServerRequestProofRuntimeConfigV1 {
            request_replay_root: capability.capability_root.clone(),
            request_replay_retention_ms: 60_000,
        };

        let query_hash = digest("canonical_query_hash", HEX_C);
        let body_hash = digest("body_hash", HEX_D);

        let proof = signed_proof(&fixture, body_hash.clone(), digest("request_nonce", HEX_E));

        assert!(matches!(
            admit_username_claim_request_proof_v1(
                &base,
                &capability,
                &request,
                &proof,
                &query_hash,
                &body_hash,
                NOW_MS,
            ),
            Err(NativePassportServerRequestProofRuntimeError::InvalidConfig),
        ));
    }

    #[test]
    fn runtime_source_has_no_http_username_secret_or_value_mutation_authority() {
        let implementation = include_str!("server_request_proof_runtime.rs")
            .split_once("\n#[cfg(test)]")
            .expect("implementation before tests")
            .0;

        for forbidden in [
            "Router::new",
            ".route(",
            "claim_main_username(",
            "UsernameClaimStore",
            "device_private_key",
            "root_private_key",
            "NativeSecretBytes",
            "recovery_factor",
            "sign_native_recovery",
            "raw_pin",
            "wallet.spend(",
            "ledger.write(",
            "reward.issue(",
            "node.control(",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "request-proof runtime gained forbidden authority pattern {forbidden}",
            );
        }
    }
}
