//! RO:WHAT — Private crash-recoverable CN-4 runtime for Native Passport V1 device-bound capability issuance.
//! RO:WHY — A registered device that proves its DeviceKey must receive only bounded, short-lived authority whose challenge consumption and capability publication survive process failure without fake success.
//! RO:INTERACTS — server_device_session_runtime live authority/proof verification, durable challenge runtime, capability snapshot store, capability issuance redo journal, durable root/device registry, ron-policy device ceilings, and canonical ron-proto capability/challenge DTOs.
//! RO:INVARIANTS — IssueCapability only; requested scopes are canonical and fit both root-signed/current policy; operation body, policy version, TTL, Passport, Device and scopes are challenge-bound; proof uses the registered DeviceKey; exact authority is re-read before journaling; redo intent precedes challenge consumption; capability publication is idempotent; recovery uses the original admitted time and exact journaled DeviceAuthorization.
//! RO:METRICS — none yet; later HTTP/service composition owns bounded outcome counters without identity-bearing labels.
//! RO:CONFIG — capability state and capability-redo roots are dedicated paths; capability TTL is non-zero and at most the frozen one-hour V1 ceiling.
//! RO:SECURITY — no DeviceKey/root secret/PIN/RecoveryRoot access, no bearer-only mutation authority, no username/profile mutation, no HTTP route, no wallet/ledger/reward/node authority.
//! RO:TEST — focused tests below prove normal issuance, bad-proof non-consumption, durable replay rejection, and restart recovery after journal/consume/publication crash points.

#![forbid(unsafe_code)]

use std::{path::PathBuf, sync::Arc};

use ron_auth::native_passport::{
    passport_challenge_v1_transcript_b3_hex, verify_device_authorization_v1_strict,
    DeviceAuthorizationVerificationContextV1,
};
use ron_policy::NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION;
use ron_proto::{
    B3DigestHex, CapabilityIdV1, DeviceAuthorizationV1, DeviceIdV1, Ed25519SignatureV1,
    NativePassportDeviceBoundCapabilityV1, NativePassportScopeV1, PassportChallengePurposeV1,
    PassportChallengeV1, PassportIdV1, NATIVE_PASSPORT_DEVICE_BOUND_CAPABILITY_V1_VERSION,
};
use thiserror::Error;

use crate::kms::client::KmsClient;

use super::{
    capability_contract::PHASE11C_MAX_CAPABILITY_TTL_MS,
    server_capability_store::{
        LoadedNativePassportServerCapabilityStateV1, NativePassportServerCapabilityRecordV1,
        NativePassportServerCapabilitySnapshotStore, NativePassportServerCapabilityStatusV1,
        NativePassportServerCapabilityStoreError,
    },
    server_capability_txn_store::{
        NativePassportCapabilityIssuanceTxnIntentV1, NativePassportCapabilityIssuanceTxnStore,
        NativePassportCapabilityIssuanceTxnStoreError,
    },
    server_challenge_issuer::{
        NativePassportServerChallengeIssueRequestV1, NativePassportServerChallengeIssuerError,
    },
    server_challenge_runtime::{
        NativePassportServerChallengeRuntime, NativePassportServerChallengeRuntimeError,
    },
    server_device_session_runtime::{
        load_live_device_authority, parse_trusted_context, validate_requested_scope_shape,
        verify_device_challenge_proof_v1, LiveDeviceAuthorityV1,
        NativePassportServerDeviceSessionError, TrustedDeviceSessionContextV1,
    },
    server_registry_store::{
        NativePassportServerRegistrySnapshotStore, NativePassportServerRegistryStoreError,
    },
    NativePassportServerRuntimeMountConfigV1,
};

const ISSUE_CAPABILITY_OPERATION_DOMAIN: &str =
    "rustyonions.native-passport.issue-capability-operation.v1";

const CAPABILITY_ID_DOMAIN: &str = "rustyonions.native-passport.capability-id.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePassportServerCapabilityRuntimeConfigV1 {
    pub capability_root: PathBuf,
    pub transaction_root: PathBuf,
    pub capability_ttl_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePassportServerCapabilityIssueDispositionV1 {
    Issued,
    AlreadyIssued,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativePassportServerCapabilityIssueOutcomeV1 {
    pub(crate) disposition: NativePassportServerCapabilityIssueDispositionV1,
    pub(crate) capability: NativePassportDeviceBoundCapabilityV1,
    pub(crate) durable_generation: u64,
}

#[derive(Debug, Error)]
pub(super) enum NativePassportServerCapabilityRuntimeError {
    #[error("Native Passport capability runtime configuration is invalid")]
    InvalidConfig,

    #[error("Native Passport capability request is invalid")]
    InvalidRequest,

    #[error("Native Passport device authority rejected capability issuance")]
    DeviceAuthority(#[from] NativePassportServerDeviceSessionError),

    #[error("Native Passport challenge runtime failed: {0}")]
    Challenge(#[from] NativePassportServerChallengeRuntimeError),

    #[error("Native Passport capability store unavailable: {0}")]
    CapabilityStoreUnavailable(&'static str),

    #[error("Native Passport capability store corrupt: {0}")]
    CapabilityStoreCorrupt(&'static str),

    #[error("Native Passport capability redo journal failed: {0:?}")]
    TransactionStore(NativePassportCapabilityIssuanceTxnStoreError),

    #[error("Native Passport capability operation binding is invalid")]
    OperationBindingInvalid,

    #[error("Native Passport capability ID derivation failed")]
    CapabilityIdInvalid,

    #[error("Native Passport capability lifetime overflowed")]
    CapabilityTimeOverflow,

    #[error("Native Passport capability conflicts with durable state")]
    CapabilityConflict,

    #[error("Native Passport capability generation overflowed")]
    CapabilityGenerationOverflow,

    #[error("Native Passport recovery authority no longer matches durable identity state")]
    RecoveryAuthorityRejected,
}

impl From<NativePassportCapabilityIssuanceTxnStoreError>
    for NativePassportServerCapabilityRuntimeError
{
    fn from(error: NativePassportCapabilityIssuanceTxnStoreError) -> Self {
        Self::TransactionStore(error)
    }
}

fn map_capability_store_error(
    error: NativePassportServerCapabilityStoreError,
) -> NativePassportServerCapabilityRuntimeError {
    match error {
        NativePassportServerCapabilityStoreError::Unavailable(reason) => {
            NativePassportServerCapabilityRuntimeError::CapabilityStoreUnavailable(reason)
        }
        NativePassportServerCapabilityStoreError::Corrupt(reason) => {
            NativePassportServerCapabilityRuntimeError::CapabilityStoreCorrupt(reason)
        }
    }
}

fn map_registry_recovery_error(
    error: NativePassportServerRegistryStoreError,
) -> NativePassportServerCapabilityRuntimeError {
    match error {
        NativePassportServerRegistryStoreError::Unavailable(_) => {
            NativePassportServerCapabilityRuntimeError::RecoveryAuthorityRejected
        }
        NativePassportServerRegistryStoreError::Corrupt(_) => {
            NativePassportServerCapabilityRuntimeError::RecoveryAuthorityRejected
        }
    }
}

struct NativePassportServerCapabilityCoordinator<'a> {
    base_config: &'a NativePassportServerRuntimeMountConfigV1,
    capability_config: &'a NativePassportServerCapabilityRuntimeConfigV1,

    trusted: TrustedDeviceSessionContextV1,

    challenge_runtime: NativePassportServerChallengeRuntime<'a>,

    capability_store: NativePassportServerCapabilitySnapshotStore,
    capability_state: LoadedNativePassportServerCapabilityStateV1,

    txn_store: NativePassportCapabilityIssuanceTxnStore,
}

impl<'a> NativePassportServerCapabilityCoordinator<'a> {
    async fn open(
        base_config: &'a NativePassportServerRuntimeMountConfigV1,
        capability_config: &'a NativePassportServerCapabilityRuntimeConfigV1,
        kms: &'a dyn KmsClient,
    ) -> Result<Self, NativePassportServerCapabilityRuntimeError> {
        validate_runtime_config(base_config, capability_config)?;

        let trusted = parse_trusted_context(base_config)?;

        let challenge_runtime = NativePassportServerChallengeRuntime::open(
            &base_config.challenge_root,
            kms,
            trusted.network_id.clone(),
            trusted.environment.clone(),
            trusted.audience.clone(),
            trusted.issuing_service_id.clone(),
            base_config.challenge_ttl_ms,
            base_config.replay_retention_ms,
        )
        .await?;

        let (capability_store, capability_state) =
            NativePassportServerCapabilitySnapshotStore::open(
                &capability_config.capability_root,
                trusted.audience.clone(),
                trusted.environment.clone(),
            )
            .map_err(map_capability_store_error)?;

        let (txn_store, pending) =
            NativePassportCapabilityIssuanceTxnStore::open(&capability_config.transaction_root)?;

        let mut coordinator = Self {
            base_config,
            capability_config,
            trusted,
            challenge_runtime,
            capability_store,
            capability_state,
            txn_store,
        };

        coordinator.recover_pending(pending).await?;

        Ok(coordinator)
    }

    async fn issue_challenge(
        &mut self,
        passport_id: PassportIdV1,
        device_id: DeviceIdV1,
        requested_scopes: Vec<NativePassportScopeV1>,
        now_ms: u64,
    ) -> Result<PassportChallengeV1, NativePassportServerCapabilityRuntimeError> {
        if now_ms == 0 {
            return Err(NativePassportServerCapabilityRuntimeError::InvalidRequest);
        }

        validate_requested_scope_shape(&requested_scopes)?;

        load_live_device_authority(
            self.base_config,
            &self.trusted,
            &passport_id,
            &device_id,
            &requested_scopes,
            now_ms,
        )?;

        let operation_body_hash = issue_capability_operation_body_hash_v1(
            &passport_id,
            &device_id,
            &requested_scopes,
            self.capability_config.capability_ttl_ms,
            NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION,
        )?;

        let request = NativePassportServerChallengeIssueRequestV1 {
            purpose: PassportChallengePurposeV1::IssueCapability,
            requested_scopes,
            passport_id: Some(passport_id),
            device_id: Some(device_id),
            operation_body_hash: Some(operation_body_hash),
        };

        self.challenge_runtime
            .issue_durable(request, now_ms)
            .await
            .map_err(|error| match error {
                NativePassportServerChallengeRuntimeError::Issuer(
                    NativePassportServerChallengeIssuerError::InvalidChallengePayload,
                ) => NativePassportServerCapabilityRuntimeError::InvalidRequest,

                other => NativePassportServerCapabilityRuntimeError::Challenge(other),
            })
    }

    async fn submit_proof(
        &mut self,
        challenge: PassportChallengeV1,
        proof_created_at_ms: u64,
        proof_signature: Ed25519SignatureV1,
        accepted_at_ms: u64,
    ) -> Result<
        NativePassportServerCapabilityIssueOutcomeV1,
        NativePassportServerCapabilityRuntimeError,
    > {
        let intent = self
            .prepare_new_intent(
                challenge,
                proof_created_at_ms,
                proof_signature,
                accepted_at_ms,
            )
            .await?;

        self.txn_store.prepare(&intent)?;

        self.apply_durable_intent(&intent).await
    }

    async fn prepare_new_intent(
        &self,
        challenge: PassportChallengeV1,
        proof_created_at_ms: u64,
        proof_signature: Ed25519SignatureV1,
        accepted_at_ms: u64,
    ) -> Result<
        NativePassportCapabilityIssuanceTxnIntentV1,
        NativePassportServerCapabilityRuntimeError,
    > {
        if proof_created_at_ms == 0 || accepted_at_ms == 0 {
            return Err(NativePassportServerCapabilityRuntimeError::InvalidRequest);
        }

        challenge
            .validate()
            .map_err(|_| NativePassportServerCapabilityRuntimeError::InvalidRequest)?;

        if challenge.purpose != PassportChallengePurposeV1::IssueCapability
            || challenge.operation_body_hash.is_none()
        {
            return Err(NativePassportServerCapabilityRuntimeError::InvalidRequest);
        }

        let passport_id = challenge
            .passport_id
            .as_ref()
            .ok_or(NativePassportServerCapabilityRuntimeError::InvalidRequest)?;

        let device_id = challenge
            .device_id
            .as_ref()
            .ok_or(NativePassportServerCapabilityRuntimeError::InvalidRequest)?;

        validate_requested_scope_shape(&challenge.requested_scopes)?;

        if challenge.network_id != self.trusted.network_id
            || challenge.environment != self.trusted.environment
            || challenge.audience != self.trusted.audience
            || challenge.issuing_service_id != self.trusted.issuing_service_id
        {
            return Err(NativePassportServerCapabilityRuntimeError::InvalidRequest);
        }

        let expected_operation_body_hash = issue_capability_operation_body_hash_v1(
            passport_id,
            device_id,
            &challenge.requested_scopes,
            self.capability_config.capability_ttl_ms,
            NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION,
        )?;

        if challenge.operation_body_hash.as_ref() != Some(&expected_operation_body_hash) {
            return Err(NativePassportServerCapabilityRuntimeError::OperationBindingInvalid);
        }

        self.challenge_runtime
            .validate_consumption_without_mutation(&challenge, accepted_at_ms)
            .await?;

        let authority_before = load_live_device_authority(
            self.base_config,
            &self.trusted,
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

        let authority_before_journal = load_live_device_authority(
            self.base_config,
            &self.trusted,
            passport_id,
            device_id,
            &challenge.requested_scopes,
            accepted_at_ms,
        )?;

        if authority_before_journal != authority_before {
            return Err(NativePassportServerCapabilityRuntimeError::DeviceAuthority(
                NativePassportServerDeviceSessionError::DeviceAuthorityChanged,
            ));
        }

        let capability = build_capability_from_challenge(
            &challenge,
            &authority_before,
            accepted_at_ms,
            self.capability_config.capability_ttl_ms,
            NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION,
        )?;

        Ok(NativePassportCapabilityIssuanceTxnIntentV1 {
            challenge,
            proof_created_at_ms,
            proof_signature,
            device_authorization: authority_before.authorization,
            capability,
            accepted_at_ms,
        })
    }

    async fn recover_pending(
        &mut self,
        pending: Vec<NativePassportCapabilityIssuanceTxnIntentV1>,
    ) -> Result<(), NativePassportServerCapabilityRuntimeError> {
        for intent in pending {
            self.apply_durable_intent(&intent).await?;
        }

        Ok(())
    }

    async fn apply_durable_intent(
        &mut self,
        intent: &NativePassportCapabilityIssuanceTxnIntentV1,
    ) -> Result<
        NativePassportServerCapabilityIssueOutcomeV1,
        NativePassportServerCapabilityRuntimeError,
    > {
        self.validate_recovery_intent(intent)?;

        self.validate_journaled_authority(intent)?;

        verify_device_challenge_proof_v1(
            &intent.challenge,
            &intent.device_authorization,
            intent.proof_created_at_ms,
            &intent.proof_signature,
        )?;

        match self
            .challenge_runtime
            .consume_durable(&intent.challenge, intent.accepted_at_ms)
            .await
        {
            Ok(()) => {}

            Err(NativePassportServerChallengeRuntimeError::AlreadyConsumed) => {}

            Err(error) => {
                return Err(NativePassportServerCapabilityRuntimeError::Challenge(error));
            }
        }

        let outcome = self.record_capability(intent)?;

        self.txn_store.complete(intent)?;

        Ok(outcome)
    }

    fn validate_recovery_intent(
        &self,
        intent: &NativePassportCapabilityIssuanceTxnIntentV1,
    ) -> Result<(), NativePassportServerCapabilityRuntimeError> {
        if intent.challenge.purpose != PassportChallengePurposeV1::IssueCapability {
            return Err(NativePassportServerCapabilityRuntimeError::OperationBindingInvalid);
        }

        let passport_id = intent
            .challenge
            .passport_id
            .as_ref()
            .ok_or(NativePassportServerCapabilityRuntimeError::OperationBindingInvalid)?;

        let device_id = intent
            .challenge
            .device_id
            .as_ref()
            .ok_or(NativePassportServerCapabilityRuntimeError::OperationBindingInvalid)?;

        let capability_ttl_ms = intent
            .capability
            .expires_at_ms
            .checked_sub(intent.capability.issued_at_ms)
            .ok_or(NativePassportServerCapabilityRuntimeError::OperationBindingInvalid)?;

        if capability_ttl_ms == 0 || capability_ttl_ms > PHASE11C_MAX_CAPABILITY_TTL_MS {
            return Err(NativePassportServerCapabilityRuntimeError::OperationBindingInvalid);
        }

        let expected_operation_body_hash = issue_capability_operation_body_hash_v1(
            passport_id,
            device_id,
            &intent.challenge.requested_scopes,
            capability_ttl_ms,
            intent.capability.policy_version,
        )?;

        if intent.challenge.operation_body_hash.as_ref() != Some(&expected_operation_body_hash) {
            return Err(NativePassportServerCapabilityRuntimeError::OperationBindingInvalid);
        }

        let expected = build_capability_from_authorization(
            &intent.challenge,
            &intent.device_authorization,
            intent.accepted_at_ms,
            capability_ttl_ms,
            intent.capability.policy_version,
        )?;

        if expected != intent.capability {
            return Err(NativePassportServerCapabilityRuntimeError::OperationBindingInvalid);
        }

        Ok(())
    }

    fn validate_journaled_authority(
        &self,
        intent: &NativePassportCapabilityIssuanceTxnIntentV1,
    ) -> Result<(), NativePassportServerCapabilityRuntimeError> {
        let passport_id = intent
            .challenge
            .passport_id
            .as_ref()
            .ok_or(NativePassportServerCapabilityRuntimeError::RecoveryAuthorityRejected)?;

        let device_id = intent
            .challenge
            .device_id
            .as_ref()
            .ok_or(NativePassportServerCapabilityRuntimeError::RecoveryAuthorityRejected)?;

        let (_, loaded) = NativePassportServerRegistrySnapshotStore::open(
            &self.base_config.registry_root,
            self.trusted.network_id.clone(),
            self.trusted.environment.clone(),
        )
        .map_err(map_registry_recovery_error)?;

        let root = loaded
            .snapshot
            .passports
            .iter()
            .find(|record| &record.passport_id == passport_id)
            .ok_or(NativePassportServerCapabilityRuntimeError::RecoveryAuthorityRejected)?;

        let device = loaded
            .snapshot
            .devices
            .iter()
            .find(|record| &record.passport_id == passport_id && &record.device_id == device_id)
            .ok_or(NativePassportServerCapabilityRuntimeError::RecoveryAuthorityRejected)?;

        if device.authorization != intent.device_authorization {
            return Err(NativePassportServerCapabilityRuntimeError::RecoveryAuthorityRejected);
        }

        verify_device_authorization_v1_strict(
            &intent.device_authorization,
            DeviceAuthorizationVerificationContextV1 {
                trusted_passport_id: &root.passport_id,
                trusted_root_public_key: &root.root_public_key,
                trusted_root_key_epoch: root.root_key_epoch,
                expected_network_id: &self.trusted.network_id,
                expected_environment: &self.trusted.environment,
                now_ms: intent.accepted_at_ms,
                max_clock_skew_ms: 0,
            },
        )
        .map_err(|_| NativePassportServerCapabilityRuntimeError::RecoveryAuthorityRejected)?;

        Ok(())
    }

    fn record_capability(
        &mut self,
        intent: &NativePassportCapabilityIssuanceTxnIntentV1,
    ) -> Result<
        NativePassportServerCapabilityIssueOutcomeV1,
        NativePassportServerCapabilityRuntimeError,
    > {
        let expected_record = NativePassportServerCapabilityRecordV1 {
            capability: intent.capability.clone(),
            issued_from_challenge_id: intent.challenge.challenge_id.clone(),
            status: NativePassportServerCapabilityStatusV1::Active,
            status_changed_at_ms: intent.capability.issued_at_ms,
            revoked_at_ms: None,
        };

        if let Some(existing) = self
            .capability_state
            .snapshot
            .capabilities
            .iter()
            .find(|record| {
                record.capability.capability_id == expected_record.capability.capability_id
            })
        {
            if existing == &expected_record {
                return Ok(NativePassportServerCapabilityIssueOutcomeV1 {
                    disposition: NativePassportServerCapabilityIssueDispositionV1::AlreadyIssued,
                    capability: intent.capability.clone(),
                    durable_generation: self.capability_state.generation,
                });
            }

            return Err(NativePassportServerCapabilityRuntimeError::CapabilityConflict);
        }

        if self
            .capability_state
            .snapshot
            .capabilities
            .iter()
            .any(|record| record.issued_from_challenge_id == intent.challenge.challenge_id)
        {
            return Err(NativePassportServerCapabilityRuntimeError::CapabilityConflict);
        }

        let mut next_snapshot = self.capability_state.snapshot.clone();

        let insertion_index = next_snapshot
            .capabilities
            .binary_search_by(|record| {
                record
                    .capability
                    .capability_id
                    .as_str()
                    .cmp(expected_record.capability.capability_id.as_str())
            })
            .unwrap_or_else(|index| index);

        next_snapshot
            .capabilities
            .insert(insertion_index, expected_record);

        let next_generation = self
            .capability_state
            .generation
            .checked_add(1)
            .ok_or(NativePassportServerCapabilityRuntimeError::CapabilityGenerationOverflow)?;

        self.capability_store
            .persist(
                self.capability_state.generation,
                &self.capability_state.snapshot,
                next_generation,
                &next_snapshot,
            )
            .map_err(map_capability_store_error)?;

        self.capability_state = LoadedNativePassportServerCapabilityStateV1 {
            generation: next_generation,
            snapshot: next_snapshot,
        };

        Ok(NativePassportServerCapabilityIssueOutcomeV1 {
            disposition: NativePassportServerCapabilityIssueDispositionV1::Issued,
            capability: intent.capability.clone(),
            durable_generation: next_generation,
        })
    }
}

pub(crate) async fn issue_capability_challenge_durable(
    base_config: &NativePassportServerRuntimeMountConfigV1,
    capability_config: &NativePassportServerCapabilityRuntimeConfigV1,
    kms: Arc<dyn KmsClient>,
    passport_id: PassportIdV1,
    device_id: DeviceIdV1,
    requested_scopes: Vec<NativePassportScopeV1>,
    now_ms: u64,
) -> Result<PassportChallengeV1, NativePassportServerCapabilityRuntimeError> {
    let mut coordinator = NativePassportServerCapabilityCoordinator::open(
        base_config,
        capability_config,
        kms.as_ref(),
    )
    .await?;

    coordinator
        .issue_challenge(passport_id, device_id, requested_scopes, now_ms)
        .await
}

pub(crate) async fn submit_capability_proof_durable(
    base_config: &NativePassportServerRuntimeMountConfigV1,
    capability_config: &NativePassportServerCapabilityRuntimeConfigV1,
    kms: Arc<dyn KmsClient>,
    challenge: PassportChallengeV1,
    proof_created_at_ms: u64,
    proof_signature: Ed25519SignatureV1,
    accepted_at_ms: u64,
) -> Result<NativePassportServerCapabilityIssueOutcomeV1, NativePassportServerCapabilityRuntimeError>
{
    let mut coordinator = NativePassportServerCapabilityCoordinator::open(
        base_config,
        capability_config,
        kms.as_ref(),
    )
    .await?;

    coordinator
        .submit_proof(
            challenge,
            proof_created_at_ms,
            proof_signature,
            accepted_at_ms,
        )
        .await
}

/// Stable service-facing error classes for the fixed IssueCapability surface.
///
/// Detailed durable-store, registry, challenge, and recovery failures stay
/// inside svc-passport and never become a network error oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePassportCapabilityServiceError {
    InvalidRequest,
    NotFound,
    Forbidden,
    ChallengeNotConsumable,
    Conflict,
    Unavailable,
}

/// Stable service-facing issuance result class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePassportCapabilityServiceDispositionV1 {
    Issued,
    AlreadyIssued,
}

/// Public-safe result retained by the future HTTP adapter.
///
/// The capability DTO contains public authority metadata and no DeviceKey,
/// RecoveryRoot, PIN, root private key, or bearer secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativePassportCapabilityServiceOutcomeV1 {
    pub(crate) disposition: NativePassportCapabilityServiceDispositionV1,
    pub(crate) capability: NativePassportDeviceBoundCapabilityV1,
    pub(crate) durable_generation: u64,
}

/// Open/recover the capability runtime without adding a route.
///
/// Startup composition uses this as a truthful readiness gate: pending redo
/// transactions must recover successfully before capability HTTP is mounted.
pub(crate) async fn preflight_native_passport_capability_runtime(
    base_config: &NativePassportServerRuntimeMountConfigV1,
    capability_config: &NativePassportServerCapabilityRuntimeConfigV1,
    kms: Arc<dyn KmsClient>,
) -> Result<(), NativePassportCapabilityServiceError> {
    let coordinator = NativePassportServerCapabilityCoordinator::open(
        base_config,
        capability_config,
        kms.as_ref(),
    )
    .await
    .map_err(classify_capability_service_error)?;

    drop(coordinator);

    Ok(())
}

/// Issue one fixed-purpose durable IssueCapability challenge.
///
/// Purpose, TTL, policy version, network, environment, audience, service KID,
/// operation hash, capability ID, and trusted time remain server-owned.
pub(crate) async fn issue_capability_challenge_service(
    base_config: &NativePassportServerRuntimeMountConfigV1,
    capability_config: &NativePassportServerCapabilityRuntimeConfigV1,
    kms: Arc<dyn KmsClient>,
    passport_id: PassportIdV1,
    device_id: DeviceIdV1,
    requested_scopes: Vec<NativePassportScopeV1>,
    now_ms: u64,
) -> Result<PassportChallengeV1, NativePassportCapabilityServiceError> {
    issue_capability_challenge_durable(
        base_config,
        capability_config,
        kms,
        passport_id,
        device_id,
        requested_scopes,
        now_ms,
    )
    .await
    .map_err(classify_capability_service_error)
}

/// Verify DeviceKey proof and durably finish one IssueCapability transaction.
pub(crate) async fn submit_capability_proof_service(
    base_config: &NativePassportServerRuntimeMountConfigV1,
    capability_config: &NativePassportServerCapabilityRuntimeConfigV1,
    kms: Arc<dyn KmsClient>,
    challenge: PassportChallengeV1,
    proof_created_at_ms: u64,
    proof_signature: Ed25519SignatureV1,
    accepted_at_ms: u64,
) -> Result<NativePassportCapabilityServiceOutcomeV1, NativePassportCapabilityServiceError> {
    let outcome = submit_capability_proof_durable(
        base_config,
        capability_config,
        kms,
        challenge,
        proof_created_at_ms,
        proof_signature,
        accepted_at_ms,
    )
    .await
    .map_err(classify_capability_service_error)?;

    let disposition = match outcome.disposition {
        NativePassportServerCapabilityIssueDispositionV1::Issued => {
            NativePassportCapabilityServiceDispositionV1::Issued
        }

        NativePassportServerCapabilityIssueDispositionV1::AlreadyIssued => {
            NativePassportCapabilityServiceDispositionV1::AlreadyIssued
        }
    };

    Ok(NativePassportCapabilityServiceOutcomeV1 {
        disposition,
        capability: outcome.capability,
        durable_generation: outcome.durable_generation,
    })
}

fn classify_capability_service_error(
    error: NativePassportServerCapabilityRuntimeError,
) -> NativePassportCapabilityServiceError {
    match error {
        NativePassportServerCapabilityRuntimeError::InvalidRequest
        | NativePassportServerCapabilityRuntimeError::OperationBindingInvalid => {
            NativePassportCapabilityServiceError::InvalidRequest
        }

        NativePassportServerCapabilityRuntimeError::DeviceAuthority(error) => {
            classify_device_authority_service_error(error)
        }

        NativePassportServerCapabilityRuntimeError::Challenge(error) => {
            classify_challenge_service_error(error)
        }

        NativePassportServerCapabilityRuntimeError::CapabilityConflict
        | NativePassportServerCapabilityRuntimeError::TransactionStore(
            NativePassportCapabilityIssuanceTxnStoreError::Conflict,
        ) => NativePassportCapabilityServiceError::Conflict,

        NativePassportServerCapabilityRuntimeError::InvalidConfig
        | NativePassportServerCapabilityRuntimeError::CapabilityStoreUnavailable(_)
        | NativePassportServerCapabilityRuntimeError::CapabilityStoreCorrupt(_)
        | NativePassportServerCapabilityRuntimeError::TransactionStore(_)
        | NativePassportServerCapabilityRuntimeError::CapabilityIdInvalid
        | NativePassportServerCapabilityRuntimeError::CapabilityTimeOverflow
        | NativePassportServerCapabilityRuntimeError::CapabilityGenerationOverflow
        | NativePassportServerCapabilityRuntimeError::RecoveryAuthorityRejected => {
            NativePassportCapabilityServiceError::Unavailable
        }
    }
}

fn classify_device_authority_service_error(
    error: NativePassportServerDeviceSessionError,
) -> NativePassportCapabilityServiceError {
    match error {
        NativePassportServerDeviceSessionError::InvalidRequest => {
            NativePassportCapabilityServiceError::InvalidRequest
        }

        NativePassportServerDeviceSessionError::UnknownPassport
        | NativePassportServerDeviceSessionError::UnknownDevice => {
            NativePassportCapabilityServiceError::NotFound
        }

        NativePassportServerDeviceSessionError::DeviceRevoked
        | NativePassportServerDeviceSessionError::DeviceAuthorizationRejected
        | NativePassportServerDeviceSessionError::DevicePolicyRejected
        | NativePassportServerDeviceSessionError::RequestedScopeRejected
        | NativePassportServerDeviceSessionError::ProofRejected => {
            NativePassportCapabilityServiceError::Forbidden
        }

        NativePassportServerDeviceSessionError::Challenge(error) => {
            classify_challenge_service_error(error)
        }

        NativePassportServerDeviceSessionError::InvalidTrustedContext { .. }
        | NativePassportServerDeviceSessionError::RegistryUnavailable(_)
        | NativePassportServerDeviceSessionError::RegistryCorrupt(_)
        | NativePassportServerDeviceSessionError::ChallengeTranscriptHashInvalid
        | NativePassportServerDeviceSessionError::DeviceAuthorityChanged => {
            NativePassportCapabilityServiceError::Unavailable
        }
    }
}

fn classify_challenge_service_error(
    error: NativePassportServerChallengeRuntimeError,
) -> NativePassportCapabilityServiceError {
    match error {
        NativePassportServerChallengeRuntimeError::UnknownChallenge
        | NativePassportServerChallengeRuntimeError::ChallengeBindingMismatch
        | NativePassportServerChallengeRuntimeError::AlreadyConsumed
        | NativePassportServerChallengeRuntimeError::ChallengeExpired
        | NativePassportServerChallengeRuntimeError::ChallengeCancelled
        | NativePassportServerChallengeRuntimeError::ChallengeNotYetValid => {
            NativePassportCapabilityServiceError::ChallengeNotConsumable
        }

        _ => NativePassportCapabilityServiceError::Unavailable,
    }
}

fn validate_runtime_config(
    base_config: &NativePassportServerRuntimeMountConfigV1,
    capability_config: &NativePassportServerCapabilityRuntimeConfigV1,
) -> Result<(), NativePassportServerCapabilityRuntimeError> {
    if capability_config.capability_ttl_ms == 0
        || capability_config.capability_ttl_ms > PHASE11C_MAX_CAPABILITY_TTL_MS
    {
        return Err(NativePassportServerCapabilityRuntimeError::InvalidConfig);
    }

    let capability_root = &capability_config.capability_root;
    let capability_txn_root = &capability_config.transaction_root;

    if capability_root == capability_txn_root
        || capability_root == &base_config.challenge_root
        || capability_root == &base_config.registry_root
        || capability_root == &base_config.transaction_root
        || capability_txn_root == &base_config.challenge_root
        || capability_txn_root == &base_config.registry_root
        || capability_txn_root == &base_config.transaction_root
    {
        return Err(NativePassportServerCapabilityRuntimeError::InvalidConfig);
    }

    Ok(())
}

fn issue_capability_operation_body_hash_v1(
    passport_id: &PassportIdV1,
    device_id: &DeviceIdV1,
    scopes: &[NativePassportScopeV1],
    capability_ttl_ms: u64,
    policy_version: u16,
) -> Result<B3DigestHex, NativePassportServerCapabilityRuntimeError> {
    let mut transcript = Vec::with_capacity(1024);

    push_len_prefixed(
        &mut transcript,
        ISSUE_CAPABILITY_OPERATION_DOMAIN.as_bytes(),
    )?;

    push_len_prefixed(&mut transcript, passport_id.as_str().as_bytes())?;

    push_len_prefixed(&mut transcript, device_id.as_str().as_bytes())?;

    let scope_count = u16::try_from(scopes.len())
        .map_err(|_| NativePassportServerCapabilityRuntimeError::OperationBindingInvalid)?;

    transcript.extend_from_slice(&scope_count.to_be_bytes());

    for scope in scopes {
        push_len_prefixed(&mut transcript, scope.as_str().as_bytes())?;
    }

    transcript.extend_from_slice(&capability_ttl_ms.to_be_bytes());
    transcript.extend_from_slice(&policy_version.to_be_bytes());

    B3DigestHex::parse(
        "issue_capability_operation_body_hash",
        blake3::hash(&transcript).to_hex().to_string(),
    )
    .map_err(|_| NativePassportServerCapabilityRuntimeError::OperationBindingInvalid)
}

fn capability_id_from_challenge(
    challenge: &PassportChallengeV1,
) -> Result<CapabilityIdV1, NativePassportServerCapabilityRuntimeError> {
    let challenge_hash = passport_challenge_v1_transcript_b3_hex(&challenge.signing_payload())
        .map_err(|_| NativePassportServerCapabilityRuntimeError::CapabilityIdInvalid)?;

    let mut transcript = Vec::with_capacity(192);

    push_len_prefixed(&mut transcript, CAPABILITY_ID_DOMAIN.as_bytes())?;

    push_len_prefixed(&mut transcript, challenge_hash.as_bytes())?;

    let digest = blake3::hash(&transcript).to_hex().to_string();

    CapabilityIdV1::parse(format!("capability:v1:b3:{digest}"))
        .map_err(|_| NativePassportServerCapabilityRuntimeError::CapabilityIdInvalid)
}

fn build_capability_from_challenge(
    challenge: &PassportChallengeV1,
    authority: &LiveDeviceAuthorityV1,
    accepted_at_ms: u64,
    capability_ttl_ms: u64,
    policy_version: u16,
) -> Result<NativePassportDeviceBoundCapabilityV1, NativePassportServerCapabilityRuntimeError> {
    build_capability_from_authorization(
        challenge,
        &authority.authorization,
        accepted_at_ms,
        capability_ttl_ms,
        policy_version,
    )
}

fn build_capability_from_authorization(
    challenge: &PassportChallengeV1,
    authorization: &DeviceAuthorizationV1,
    accepted_at_ms: u64,
    capability_ttl_ms: u64,
    policy_version: u16,
) -> Result<NativePassportDeviceBoundCapabilityV1, NativePassportServerCapabilityRuntimeError> {
    let passport_id = challenge
        .passport_id
        .clone()
        .ok_or(NativePassportServerCapabilityRuntimeError::OperationBindingInvalid)?;

    let device_id = challenge
        .device_id
        .clone()
        .ok_or(NativePassportServerCapabilityRuntimeError::OperationBindingInvalid)?;

    if authorization.passport_id != passport_id || authorization.device_id != device_id {
        return Err(NativePassportServerCapabilityRuntimeError::RecoveryAuthorityRejected);
    }

    let expires_at_ms = accepted_at_ms
        .checked_add(capability_ttl_ms)
        .ok_or(NativePassportServerCapabilityRuntimeError::CapabilityTimeOverflow)?;

    let capability = NativePassportDeviceBoundCapabilityV1 {
        version: NATIVE_PASSPORT_DEVICE_BOUND_CAPABILITY_V1_VERSION,
        capability_id: capability_id_from_challenge(challenge)?,
        passport_id,
        device_id,
        audience: challenge.audience.clone(),
        environment: challenge.environment.clone(),
        scopes: challenge.requested_scopes.clone(),
        issued_at_ms: accepted_at_ms,
        expires_at_ms,
        policy_version,
        root_key_epoch: Some(authorization.root_key_epoch),
    };

    capability
        .validate()
        .map_err(|_| NativePassportServerCapabilityRuntimeError::OperationBindingInvalid)?;

    Ok(capability)
}

fn push_len_prefixed(
    output: &mut Vec<u8>,
    bytes: &[u8],
) -> Result<(), NativePassportServerCapabilityRuntimeError> {
    let length = u16::try_from(bytes.len())
        .map_err(|_| NativePassportServerCapabilityRuntimeError::OperationBindingInvalid)?;

    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);

    Ok(())
}

#[cfg(all(test, feature = "dev-kms"))]
mod tests {
    use std::{
        fs,
        path::PathBuf,
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
        DeviceClassV1, Ed25519PublicKeyHex, NativePassportContextLabelV1, NativePassportScopeV1,
        DEVICE_AUTHORIZATION_V1_VERSION,
    };

    use crate::{
        kms::client::{DevKms, KmsClient},
        native::{
            derive_native_device_public_identity_v1, derive_native_recovery_public_identity_v1,
            sign_native_recovery_device_authorization_v1, NativeSecretBytes,
            PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
            PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
        },
    };

    use super::super::{
        server_capability_store::{
            NativePassportServerCapabilitySnapshotStore, NativePassportServerCapabilityStatusV1,
        },
        server_capability_txn_store::NativePassportCapabilityIssuanceTxnStore,
        server_registry_store::{
            NativePassportServerDeviceRecordV1, NativePassportServerDeviceStatusV1,
            NativePassportServerRegistrySnapshotStore, NativePassportServerRegistrySnapshotV1,
            NativePassportServerRootRecordV1,
        },
    };

    use super::*;

    const ROOT_REGISTERED_AT_MS: u64 = 900_000;
    const AUTHORIZED_AT_MS: u64 = 950_000;
    const DEVICE_REGISTERED_AT_MS: u64 = 960_000;
    const NOW_MS: u64 = 1_000_000;
    const PROOF_AT_MS: u64 = 1_000_010;
    const ACCEPTED_AT_MS: u64 = 1_000_020;
    const AUTH_EXPIRES_AT_MS: u64 = 5_000_000;

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
                    "svc-passport-capability-runtime-{label}-{}-{stamp}",
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

        fn root_transaction_root(&self) -> PathBuf {
            self.root.join("root-transaction")
        }

        fn capability_root(&self) -> PathBuf {
            self.root.join("capability")
        }

        fn capability_transaction_root(&self) -> PathBuf {
            self.root.join("capability-transaction")
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
            let recovery_factor = NativeSecretBytes::new(vec![0x71; 32]).expect("recovery factor");

            let root_identity =
                derive_native_recovery_public_identity_v1(&recovery_factor).expect("root identity");

            let device_seed_bytes = [0x42; 32];

            let device_seed =
                NativeSecretBytes::new(device_seed_bytes.to_vec()).expect("device seed");

            let device_identity =
                derive_native_device_public_identity_v1(&device_seed).expect("device identity");

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
                    .expect("policy ceiling");

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
                    .expect("signed authorization");

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
        vec![scope("identity.read"), scope("identity.username.claim")]
    }

    fn base_config(directory: &TestDirectory) -> NativePassportServerRuntimeMountConfigV1 {
        NativePassportServerRuntimeMountConfigV1 {
            challenge_root: directory.challenge_root(),
            registry_root: directory.registry_root(),
            transaction_root: directory.root_transaction_root(),
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
            capability_root: directory.capability_root(),
            transaction_root: directory.capability_transaction_root(),
            capability_ttl_ms: 3_600_000,
        }
    }

    fn write_authorized_registry(directory: &TestDirectory, fixture: &Fixture) {
        let (store, loaded) = NativePassportServerRegistrySnapshotStore::open(
            directory.registry_root(),
            context("rustyonions-devnet"),
            context("private-beta"),
        )
        .expect("open registry");

        assert_eq!(loaded.generation, 0);

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
            .expect("persist registry");
    }

    fn sign_proof(fixture: &Fixture, challenge: &PassportChallengeV1) -> Ed25519SignatureV1 {
        let hash_text = passport_challenge_v1_transcript_b3_hex(&challenge.signing_payload())
            .expect("challenge hash");

        let challenge_hash =
            B3DigestHex::parse("challenge_transcript_hash", hash_text).expect("challenge hash DTO");

        let transcript = DeviceSessionProofTranscriptV1 {
            challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,
            challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            challenge_id: &challenge.challenge_id,
            network_id: &challenge.network_id,
            environment: &challenge.environment,
            audience: &challenge.audience,
            passport_id: challenge.passport_id.as_ref().expect("Passport"),
            device_id: challenge.device_id.as_ref().expect("Device"),
            device_public_key: &fixture.authorization.device_public_key,
            challenge_transcript_hash: &challenge_hash,
            requested_scopes: &challenge.requested_scopes,
            challenge_issued_at_ms: challenge.issued_at_ms,
            challenge_expires_at_ms: challenge.expires_at_ms,
            proof_created_at_ms: PROOF_AT_MS,
        };

        let bytes =
            canonical_device_session_proof_v1_transcript(&transcript).expect("proof transcript");

        let signing_key = SigningKey::from_bytes(&fixture.device_seed_bytes);

        Ed25519SignatureV1::from_bytes(signing_key.sign(&bytes).to_bytes())
    }

    async fn issue(
        directory: &TestDirectory,
        fixture: &Fixture,
        kms: Arc<dyn KmsClient>,
    ) -> PassportChallengeV1 {
        issue_capability_challenge_durable(
            &base_config(directory),
            &capability_config(directory),
            kms,
            fixture.passport_id.clone(),
            fixture.device_id.clone(),
            requested_scopes(),
            NOW_MS,
        )
        .await
        .expect("IssueCapability challenge")
    }

    #[tokio::test]
    async fn valid_device_proof_issues_one_durable_device_bound_capability() {
        let directory = TestDirectory::new("normal");

        let fixture = Fixture::new();

        write_authorized_registry(&directory, &fixture);

        let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());

        let challenge = issue(&directory, &fixture, Arc::clone(&kms)).await;

        assert_eq!(
            challenge.purpose,
            PassportChallengePurposeV1::IssueCapability,
        );

        assert!(challenge.operation_body_hash.is_some());

        let signature = sign_proof(&fixture, &challenge);

        let outcome = submit_capability_proof_durable(
            &base_config(&directory),
            &capability_config(&directory),
            Arc::clone(&kms),
            challenge.clone(),
            PROOF_AT_MS,
            signature.clone(),
            ACCEPTED_AT_MS,
        )
        .await
        .expect("issue capability");

        assert_eq!(
            outcome.disposition,
            NativePassportServerCapabilityIssueDispositionV1::Issued,
        );

        assert_eq!(outcome.durable_generation, 1);

        assert_eq!(outcome.capability.passport_id, fixture.passport_id,);

        assert_eq!(outcome.capability.device_id, fixture.device_id,);

        assert_eq!(outcome.capability.scopes, requested_scopes(),);

        assert_eq!(
            outcome.capability.policy_version,
            NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION,
        );

        assert_eq!(outcome.capability.root_key_epoch, Some(0),);

        let (_, loaded) = NativePassportServerCapabilitySnapshotStore::open(
            directory.capability_root(),
            context("svc-passport"),
            context("private-beta"),
        )
        .expect("reopen capability state");

        assert_eq!(loaded.generation, 1);
        assert_eq!(loaded.snapshot.capabilities.len(), 1);
        assert_eq!(
            loaded.snapshot.capabilities[0].status,
            NativePassportServerCapabilityStatusV1::Active,
        );

        let (_, pending) =
            NativePassportCapabilityIssuanceTxnStore::open(directory.capability_transaction_root())
                .expect("reopen journal");

        assert!(pending.is_empty());

        assert!(matches!(
            submit_capability_proof_durable(
                &base_config(&directory),
                &capability_config(&directory),
                Arc::clone(&kms),
                challenge,
                PROOF_AT_MS,
                signature,
                ACCEPTED_AT_MS,
            )
            .await,
            Err(NativePassportServerCapabilityRuntimeError::Challenge(
                NativePassportServerChallengeRuntimeError::AlreadyConsumed
            ))
        ));
    }

    #[tokio::test]
    async fn bad_device_proof_never_consumes_or_journals_and_correct_retry_succeeds() {
        let directory = TestDirectory::new("bad-proof");

        let fixture = Fixture::new();

        write_authorized_registry(&directory, &fixture);

        let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());

        let challenge = issue(&directory, &fixture, Arc::clone(&kms)).await;

        assert!(matches!(
            submit_capability_proof_durable(
                &base_config(&directory),
                &capability_config(&directory),
                Arc::clone(&kms),
                challenge.clone(),
                PROOF_AT_MS,
                Ed25519SignatureV1::from_bytes([0x99; 64]),
                ACCEPTED_AT_MS,
            )
            .await,
            Err(NativePassportServerCapabilityRuntimeError::DeviceAuthority(
                NativePassportServerDeviceSessionError::ProofRejected
            ))
        ));

        let (_, capabilities) = NativePassportServerCapabilitySnapshotStore::open(
            directory.capability_root(),
            context("svc-passport"),
            context("private-beta"),
        )
        .expect("capability store");

        assert!(capabilities.snapshot.capabilities.is_empty());

        let (_, pending) =
            NativePassportCapabilityIssuanceTxnStore::open(directory.capability_transaction_root())
                .expect("journal");

        assert!(pending.is_empty());

        let signature = sign_proof(&fixture, &challenge);

        submit_capability_proof_durable(
            &base_config(&directory),
            &capability_config(&directory),
            kms,
            challenge,
            PROOF_AT_MS,
            signature,
            ACCEPTED_AT_MS,
        )
        .await
        .expect("correct proof after rejected proof");
    }

    #[tokio::test]
    async fn restart_recovers_journal_consume_and_capability_publication_crash_points() {
        for crash_point in ["journal-only", "after-consume", "after-capability"] {
            let directory = TestDirectory::new(crash_point);

            let fixture = Fixture::new();

            write_authorized_registry(&directory, &fixture);

            let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());

            let base = base_config(&directory);
            let cap = capability_config(&directory);

            let mut coordinator =
                NativePassportServerCapabilityCoordinator::open(&base, &cap, kms.as_ref())
                    .await
                    .expect("open coordinator");

            let challenge = coordinator
                .issue_challenge(
                    fixture.passport_id.clone(),
                    fixture.device_id.clone(),
                    requested_scopes(),
                    NOW_MS,
                )
                .await
                .expect("challenge");

            let signature = sign_proof(&fixture, &challenge);

            let intent = coordinator
                .prepare_new_intent(challenge, PROOF_AT_MS, signature, ACCEPTED_AT_MS)
                .await
                .expect("accepted intent");

            coordinator
                .txn_store
                .prepare(&intent)
                .expect("journal intent");

            if crash_point == "after-consume" || crash_point == "after-capability" {
                coordinator
                    .challenge_runtime
                    .consume_durable(&intent.challenge, intent.accepted_at_ms)
                    .await
                    .expect("consume before simulated crash");
            }

            if crash_point == "after-capability" {
                coordinator
                    .record_capability(&intent)
                    .expect("publish capability before crash");
            }

            drop(coordinator);

            let recovered =
                NativePassportServerCapabilityCoordinator::open(&base, &cap, kms.as_ref())
                    .await
                    .expect("restart recovery");

            assert_eq!(
                recovered.capability_state.snapshot.capabilities.len(),
                1,
                "{crash_point}",
            );

            assert_eq!(
                recovered.capability_state.snapshot.capabilities[0].capability, intent.capability,
                "{crash_point}",
            );

            assert!(
                recovered
                    .txn_store
                    .load_pending()
                    .expect("pending")
                    .is_empty(),
                "{crash_point}",
            );
        }
    }

    #[test]
    fn runtime_source_has_no_http_username_secret_or_value_authority() {
        let implementation = include_str!("server_capability_runtime.rs")
            .split_once("\n#[cfg(all(test, feature = \"dev-kms\"))]")
            .expect("implementation before tests")
            .0;

        for forbidden in [
            "Router::new",
            ".route(",
            "claim_username(",
            "profile.update(",
            "device_private_key",
            "root_private_key",
            "recovery_phrase",
            "raw_pin",
            "wallet.spend(",
            "ledger.write(",
            "reward.issue(",
            "node.control(",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "capability runtime gained forbidden authority pattern {forbidden}",
            );
        }
    }
}
