//! RO:WHAT — Private CN-4 server runtime for durable Native Passport `ProveSession` device-possession challenges and one-time proof consumption.
//! RO:WHY — A root-authorized device must prove possession of its registered Ed25519 private key before any device-bound capability or username/profile mutation can be admitted.
//! RO:INTERACTS — durable Native Passport challenge runtime, durable server registry, strict `ron-auth` DeviceAuthorization and device-session proof verifiers, `ron-policy` private-beta device ceilings, and canonical `ron-proto` challenge/identity DTOs.
//! RO:INVARIANTS — only an active durable registered device may receive or satisfy `ProveSession`; requested scopes must fit both the root-signed authorization and current policy; proof public key comes only from durable server state; invalid proof never consumes; successful proof consumes exactly once; registry is never mutated here.
//! RO:METRICS — none yet; later HTTP/service composition owns observable request counters without authority-bearing labels.
//! RO:CONFIG — uses the existing Native Passport runtime mount roots/context/TTL/replay retention; caller cannot choose service context or proof purpose.
//! RO:SECURITY — no RecoveryRoot, PIN, vault unseal, device-secret loading, signing, capability issuance, profile/username mutation, wallet/ledger mutation, or route exposure; live device state is re-read immediately before challenge consumption.
//! RO:TEST — focused unit tests in this module cover one-time proof, replay persistence, bad-proof non-consumption, scope rejection, revocation, and registry non-mutation.

#![forbid(unsafe_code)]

use std::sync::Arc;

use ron_auth::native_passport::{
    passport_challenge_v1_transcript_b3_hex, verify_device_authorization_v1_strict,
    verify_device_session_proof_v1_strict, DeviceAuthorizationVerificationContextV1,
    DeviceSessionProofTranscriptV1,
};
use ron_policy::private_beta_device_authorization_scope_ceiling_v1;
use ron_proto::{
    B3DigestHex, DeviceAuthorizationV1, DeviceIdV1, Ed25519SignatureV1,
    NativePassportContextLabelV1, NativePassportScopeV1, PassportChallengePurposeV1,
    PassportChallengeV1, PassportIdV1,
};
use thiserror::Error;

use crate::kms::client::KmsClient;

use super::{
    server_challenge_issuer::{
        NativePassportServerChallengeIssueRequestV1, NativePassportServerChallengeIssuerError,
    },
    server_challenge_runtime::{
        NativePassportServerChallengeRuntime, NativePassportServerChallengeRuntimeError,
    },
    server_registry_store::{
        NativePassportServerDeviceStatusV1, NativePassportServerRegistrySnapshotStore,
        NativePassportServerRegistryStoreError,
    },
    NativePassportServerRuntimeMountConfigV1, PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
    PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION, PHASE8B_PROOF_CONTRACT_DOMAIN,
    PHASE8B_PROOF_CONTRACT_VERSION,
};

#[derive(Debug, Error)]
pub(super) enum NativePassportServerDeviceSessionError {
    #[error("invalid trusted Native Passport device-session context {field}: {reason}")]
    InvalidTrustedContext { field: &'static str, reason: String },

    #[error("Native Passport device-session request is invalid")]
    InvalidRequest,

    #[error("Native Passport durable registry is unavailable: {0}")]
    RegistryUnavailable(&'static str),

    #[error("Native Passport durable registry is corrupt: {0}")]
    RegistryCorrupt(&'static str),

    #[error("Native Passport is not registered")]
    UnknownPassport,

    #[error("Native Passport device is not registered")]
    UnknownDevice,

    #[error("Native Passport device is revoked")]
    DeviceRevoked,

    #[error("Native Passport device authorization is not currently valid")]
    DeviceAuthorizationRejected,

    #[error("Native Passport current device-class policy rejected the device")]
    DevicePolicyRejected,

    #[error("Native Passport requested scopes exceed current device authority")]
    RequestedScopeRejected,

    #[error("Native Passport challenge runtime failed: {0}")]
    Challenge(#[from] NativePassportServerChallengeRuntimeError),

    #[error("Native Passport challenge transcript hash is invalid")]
    ChallengeTranscriptHashInvalid,

    #[error("Native Passport device-session proof was rejected")]
    ProofRejected,

    #[error("Native Passport durable device authority changed during proof verification")]
    DeviceAuthorityChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativePassportDeviceSessionProofDispositionV1 {
    Proven,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct NativePassportDeviceSessionProofOutcomeV1 {
    pub(super) disposition: NativePassportDeviceSessionProofDispositionV1,
}

#[derive(Debug, Clone)]
pub(super) struct TrustedDeviceSessionContextV1 {
    pub(super) network_id: NativePassportContextLabelV1,
    pub(super) environment: NativePassportContextLabelV1,
    pub(super) audience: NativePassportContextLabelV1,
    pub(super) issuing_service_id: NativePassportContextLabelV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LiveDeviceAuthorityV1 {
    pub(super) registry_generation: u64,
    pub(super) authorization: DeviceAuthorizationV1,
}

pub(super) async fn issue_device_session_challenge_durable(
    config: &NativePassportServerRuntimeMountConfigV1,
    kms: Arc<dyn KmsClient>,
    passport_id: PassportIdV1,
    device_id: DeviceIdV1,
    requested_scopes: Vec<NativePassportScopeV1>,
    now_ms: u64,
) -> Result<PassportChallengeV1, NativePassportServerDeviceSessionError> {
    if now_ms == 0 {
        return Err(NativePassportServerDeviceSessionError::InvalidRequest);
    }

    validate_requested_scope_shape(&requested_scopes)?;

    let trusted = parse_trusted_context(config)?;

    load_live_device_authority(
        config,
        &trusted,
        &passport_id,
        &device_id,
        &requested_scopes,
        now_ms,
    )?;

    let mut challenge_runtime = NativePassportServerChallengeRuntime::open(
        &config.challenge_root,
        kms.as_ref(),
        trusted.network_id,
        trusted.environment,
        trusted.audience,
        trusted.issuing_service_id,
        config.challenge_ttl_ms,
        config.replay_retention_ms,
    )
    .await?;

    let request = NativePassportServerChallengeIssueRequestV1 {
        purpose: PassportChallengePurposeV1::ProveSession,
        requested_scopes,
        passport_id: Some(passport_id),
        device_id: Some(device_id),
        operation_body_hash: None,
    };

    challenge_runtime
        .issue_durable(request, now_ms)
        .await
        .map_err(|error| match error {
            NativePassportServerChallengeRuntimeError::Issuer(
                NativePassportServerChallengeIssuerError::InvalidChallengePayload,
            ) => NativePassportServerDeviceSessionError::InvalidRequest,

            other => NativePassportServerDeviceSessionError::Challenge(other),
        })
}

pub(super) async fn submit_device_session_proof_durable(
    config: &NativePassportServerRuntimeMountConfigV1,
    kms: Arc<dyn KmsClient>,
    challenge: PassportChallengeV1,
    proof_created_at_ms: u64,
    proof_signature: Ed25519SignatureV1,
    accepted_at_ms: u64,
) -> Result<NativePassportDeviceSessionProofOutcomeV1, NativePassportServerDeviceSessionError> {
    if proof_created_at_ms == 0 || accepted_at_ms == 0 {
        return Err(NativePassportServerDeviceSessionError::InvalidRequest);
    }

    challenge
        .validate()
        .map_err(|_| NativePassportServerDeviceSessionError::InvalidRequest)?;

    if challenge.purpose != PassportChallengePurposeV1::ProveSession
        || challenge.operation_body_hash.is_some()
    {
        return Err(NativePassportServerDeviceSessionError::InvalidRequest);
    }

    let passport_id = challenge
        .passport_id
        .as_ref()
        .ok_or(NativePassportServerDeviceSessionError::InvalidRequest)?;

    let device_id = challenge
        .device_id
        .as_ref()
        .ok_or(NativePassportServerDeviceSessionError::InvalidRequest)?;

    validate_requested_scope_shape(&challenge.requested_scopes)?;

    let trusted = parse_trusted_context(config)?;

    if challenge.network_id != trusted.network_id
        || challenge.environment != trusted.environment
        || challenge.audience != trusted.audience
        || challenge.issuing_service_id != trusted.issuing_service_id
    {
        return Err(NativePassportServerDeviceSessionError::InvalidRequest);
    }

    let mut challenge_runtime = NativePassportServerChallengeRuntime::open(
        &config.challenge_root,
        kms.as_ref(),
        trusted.network_id.clone(),
        trusted.environment.clone(),
        trusted.audience.clone(),
        trusted.issuing_service_id.clone(),
        config.challenge_ttl_ms,
        config.replay_retention_ms,
    )
    .await?;

    challenge_runtime
        .validate_consumption_without_mutation(&challenge, accepted_at_ms)
        .await?;

    let authority_before = load_live_device_authority(
        config,
        &trusted,
        passport_id,
        device_id,
        &challenge.requested_scopes,
        accepted_at_ms,
    )?;

    verify_device_challenge_proof_v1(
        &challenge,
        &authority_before.authorization,
        proof_created_at_ms,
        &proof_signature,
    )?;

    /*
     * Registry and challenge replay state are intentionally separate durable
     * stores. Re-read current device authority after cryptographic work and
     * immediately before consumption so a revoke/policy change that occurred
     * during proof verification fails closed.
     *
     * A later capability-issuance boundary must revalidate device state again;
     * this runtime grants no reusable authority itself.
     */
    let authority_before_consume = load_live_device_authority(
        config,
        &trusted,
        passport_id,
        device_id,
        &challenge.requested_scopes,
        accepted_at_ms,
    )?;

    if authority_before_consume != authority_before {
        return Err(NativePassportServerDeviceSessionError::DeviceAuthorityChanged);
    }

    challenge_runtime
        .consume_durable(&challenge, accepted_at_ms)
        .await?;

    Ok(NativePassportDeviceSessionProofOutcomeV1 {
        disposition: NativePassportDeviceSessionProofDispositionV1::Proven,
    })
}

pub(super) fn verify_device_challenge_proof_v1(
    challenge: &PassportChallengeV1,
    authorization: &DeviceAuthorizationV1,
    proof_created_at_ms: u64,
    proof_signature: &Ed25519SignatureV1,
) -> Result<(), NativePassportServerDeviceSessionError> {
    let passport_id = challenge
        .passport_id
        .as_ref()
        .ok_or(NativePassportServerDeviceSessionError::InvalidRequest)?;

    let device_id = challenge
        .device_id
        .as_ref()
        .ok_or(NativePassportServerDeviceSessionError::InvalidRequest)?;

    if authorization.passport_id != *passport_id || authorization.device_id != *device_id {
        return Err(NativePassportServerDeviceSessionError::DeviceAuthorizationRejected);
    }

    let challenge_hash_text = passport_challenge_v1_transcript_b3_hex(&challenge.signing_payload())
        .map_err(|_| NativePassportServerDeviceSessionError::ChallengeTranscriptHashInvalid)?;

    let challenge_transcript_hash =
        B3DigestHex::parse("challenge_transcript_hash", challenge_hash_text)
            .map_err(|_| NativePassportServerDeviceSessionError::ChallengeTranscriptHashInvalid)?;

    let transcript = DeviceSessionProofTranscriptV1 {
        challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
        challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
        proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
        proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
        challenge_id: &challenge.challenge_id,
        network_id: &challenge.network_id,
        environment: &challenge.environment,
        audience: &challenge.audience,
        passport_id,
        device_id,
        device_public_key: &authorization.device_public_key,
        challenge_transcript_hash: &challenge_transcript_hash,
        requested_scopes: &challenge.requested_scopes,
        challenge_issued_at_ms: challenge.issued_at_ms,
        challenge_expires_at_ms: challenge.expires_at_ms,
        proof_created_at_ms,
    };

    verify_device_session_proof_v1_strict(&transcript, proof_signature)
        .map_err(|_| NativePassportServerDeviceSessionError::ProofRejected)
}

pub(super) fn load_live_device_authority(
    config: &NativePassportServerRuntimeMountConfigV1,
    trusted: &TrustedDeviceSessionContextV1,
    passport_id: &PassportIdV1,
    device_id: &DeviceIdV1,
    requested_scopes: &[NativePassportScopeV1],
    now_ms: u64,
) -> Result<LiveDeviceAuthorityV1, NativePassportServerDeviceSessionError> {
    let (_, loaded) = NativePassportServerRegistrySnapshotStore::open(
        &config.registry_root,
        trusted.network_id.clone(),
        trusted.environment.clone(),
    )
    .map_err(map_registry_error)?;

    let root = loaded
        .snapshot
        .passports
        .iter()
        .find(|record| &record.passport_id == passport_id)
        .ok_or(NativePassportServerDeviceSessionError::UnknownPassport)?;

    let device = loaded
        .snapshot
        .devices
        .iter()
        .find(|record| &record.passport_id == passport_id && &record.device_id == device_id)
        .ok_or(NativePassportServerDeviceSessionError::UnknownDevice)?;

    if device.status != NativePassportServerDeviceStatusV1::Authorized {
        return Err(NativePassportServerDeviceSessionError::DeviceRevoked);
    }

    verify_device_authorization_v1_strict(
        &device.authorization,
        DeviceAuthorizationVerificationContextV1 {
            trusted_passport_id: &root.passport_id,
            trusted_root_public_key: &root.root_public_key,
            trusted_root_key_epoch: root.root_key_epoch,
            expected_network_id: &trusted.network_id,
            expected_environment: &trusted.environment,
            now_ms,
            max_clock_skew_ms: 0,
        },
    )
    .map_err(|_| NativePassportServerDeviceSessionError::DeviceAuthorizationRejected)?;

    if device.authorization.passport_id != *passport_id
        || device.authorization.device_id != *device_id
    {
        return Err(NativePassportServerDeviceSessionError::DeviceAuthorizationRejected);
    }

    let current_policy_ceiling =
        private_beta_device_authorization_scope_ceiling_v1(device.authorization.device_class)
            .map_err(|_| NativePassportServerDeviceSessionError::DevicePolicyRejected)?;

    for requested in requested_scopes {
        let root_signed_allowed = device
            .authorization
            .authorized_scope_ceiling
            .as_slice()
            .iter()
            .any(|allowed| allowed == requested);

        let currently_allowed = current_policy_ceiling
            .as_slice()
            .iter()
            .any(|allowed| allowed == requested);

        if !root_signed_allowed || !currently_allowed {
            return Err(NativePassportServerDeviceSessionError::RequestedScopeRejected);
        }
    }

    Ok(LiveDeviceAuthorityV1 {
        registry_generation: loaded.generation,
        authorization: device.authorization.clone(),
    })
}

pub(super) fn validate_requested_scope_shape(
    requested_scopes: &[NativePassportScopeV1],
) -> Result<(), NativePassportServerDeviceSessionError> {
    if requested_scopes.is_empty() {
        return Err(NativePassportServerDeviceSessionError::RequestedScopeRejected);
    }

    if requested_scopes
        .windows(2)
        .any(|pair| pair[0].as_str() >= pair[1].as_str())
    {
        return Err(NativePassportServerDeviceSessionError::RequestedScopeRejected);
    }

    Ok(())
}

pub(super) fn parse_trusted_context(
    config: &NativePassportServerRuntimeMountConfigV1,
) -> Result<TrustedDeviceSessionContextV1, NativePassportServerDeviceSessionError> {
    Ok(TrustedDeviceSessionContextV1 {
        network_id: parse_context("network_id", &config.network_id)?,
        environment: parse_context("environment", &config.environment)?,
        audience: parse_context("audience", &config.audience)?,
        issuing_service_id: parse_context("issuing_service_id", &config.issuing_service_id)?,
    })
}

fn parse_context(
    field: &'static str,
    value: &str,
) -> Result<NativePassportContextLabelV1, NativePassportServerDeviceSessionError> {
    NativePassportContextLabelV1::parse(value).map_err(|error| {
        NativePassportServerDeviceSessionError::InvalidTrustedContext {
            field,
            reason: error.to_string(),
        }
    })
}

fn map_registry_error(
    error: NativePassportServerRegistryStoreError,
) -> NativePassportServerDeviceSessionError {
    match error {
        NativePassportServerRegistryStoreError::Unavailable(reason) => {
            NativePassportServerDeviceSessionError::RegistryUnavailable(reason)
        }

        NativePassportServerRegistryStoreError::Corrupt(reason) => {
            NativePassportServerDeviceSessionError::RegistryCorrupt(reason)
        }
    }
}

/// Stable service-facing error classes for fixed `ProveSession` HTTP composition.
///
/// Detailed store/KMS/challenge failures stay private to the Native Passport
/// runtime and never become a network error oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePassportDeviceSessionServiceError {
    InvalidRequest,
    NotFound,
    Forbidden,
    ChallengeNotConsumable,
    Unavailable,
}

/// Issue one durable, fixed-purpose `ProveSession` challenge.
///
/// The HTTP layer cannot select another challenge purpose because purpose is
/// absent from this service-facing operation.
pub(crate) async fn issue_device_session_challenge_service(
    config: &NativePassportServerRuntimeMountConfigV1,
    kms: Arc<dyn KmsClient>,
    passport_id: PassportIdV1,
    device_id: DeviceIdV1,
    requested_scopes: Vec<NativePassportScopeV1>,
    now_ms: u64,
) -> Result<PassportChallengeV1, NativePassportDeviceSessionServiceError> {
    issue_device_session_challenge_durable(
        config,
        kms,
        passport_id,
        device_id,
        requested_scopes,
        now_ms,
    )
    .await
    .map_err(classify_device_session_service_error)
}

/// Verify and durably consume one fixed `ProveSession` proof.
///
/// This returns only success/failure classification. It does not issue a
/// capability or mutate profile/username state.
pub(crate) async fn submit_device_session_proof_service(
    config: &NativePassportServerRuntimeMountConfigV1,
    kms: Arc<dyn KmsClient>,
    challenge: PassportChallengeV1,
    proof_created_at_ms: u64,
    proof_signature: Ed25519SignatureV1,
    accepted_at_ms: u64,
) -> Result<(), NativePassportDeviceSessionServiceError> {
    submit_device_session_proof_durable(
        config,
        kms,
        challenge,
        proof_created_at_ms,
        proof_signature,
        accepted_at_ms,
    )
    .await
    .map(|_| ())
    .map_err(classify_device_session_service_error)
}

fn classify_device_session_service_error(
    error: NativePassportServerDeviceSessionError,
) -> NativePassportDeviceSessionServiceError {
    match error {
        NativePassportServerDeviceSessionError::InvalidRequest => {
            NativePassportDeviceSessionServiceError::InvalidRequest
        }

        NativePassportServerDeviceSessionError::UnknownPassport
        | NativePassportServerDeviceSessionError::UnknownDevice => {
            NativePassportDeviceSessionServiceError::NotFound
        }

        NativePassportServerDeviceSessionError::DeviceRevoked
        | NativePassportServerDeviceSessionError::DeviceAuthorizationRejected
        | NativePassportServerDeviceSessionError::DevicePolicyRejected
        | NativePassportServerDeviceSessionError::RequestedScopeRejected
        | NativePassportServerDeviceSessionError::ProofRejected => {
            NativePassportDeviceSessionServiceError::Forbidden
        }

        NativePassportServerDeviceSessionError::Challenge(
            NativePassportServerChallengeRuntimeError::UnknownChallenge
            | NativePassportServerChallengeRuntimeError::ChallengeBindingMismatch
            | NativePassportServerChallengeRuntimeError::AlreadyConsumed
            | NativePassportServerChallengeRuntimeError::ChallengeExpired
            | NativePassportServerChallengeRuntimeError::ChallengeCancelled
            | NativePassportServerChallengeRuntimeError::ChallengeNotYetValid,
        ) => NativePassportDeviceSessionServiceError::ChallengeNotConsumable,

        NativePassportServerDeviceSessionError::InvalidTrustedContext { .. }
        | NativePassportServerDeviceSessionError::RegistryUnavailable(_)
        | NativePassportServerDeviceSessionError::RegistryCorrupt(_)
        | NativePassportServerDeviceSessionError::Challenge(_)
        | NativePassportServerDeviceSessionError::ChallengeTranscriptHashInvalid
        | NativePassportServerDeviceSessionError::DeviceAuthorityChanged => {
            NativePassportDeviceSessionServiceError::Unavailable
        }
    }
}

#[cfg(all(test, feature = "dev-kms"))]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use ed25519_dalek::{Signer as _, SigningKey};
    use ron_auth::native_passport::{
        canonical_device_session_proof_v1_transcript, passport_challenge_v1_transcript_b3_hex,
        DeviceSessionProofTranscriptV1,
    };
    use ron_policy::private_beta_device_authorization_scope_ceiling_v1;
    use ron_proto::{
        B3DigestHex, DeviceAuthorizationNonceV1, DeviceAuthorizationSigningPayloadV1,
        DeviceClassV1, DeviceIdV1, Ed25519PublicKeyHex, Ed25519SignatureV1,
        NativePassportContextLabelV1, NativePassportScopeV1, PassportChallengeV1, PassportIdV1,
        DEVICE_AUTHORIZATION_V1_VERSION,
    };

    use crate::{
        kms::client::{DevKms, KmsClient},
        native::{
            derive_native_device_public_identity_v1, derive_native_recovery_public_identity_v1,
            sign_native_recovery_device_authorization_v1, NativeSecretBytes,
        },
    };

    use super::super::server_registry_store::{
        NativePassportServerDeviceRecordV1, NativePassportServerDeviceStatusV1,
        NativePassportServerRegistrySnapshotStore, NativePassportServerRegistrySnapshotV1,
        NativePassportServerRootRecordV1,
    };

    use super::*;

    const ROOT_REGISTERED_AT_MS: u64 = 900_000;
    const AUTHORIZED_AT_MS: u64 = 950_000;
    const DEVICE_REGISTERED_AT_MS: u64 = 960_000;
    const NOW_MS: u64 = 1_000_000;
    const PROOF_AT_MS: u64 = 1_000_010;
    const ACCEPTED_AT_MS: u64 = 1_000_020;
    const AUTH_EXPIRES_AT_MS: u64 = 2_000_000;

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
                    "svc-passport-device-session-{label}-{}-{stamp}",
                    std::process::id(),
                )),
            }
        }

        fn challenge_root(&self) -> PathBuf {
            self.root.join("challenge")
        }

        fn registry_root(&self) -> PathBuf {
            self.root.join("registry")
        }

        fn transaction_root(&self) -> PathBuf {
            self.root.join("transaction")
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
                NativeSecretBytes::new(vec![0x71; 32]).expect("nonphysical recovery factor");

            let root_identity = derive_native_recovery_public_identity_v1(&recovery_factor)
                .expect("root public identity");

            let device_seed_bytes = [0x42; 32];

            let device_seed = NativeSecretBytes::new(device_seed_bytes.to_vec())
                .expect("nonphysical device seed");

            let device_identity = derive_native_device_public_identity_v1(&device_seed)
                .expect("device public identity");

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
                issued_at_ms: AUTHORIZED_AT_MS,
                expires_at_ms: Some(AUTH_EXPIRES_AT_MS),
            };

            let authorization =
                sign_native_recovery_device_authorization_v1(&recovery_factor, payload)
                    .expect("root-signed DeviceAuthorizationV1");

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

    fn requested_scopes() -> Vec<NativePassportScopeV1> {
        vec![scope("catalog.read"), scope("identity.read")]
    }

    fn config(directory: &TestDirectory) -> NativePassportServerRuntimeMountConfigV1 {
        NativePassportServerRuntimeMountConfigV1 {
            challenge_root: directory.challenge_root(),
            registry_root: directory.registry_root(),
            transaction_root: directory.transaction_root(),
            network_id: "rustyonions-devnet".to_owned(),
            environment: "private-beta".to_owned(),
            audience: "svc-passport".to_owned(),
            issuing_service_id: "svc-passport".to_owned(),
            challenge_ttl_ms: 60_000,
            replay_retention_ms: 120_000,
            trusted_initial_root_key_epoch: 0,
        }
    }

    fn write_authorized_registry(directory: &TestDirectory, fixture: &Fixture) {
        let network = context("rustyonions-devnet");
        let environment = context("private-beta");

        let (store, loaded) = NativePassportServerRegistrySnapshotStore::open(
            directory.registry_root(),
            network,
            environment,
        )
        .expect("open empty registry");

        assert_eq!(loaded.generation, 0);
        assert_eq!(
            loaded.snapshot,
            NativePassportServerRegistrySnapshotV1::default(),
        );

        let snapshot = NativePassportServerRegistrySnapshotV1 {
            passports: vec![NativePassportServerRootRecordV1 {
                passport_id: fixture.passport_id.clone(),
                root_public_key: fixture.root_public_key.clone(),
                root_key_epoch: 0,
                registered_at_ms: ROOT_REGISTERED_AT_MS,
            }],
            devices: vec![NativePassportServerDeviceRecordV1 {
                passport_id: fixture.passport_id.clone(),
                device_id: fixture.device_id.clone(),
                authorization: fixture.authorization.clone(),
                status: NativePassportServerDeviceStatusV1::Authorized,
                registered_at_ms: DEVICE_REGISTERED_AT_MS,
                revoked_at_ms: None,
            }],
        };

        store
            .persist(
                0,
                &NativePassportServerRegistrySnapshotV1::default(),
                1,
                &snapshot,
            )
            .expect("persist authorized registry");
    }

    fn load_registry(
        directory: &TestDirectory,
    ) -> (
        NativePassportServerRegistrySnapshotStore,
        super::super::server_registry_store::LoadedNativePassportServerRegistryV1,
    ) {
        NativePassportServerRegistrySnapshotStore::open(
            directory.registry_root(),
            context("rustyonions-devnet"),
            context("private-beta"),
        )
        .expect("open registry")
    }

    fn sign_proof(
        fixture: &Fixture,
        challenge: &PassportChallengeV1,
        signing_key: &SigningKey,
    ) -> Ed25519SignatureV1 {
        let hash_text = passport_challenge_v1_transcript_b3_hex(&challenge.signing_payload())
            .expect("challenge hash");

        let challenge_hash = B3DigestHex::parse("challenge_transcript_hash", hash_text)
            .expect("typed challenge hash");

        let transcript = DeviceSessionProofTranscriptV1 {
            challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
            challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            challenge_id: &challenge.challenge_id,
            network_id: &challenge.network_id,
            environment: &challenge.environment,
            audience: &challenge.audience,
            passport_id: challenge.passport_id.as_ref().expect("Passport binding"),
            device_id: challenge.device_id.as_ref().expect("device binding"),
            device_public_key: &fixture.authorization.device_public_key,
            challenge_transcript_hash: &challenge_hash,
            requested_scopes: &challenge.requested_scopes,
            challenge_issued_at_ms: challenge.issued_at_ms,
            challenge_expires_at_ms: challenge.expires_at_ms,
            proof_created_at_ms: PROOF_AT_MS,
        };

        let bytes = canonical_device_session_proof_v1_transcript(&transcript)
            .expect("canonical device-session transcript");

        Ed25519SignatureV1::from_bytes(signing_key.sign(&bytes).to_bytes())
    }

    async fn issue(
        config: &NativePassportServerRuntimeMountConfigV1,
        kms: Arc<dyn KmsClient>,
        fixture: &Fixture,
    ) -> PassportChallengeV1 {
        issue_device_session_challenge_durable(
            config,
            kms,
            fixture.passport_id.clone(),
            fixture.device_id.clone(),
            requested_scopes(),
            NOW_MS,
        )
        .await
        .expect("durable ProveSession challenge")
    }

    #[tokio::test]
    async fn authorized_device_proves_once_replay_rejects_and_registry_is_unchanged() {
        let directory = TestDirectory::new("prove-once");
        let fixture = Fixture::new();

        write_authorized_registry(&directory, &fixture);

        let config = config(&directory);
        let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());

        let (_, registry_before) = load_registry(&directory);

        let challenge = issue(&config, Arc::clone(&kms), &fixture).await;

        assert_eq!(challenge.purpose, PassportChallengePurposeV1::ProveSession,);
        assert_eq!(challenge.passport_id.as_ref(), Some(&fixture.passport_id),);
        assert_eq!(challenge.device_id.as_ref(), Some(&fixture.device_id),);
        assert!(challenge.operation_body_hash.is_none());
        assert_eq!(challenge.requested_scopes, requested_scopes());

        let device_signing_key = SigningKey::from_bytes(&fixture.device_seed_bytes);

        let signature = sign_proof(&fixture, &challenge, &device_signing_key);

        let outcome = submit_device_session_proof_durable(
            &config,
            Arc::clone(&kms),
            challenge.clone(),
            PROOF_AT_MS,
            signature.clone(),
            ACCEPTED_AT_MS,
        )
        .await
        .expect("first proof succeeds");

        assert_eq!(
            outcome.disposition,
            NativePassportDeviceSessionProofDispositionV1::Proven,
        );

        assert!(matches!(
            submit_device_session_proof_durable(
                &config,
                Arc::clone(&kms),
                challenge,
                PROOF_AT_MS,
                signature,
                ACCEPTED_AT_MS + 1,
            )
            .await,
            Err(NativePassportServerDeviceSessionError::Challenge(
                NativePassportServerChallengeRuntimeError::AlreadyConsumed
            ))
        ));

        let (_, registry_after) = load_registry(&directory);

        assert_eq!(registry_after.generation, registry_before.generation,);
        assert_eq!(registry_after.snapshot, registry_before.snapshot,);
    }

    #[tokio::test]
    async fn wrong_device_proof_does_not_consume_challenge() {
        let directory = TestDirectory::new("wrong-proof");
        let fixture = Fixture::new();

        write_authorized_registry(&directory, &fixture);

        let config = config(&directory);
        let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());

        let challenge = issue(&config, Arc::clone(&kms), &fixture).await;

        let wrong_signing_key = SigningKey::from_bytes(&[0x77; 32]);

        let wrong_signature = sign_proof(&fixture, &challenge, &wrong_signing_key);

        assert!(matches!(
            submit_device_session_proof_durable(
                &config,
                Arc::clone(&kms),
                challenge.clone(),
                PROOF_AT_MS,
                wrong_signature,
                ACCEPTED_AT_MS,
            )
            .await,
            Err(NativePassportServerDeviceSessionError::ProofRejected)
        ));

        let correct_signing_key = SigningKey::from_bytes(&fixture.device_seed_bytes);

        let correct_signature = sign_proof(&fixture, &challenge, &correct_signing_key);

        let outcome = submit_device_session_proof_durable(
            &config,
            Arc::clone(&kms),
            challenge,
            PROOF_AT_MS,
            correct_signature,
            ACCEPTED_AT_MS + 1,
        )
        .await
        .expect("correct retry succeeds");

        assert_eq!(
            outcome.disposition,
            NativePassportDeviceSessionProofDispositionV1::Proven,
        );
    }

    #[tokio::test]
    async fn scope_ceiling_and_revocation_fail_closed_without_capability_authority() {
        let directory = TestDirectory::new("policy-revoke");
        let fixture = Fixture::new();

        write_authorized_registry(&directory, &fixture);

        let config = config(&directory);
        let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());

        assert!(matches!(
            issue_device_session_challenge_durable(
                &config,
                Arc::clone(&kms),
                fixture.passport_id.clone(),
                fixture.device_id.clone(),
                vec![scope("wallet.spend")],
                NOW_MS,
            )
            .await,
            Err(NativePassportServerDeviceSessionError::RequestedScopeRejected)
        ));

        let challenge = issue(&config, Arc::clone(&kms), &fixture).await;

        let correct_signing_key = SigningKey::from_bytes(&fixture.device_seed_bytes);

        let correct_signature = sign_proof(&fixture, &challenge, &correct_signing_key);

        let (store, loaded) = load_registry(&directory);

        let mut revoked = loaded.snapshot.clone();

        let record = revoked
            .devices
            .iter_mut()
            .find(|record| record.device_id == fixture.device_id)
            .expect("registered device");

        record.status = NativePassportServerDeviceStatusV1::Revoked;
        record.revoked_at_ms = Some(NOW_MS + 1);

        store
            .persist(
                loaded.generation,
                &loaded.snapshot,
                loaded.generation + 1,
                &revoked,
            )
            .expect("persist revocation");

        assert!(matches!(
            submit_device_session_proof_durable(
                &config,
                Arc::clone(&kms),
                challenge.clone(),
                PROOF_AT_MS,
                correct_signature,
                ACCEPTED_AT_MS,
            )
            .await,
            Err(NativePassportServerDeviceSessionError::DeviceRevoked)
        ));

        let trusted = parse_trusted_context(&config).expect("trusted context");

        let challenge_runtime = NativePassportServerChallengeRuntime::open(
            &config.challenge_root,
            kms.as_ref(),
            trusted.network_id,
            trusted.environment,
            trusted.audience,
            trusted.issuing_service_id,
            config.challenge_ttl_ms,
            config.replay_retention_ms,
        )
        .await
        .expect("reopen challenge runtime");

        challenge_runtime
            .validate_consumption_without_mutation(&challenge, ACCEPTED_AT_MS)
            .await
            .expect("revocation rejection must leave challenge issued");
    }

    #[test]
    fn device_session_runtime_source_has_no_client_secret_capability_profile_or_value_authority() {
        let implementation = include_str!("server_device_session_runtime.rs")
            .split_once("\n#[cfg(all(test, feature = \"dev-kms\"))]")
            .expect("implementation precedes tests")
            .0;

        for forbidden in [
            "NativeSecureCompartment::RecoveryRoot",
            "unseal_native_secret(",
            "verify_native_recovery_root_pin(",
            "issue_capability(",
            "mint_capability(",
            "claim_username(",
            "update_profile(",
            "wallet.spend(",
            "ledger.write(",
            ".route(",
            "Router::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "device-session production runtime gained forbidden authority pattern: {forbidden}",
            );
        }
    }
}
