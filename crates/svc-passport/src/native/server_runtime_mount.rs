//! RO:WHAT — Recovery-gated Native Passport server composition for durable RegisterRoot challenge issuance and proof submission.
//! RO:WHY — CrabNode must not advertise Passport readiness or accept root authority until trusted context, service KMS, challenge replay state, registry state, and pending redo intents are valid.
//! RO:INTERACTS — `KmsClient`, `ron-auth` canonical challenge/root-proof transcripts, `NativePassportServerRootRegistrationCoordinator`, durable challenge/registry/redo stores, and constrained HTTP handlers.
//! RO:INVARIANTS — startup recovery fails closed; challenge issuance is durable-before-return; RegisterRoot proof submission reconstructs canonical server-owned bindings and uses the existing crash-recoverable coordinator; accepted time and initial root epoch are never caller authority.
//! RO:METRICS — none yet; service composition/readiness owns runtime readiness reporting.
//! RO:CONFIG — explicit challenge/registry/transaction roots, network/environment/audience/service labels, challenge TTL, replay retention, and trusted initial root epoch.
//! RO:SECURITY — KMS is injected; no DevKms construction, key export, root secret, PIN, capability issuance, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — `crabnode_cn4_native_runtime_mount`, `crabnode_cn4_register_root_challenge_route`, and `crabnode_cn4_register_root_proof_route`.

#![forbid(unsafe_code)]

use std::{path::PathBuf, sync::Arc};

use ron_auth::native_passport::{
    passport_challenge_v1_transcript_b3_hex, RootRegistrationProofTranscriptV1,
};
use ron_proto::{
    B3DigestHex, Ed25519PublicKeyHex, NativePassportContextLabelV1, NativePassportScopeV1,
    PassportChallengePurposeV1, PassportChallengeV1, PassportIdV1,
};
use thiserror::Error;

use crate::kms::client::KmsClient;

use super::{
    proof_signing_adapter::NativeProofSignedPayloadHex,
    server_challenge_issuer::{
        NativePassportServerChallengeIssueRequestV1, NativePassportServerChallengeIssuerError,
    },
    server_challenge_runtime::{
        NativePassportServerChallengeRuntime, NativePassportServerChallengeRuntimeError,
    },
    server_registry_runtime::{
        NativePassportServerRootRegistrationDispositionV1,
        NativePassportServerRootRegistrationError,
    },
    server_root_registration_coordinator::{
        NativePassportServerRootRegistrationCoordinator,
        NativePassportServerRootRegistrationCoordinatorError,
    },
    PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
    PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportServerRuntimeMountConfigV1 {
    pub challenge_root: PathBuf,
    pub registry_root: PathBuf,
    pub transaction_root: PathBuf,

    pub network_id: String,
    pub environment: String,
    pub audience: String,
    pub issuing_service_id: String,

    pub challenge_ttl_ms: u64,
    pub replay_retention_ms: u64,
    pub trusted_initial_root_key_epoch: u64,
}

#[derive(Debug, Error)]
pub enum NativePassportServerRuntimeMountError {
    #[error("invalid trusted Native Passport context {field}: {reason}")]
    InvalidTrustedContext { field: &'static str, reason: String },

    #[error("Native Passport service KMS unavailable: {0}")]
    KmsUnavailable(String),

    #[error("Native Passport service KMS returned an empty active signing key id")]
    InvalidServiceKeyIdentity,

    #[error("Native Passport durable runtime recovery failed: {0}")]
    DurableRuntimeRecovery(String),

    #[error("Native Passport RegisterRoot challenge request is invalid")]
    InvalidRegisterRootChallengeRequest,

    #[error("Native Passport durable challenge issuance failed: {0}")]
    DurableChallengeIssue(String),

    #[error("Native Passport RegisterRoot proof request is invalid")]
    InvalidRegisterRootProofRequest,

    #[error("Native Passport RegisterRoot proof was rejected")]
    RegisterRootProofRejected,

    #[error("Native Passport RegisterRoot proof replay was rejected")]
    RegisterRootProofReplayRejected,

    #[error("Native Passport RegisterRoot conflicts with durable authority")]
    RegisterRootConflict,

    #[error("Native Passport durable root registration unavailable: {0}")]
    DurableRootRegistration(String),
}

/// Redacted result class returned to the constrained HTTP layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePassportRegisterRootSubmitDispositionV1 {
    Registered,
    AlreadyRegistered,
}

/// Public-safe durable result from the private RegisterRoot coordinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativePassportRegisterRootSubmitOutcomeV1 {
    pub(crate) disposition: NativePassportRegisterRootSubmitDispositionV1,
    pub(crate) durable_generation: u64,
}

pub(crate) async fn preflight_native_passport_server_runtime_mount(
    config: &NativePassportServerRuntimeMountConfigV1,
    kms: Arc<dyn KmsClient>,
) -> Result<(), NativePassportServerRuntimeMountError> {
    let network_id = parse_context("network_id", &config.network_id)?;

    let environment = parse_context("environment", &config.environment)?;

    let audience = parse_context("audience", &config.audience)?;

    let issuing_service_id = parse_context("issuing_service_id", &config.issuing_service_id)?;

    /*
     * Readiness for the mounted CN-4 identity plane requires a usable service
     * signing identity. The exact key remains behind KmsClient; only the public
     * identity is inspected here.
     */
    let signing_identity = kms.active_signing_identity().await.map_err(|error| {
        NativePassportServerRuntimeMountError::KmsUnavailable(error.to_string())
    })?;

    if signing_identity.kid.trim().is_empty() {
        return Err(NativePassportServerRuntimeMountError::InvalidServiceKeyIdentity);
    }

    /*
     * Opening the coordinator is a recovery operation, not merely a file
     * existence check. Any durable pending RegisterRoot redo transaction must
     * complete successfully before this service surface may become ready.
     */
    let coordinator = NativePassportServerRootRegistrationCoordinator::open(
        &config.challenge_root,
        &config.registry_root,
        &config.transaction_root,
        kms.as_ref(),
        network_id,
        environment,
        audience,
        issuing_service_id,
        config.challenge_ttl_ms,
        config.replay_retention_ms,
        config.trusted_initial_root_key_epoch,
    )
    .await
    .map_err(|error| {
        NativePassportServerRuntimeMountError::DurableRuntimeRecovery(format!("{error:?}"))
    })?;

    drop(coordinator);

    Ok(())
}

/// Issue one durable RegisterRoot challenge using only server-owned trusted
/// context plus the three caller bindings admitted by the registration route.
///
/// Opening the lifecycle runtime revalidates already-stored challenge
/// signatures before new issuance. `issue_durable` then persists the new
/// Issued record before the challenge may escape this function.
pub(crate) async fn issue_register_root_challenge_durable(
    config: &NativePassportServerRuntimeMountConfigV1,
    kms: Arc<dyn KmsClient>,
    passport_id: PassportIdV1,
    requested_scopes: Vec<NativePassportScopeV1>,
    operation_body_hash: B3DigestHex,
    now_ms: u64,
) -> Result<PassportChallengeV1, NativePassportServerRuntimeMountError> {
    let network_id = parse_context("network_id", &config.network_id)?;

    let environment = parse_context("environment", &config.environment)?;

    let audience = parse_context("audience", &config.audience)?;

    let issuing_service_id = parse_context("issuing_service_id", &config.issuing_service_id)?;

    let mut runtime = NativePassportServerChallengeRuntime::open(
        &config.challenge_root,
        kms.as_ref(),
        network_id,
        environment,
        audience,
        issuing_service_id,
        config.challenge_ttl_ms,
        config.replay_retention_ms,
    )
    .await
    .map_err(|error| {
        NativePassportServerRuntimeMountError::DurableChallengeIssue(error.to_string())
    })?;

    let request = NativePassportServerChallengeIssueRequestV1 {
        /*
         * Purpose is forced by the route/runtime composition and never
         * accepted from the HTTP caller.
         */
        purpose: PassportChallengePurposeV1::RegisterRoot,

        requested_scopes,

        passport_id: Some(passport_id),

        /*
         * RegisterRoot is root authority, not device authority.
         */
        device_id: None,

        operation_body_hash: Some(operation_body_hash),
    };

    match runtime.issue_durable(request, now_ms).await {
        Ok(challenge) => Ok(challenge),

        Err(NativePassportServerChallengeRuntimeError::Issuer(
            NativePassportServerChallengeIssuerError::InvalidChallengePayload,
        )) => Err(NativePassportServerRuntimeMountError::InvalidRegisterRootChallengeRequest),

        Err(error) => {
            Err(NativePassportServerRuntimeMountError::DurableChallengeIssue(error.to_string()))
        }
    }
}

/// Verify and durably complete one fixed RegisterRoot proof transaction.
///
/// The caller presents the exact signed challenge and concrete root proof
/// material. All protocol domains, trusted deployment context, initial root
/// epoch, challenge hash, scopes, operation binding, and acceptance time are
/// reconstructed or supplied by trusted service composition.
pub(crate) async fn submit_register_root_proof_durable(
    config: &NativePassportServerRuntimeMountConfigV1,
    kms: Arc<dyn KmsClient>,
    challenge: PassportChallengeV1,
    root_public_key: Ed25519PublicKeyHex,
    proof_created_at_ms: u64,
    proof_signed_payload_hex: String,
    accepted_at_ms: u64,
) -> Result<NativePassportRegisterRootSubmitOutcomeV1, NativePassportServerRuntimeMountError> {
    if accepted_at_ms == 0 || proof_created_at_ms == 0 {
        return Err(NativePassportServerRuntimeMountError::InvalidRegisterRootProofRequest);
    }

    challenge
        .validate()
        .map_err(|_| NativePassportServerRuntimeMountError::InvalidRegisterRootProofRequest)?;

    if challenge.purpose != PassportChallengePurposeV1::RegisterRoot
        || challenge.device_id.is_some()
    {
        return Err(NativePassportServerRuntimeMountError::InvalidRegisterRootProofRequest);
    }

    let passport_id = challenge
        .passport_id
        .as_ref()
        .ok_or(NativePassportServerRuntimeMountError::InvalidRegisterRootProofRequest)?;

    let operation_body_hash = challenge
        .operation_body_hash
        .as_ref()
        .ok_or(NativePassportServerRuntimeMountError::InvalidRegisterRootProofRequest)?;

    let challenge_hash_text = passport_challenge_v1_transcript_b3_hex(&challenge.signing_payload())
        .map_err(|_| NativePassportServerRuntimeMountError::InvalidRegisterRootProofRequest)?;

    let challenge_transcript_hash =
        B3DigestHex::parse("challenge_transcript_hash", challenge_hash_text)
            .map_err(|_| NativePassportServerRuntimeMountError::InvalidRegisterRootProofRequest)?;

    let requested_scopes: Vec<&str> = challenge
        .requested_scopes
        .iter()
        .map(|scope| scope.as_str())
        .collect();

    let signed_payload = NativeProofSignedPayloadHex::parse(
        "root_registration_proof_signed_payload_hex",
        proof_signed_payload_hex,
    )
    .map_err(|_| NativePassportServerRuntimeMountError::InvalidRegisterRootProofRequest)?;

    /*
     * Initial registration epoch is service policy. It is deliberately not
     * accepted from the HTTP caller.
     */
    let root_key_epoch = config.trusted_initial_root_key_epoch;

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

        root_key_epoch,

        device_id: None,

        operation_body_hash,

        challenge_transcript_hash: &challenge_transcript_hash,

        requested_scopes: &requested_scopes,

        challenge_issued_at_ms: challenge.issued_at_ms,

        challenge_expires_at_ms: challenge.expires_at_ms,

        proof_created_at_ms,
    };

    let network_id = parse_context("network_id", &config.network_id)?;

    let environment = parse_context("environment", &config.environment)?;

    let audience = parse_context("audience", &config.audience)?;

    let issuing_service_id = parse_context("issuing_service_id", &config.issuing_service_id)?;

    /*
     * Open is also the recovery boundary. Every request first resolves any
     * previously durable RegisterRoot redo intent before admitting a new one.
     */
    let mut coordinator = NativePassportServerRootRegistrationCoordinator::open(
        &config.challenge_root,
        &config.registry_root,
        &config.transaction_root,
        kms.as_ref(),
        network_id,
        environment,
        audience,
        issuing_service_id,
        config.challenge_ttl_ms,
        config.replay_retention_ms,
        config.trusted_initial_root_key_epoch,
    )
    .await
    .map_err(map_root_registration_error)?;

    let outcome = coordinator
        .register_root_recoverably(&challenge, &transcript, &signed_payload, accepted_at_ms)
        .await
        .map_err(map_root_registration_error)?;

    let disposition = match outcome.disposition {
        NativePassportServerRootRegistrationDispositionV1::Registered => {
            NativePassportRegisterRootSubmitDispositionV1::Registered
        }

        NativePassportServerRootRegistrationDispositionV1::AlreadyRegistered => {
            NativePassportRegisterRootSubmitDispositionV1::AlreadyRegistered
        }
    };

    Ok(NativePassportRegisterRootSubmitOutcomeV1 {
        disposition,
        durable_generation: outcome.durable_generation,
    })
}

fn map_root_registration_error(
    error: NativePassportServerRootRegistrationCoordinatorError,
) -> NativePassportServerRuntimeMountError {
    match error {
        NativePassportServerRootRegistrationCoordinatorError::Challenge(challenge_error) => {
            match challenge_error {
                NativePassportServerChallengeRuntimeError::AlreadyConsumed
                | NativePassportServerChallengeRuntimeError::ChallengeExpired
                | NativePassportServerChallengeRuntimeError::ChallengeCancelled => {
                    NativePassportServerRuntimeMountError::RegisterRootProofReplayRejected
                }

                NativePassportServerChallengeRuntimeError::UnknownChallenge
                | NativePassportServerChallengeRuntimeError::ChallengeBindingMismatch
                | NativePassportServerChallengeRuntimeError::ChallengeNotYetValid => {
                    NativePassportServerRuntimeMountError::RegisterRootProofRejected
                }

                _ => NativePassportServerRuntimeMountError::DurableRootRegistration(
                    "challenge runtime unavailable".to_owned(),
                ),
            }
        }

        NativePassportServerRootRegistrationCoordinatorError::Registry(registry_error) => {
            match registry_error {
                NativePassportServerRootRegistrationError::RootRegistrationConflict => {
                    NativePassportServerRuntimeMountError::RegisterRootConflict
                }

                NativePassportServerRootRegistrationError::ChallengeContractMismatch
                | NativePassportServerRootRegistrationError::ProofContractMismatch
                | NativePassportServerRootRegistrationError::TrustedNetworkMismatch
                | NativePassportServerRootRegistrationError::TrustedEnvironmentMismatch
                | NativePassportServerRootRegistrationError::AudienceMismatch
                | NativePassportServerRootRegistrationError::InitialRootEpochMismatch
                | NativePassportServerRootRegistrationError::UnexpectedDeviceBinding
                | NativePassportServerRootRegistrationError::InvalidRegistrationTime
                | NativePassportServerRootRegistrationError::InvalidSignedPayload
                | NativePassportServerRootRegistrationError::RootProofVerificationFailed
                | NativePassportServerRootRegistrationError::RootPublicKeyInvalid
                | NativePassportServerRootRegistrationError::PassportIdDerivationFailed
                | NativePassportServerRootRegistrationError::PassportRootBindingMismatch => {
                    NativePassportServerRuntimeMountError::RegisterRootProofRejected
                }

                NativePassportServerRootRegistrationError::GenerationOverflow
                | NativePassportServerRootRegistrationError::StorageUnavailable(_)
                | NativePassportServerRootRegistrationError::StorageCorrupt(_) => {
                    NativePassportServerRuntimeMountError::DurableRootRegistration(
                        "registry runtime unavailable".to_owned(),
                    )
                }
            }
        }

        NativePassportServerRootRegistrationCoordinatorError::ChallengeTranscriptHashInvalid
        | NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(_) => {
            NativePassportServerRuntimeMountError::RegisterRootProofRejected
        }

        NativePassportServerRootRegistrationCoordinatorError::TransactionStore(_)
        | NativePassportServerRootRegistrationCoordinatorError::RecoveryIntentInvalid(_) => {
            NativePassportServerRuntimeMountError::DurableRootRegistration(
                "root registration recovery unavailable".to_owned(),
            )
        }
    }
}

fn parse_context(
    field: &'static str,
    value: &str,
) -> Result<NativePassportContextLabelV1, NativePassportServerRuntimeMountError> {
    NativePassportContextLabelV1::parse(value).map_err(|error| {
        NativePassportServerRuntimeMountError::InvalidTrustedContext {
            field,
            reason: error.to_string(),
        }
    })
}
