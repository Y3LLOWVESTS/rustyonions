//! RO:WHAT — Private crash-recoverable coordinator for one Native Passport RegisterRoot transaction.
//! RO:WHY — Root registration and one-time challenge consumption use separate durable stores; the redo journal turns an accepted proof into a recoverable transaction rather than two unrelated writes.
//! RO:INTERACTS — server_challenge_runtime, server_registry_runtime, server_root_registration_txn_store, ron-auth canonical challenge/root proof transcripts, and KmsClient.
//! RO:INVARIANTS — both non-mutating preflights pass before journal prepare; durable redo intent precedes authoritative mutation; exact challenge consumption precedes root mutation; root registration remains idempotent; pending intents are replayed before open succeeds; completion removes intent only after both authorities reached the intended state.
//! RO:METRICS — none yet; future mounted service orchestration owns bounded recovery/transaction metrics.
//! RO:CONFIG — service-owned challenge/registry/journal roots plus trusted network/environment/audience/service context, challenge TTL/replay retention, and initial root epoch.
//! RO:SECURITY — journal contains public signed evidence only; no RecoveryRoot, seed, PIN, root/device private key, capability, username, wallet, or ledger authority; no public route is added here.
//! RO:TEST — focused tests prove normal commit plus restart recovery after intent, challenge-consume, and root-registration crash points; mismatched challenge/proof binding never journals.

#![forbid(unsafe_code)]

use std::path::Path;

use ron_auth::native_passport::{
    passport_challenge_v1_transcript_b3_hex, RootRegistrationProofTranscriptV1,
};
use ron_proto::{
    B3DigestHex, Ed25519PublicKeyHex, NativePassportContextLabelV1, PassportChallengePurposeV1,
    PassportChallengeV1,
};

use crate::kms::client::KmsClient;

use super::{
    proof_signing_adapter::NativeProofSignedPayloadHex,
    server_challenge_runtime::{
        NativePassportServerChallengeRuntime, NativePassportServerChallengeRuntimeError,
    },
    server_registry_runtime::{
        NativePassportServerRootRegistrationError, NativePassportServerRootRegistrationOutcomeV1,
        NativePassportServerRootRegistryRuntime,
    },
    server_root_registration_txn_store::{
        NativePassportRootRegistrationTxnIntentV1, NativePassportRootRegistrationTxnStore,
        NativePassportRootRegistrationTxnStoreError,
    },
    PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
    PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NativePassportServerRootRegistrationCoordinatorError {
    Challenge(NativePassportServerChallengeRuntimeError),

    Registry(NativePassportServerRootRegistrationError),

    TransactionStore(NativePassportRootRegistrationTxnStoreError),

    ChallengeTranscriptHashInvalid,

    ChallengeProofBindingMismatch(&'static str),

    RecoveryIntentInvalid(&'static str),
}

impl From<NativePassportServerChallengeRuntimeError>
    for NativePassportServerRootRegistrationCoordinatorError
{
    fn from(error: NativePassportServerChallengeRuntimeError) -> Self {
        Self::Challenge(error)
    }
}

impl From<NativePassportServerRootRegistrationError>
    for NativePassportServerRootRegistrationCoordinatorError
{
    fn from(error: NativePassportServerRootRegistrationError) -> Self {
        Self::Registry(error)
    }
}

impl From<NativePassportRootRegistrationTxnStoreError>
    for NativePassportServerRootRegistrationCoordinatorError
{
    fn from(error: NativePassportRootRegistrationTxnStoreError) -> Self {
        Self::TransactionStore(error)
    }
}

/// Private composition owner for the complete RegisterRoot durable transaction.
///
/// `open` is itself a recovery gate: it does not return a usable coordinator
/// until every previously durable redo intent has been completed or recovery
/// has failed closed.
pub(super) struct NativePassportServerRootRegistrationCoordinator<'a> {
    challenge_runtime: NativePassportServerChallengeRuntime<'a>,

    registry_runtime: NativePassportServerRootRegistryRuntime,

    txn_store: NativePassportRootRegistrationTxnStore,
}

impl<'a> NativePassportServerRootRegistrationCoordinator<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn open(
        challenge_root: impl AsRef<Path>,
        registry_root: impl AsRef<Path>,
        transaction_root: impl AsRef<Path>,
        kms: &'a dyn KmsClient,
        network_id: NativePassportContextLabelV1,
        environment: NativePassportContextLabelV1,
        audience: NativePassportContextLabelV1,
        issuing_service_id: NativePassportContextLabelV1,
        challenge_ttl_ms: u64,
        replay_retention_ms: u64,
        trusted_initial_root_key_epoch: u64,
    ) -> Result<Self, NativePassportServerRootRegistrationCoordinatorError> {
        let challenge_runtime = NativePassportServerChallengeRuntime::open(
            challenge_root,
            kms,
            network_id.clone(),
            environment.clone(),
            audience,
            issuing_service_id,
            challenge_ttl_ms,
            replay_retention_ms,
        )
        .await?;

        let registry_runtime = NativePassportServerRootRegistryRuntime::open(
            registry_root,
            network_id,
            environment,
            trusted_initial_root_key_epoch,
        )?;

        let (txn_store, pending) = NativePassportRootRegistrationTxnStore::open(transaction_root)?;

        let mut coordinator = Self {
            challenge_runtime,
            registry_runtime,
            txn_store,
        };

        coordinator.recover_pending(pending).await?;

        Ok(coordinator)
    }

    /// Accept and durably complete one RegisterRoot proof.
    ///
    /// No durable intent is created until both the exact challenge and the
    /// concrete root proof have passed their existing non-mutating authority
    /// checks.
    pub(super) async fn register_root_recoverably(
        &mut self,
        challenge: &PassportChallengeV1,
        transcript: &RootRegistrationProofTranscriptV1<'_>,
        signed_payload: &NativeProofSignedPayloadHex,
        accepted_at_ms: u64,
    ) -> Result<
        NativePassportServerRootRegistrationOutcomeV1,
        NativePassportServerRootRegistrationCoordinatorError,
    > {
        self.validate_new_transaction(challenge, transcript, signed_payload, accepted_at_ms)
            .await?;

        let intent = self.build_intent(challenge, transcript, signed_payload, accepted_at_ms)?;

        self.txn_store.prepare(&intent)?;

        self.apply_durable_intent(&intent).await
    }

    async fn validate_new_transaction(
        &self,
        challenge: &PassportChallengeV1,
        transcript: &RootRegistrationProofTranscriptV1<'_>,
        signed_payload: &NativeProofSignedPayloadHex,
        accepted_at_ms: u64,
    ) -> Result<(), NativePassportServerRootRegistrationCoordinatorError> {
        self.challenge_runtime
            .validate_consumption_without_mutation(challenge, accepted_at_ms)
            .await?;

        self.validate_challenge_proof_binding(challenge, transcript)?;

        self.registry_runtime
            .validate_root_registration_without_mutation(
                transcript,
                signed_payload,
                accepted_at_ms,
            )?;

        Ok(())
    }

    fn build_intent(
        &self,
        challenge: &PassportChallengeV1,
        transcript: &RootRegistrationProofTranscriptV1<'_>,
        signed_payload: &NativeProofSignedPayloadHex,
        accepted_at_ms: u64,
    ) -> Result<
        NativePassportRootRegistrationTxnIntentV1,
        NativePassportServerRootRegistrationCoordinatorError,
    > {
        self.validate_challenge_proof_binding(challenge, transcript)?;

        Ok(NativePassportRootRegistrationTxnIntentV1 {
            challenge: challenge.clone(),

            challenge_transcript_hash: transcript.challenge_transcript_hash.as_str().to_owned(),

            root_public_key: transcript.root_public_key.as_str().to_owned(),

            root_key_epoch: transcript.root_key_epoch,

            proof_signed_payload_hex: signed_payload.as_str().to_owned(),

            proof_created_at_ms: transcript.proof_created_at_ms,

            accepted_at_ms,
        })
    }

    fn validate_challenge_proof_binding(
        &self,
        challenge: &PassportChallengeV1,
        transcript: &RootRegistrationProofTranscriptV1<'_>,
    ) -> Result<(), NativePassportServerRootRegistrationCoordinatorError> {
        if challenge.purpose != PassportChallengePurposeV1::RegisterRoot {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    "challenge purpose is not RegisterRoot",
                ),
            );
        }

        if transcript.challenge_contract_domain != PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN
            || transcript.challenge_contract_version != PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION
            || transcript.proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
            || transcript.proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
        {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    "challenge/proof contract mismatch",
                ),
            );
        }

        if transcript.challenge_id != &challenge.challenge_id {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    "challenge ID mismatch",
                ),
            );
        }

        if transcript.network_id != challenge.network_id.as_str()
            || transcript.environment != challenge.environment.as_str()
            || transcript.audience != challenge.audience.as_str()
        {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    "trusted challenge context mismatch",
                ),
            );
        }

        let challenge_passport = challenge.passport_id.as_ref().ok_or(
            NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                "RegisterRoot challenge missing Passport binding",
            ),
        )?;

        if transcript.passport_id != challenge_passport {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    "Passport binding mismatch",
                ),
            );
        }

        if challenge.device_id.is_some() || transcript.device_id.is_some() {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    "RegisterRoot unexpectedly device-bound",
                ),
            );
        }

        let challenge_body = challenge.operation_body_hash.as_ref().ok_or(
            NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                "RegisterRoot challenge missing operation body hash",
            ),
        )?;

        if transcript.operation_body_hash != challenge_body {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    "operation body hash mismatch",
                ),
            );
        }

        if transcript.challenge_issued_at_ms != challenge.issued_at_ms
            || transcript.challenge_expires_at_ms != challenge.expires_at_ms
        {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    "challenge time binding mismatch",
                ),
            );
        }

        if transcript.requested_scopes.len() != challenge.requested_scopes.len()
            || !transcript
                .requested_scopes
                .iter()
                .zip(challenge.requested_scopes.iter())
                .all(|(proof_scope, challenge_scope)| *proof_scope == challenge_scope.as_str())
        {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    "requested scope binding mismatch",
                ),
            );
        }

        let expected_challenge_hash = passport_challenge_v1_transcript_b3_hex(
            &challenge.signing_payload(),
        )
        .map_err(|_| {
            NativePassportServerRootRegistrationCoordinatorError::ChallengeTranscriptHashInvalid
        })?;

        if transcript.challenge_transcript_hash.as_str() != expected_challenge_hash {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    "canonical challenge transcript hash mismatch",
                ),
            );
        }

        Ok(())
    }

    async fn recover_pending(
        &mut self,
        pending: Vec<NativePassportRootRegistrationTxnIntentV1>,
    ) -> Result<(), NativePassportServerRootRegistrationCoordinatorError> {
        for intent in pending {
            self.apply_durable_intent(&intent).await?;
        }

        Ok(())
    }

    /// Redo one already durable transaction decision.
    ///
    /// Recovery intentionally uses `accepted_at_ms` from the immutable intent,
    /// not current wall-clock time. The transaction was admitted while the
    /// challenge was valid; a later restart must finish that same decision.
    async fn apply_durable_intent(
        &mut self,
        intent: &NativePassportRootRegistrationTxnIntentV1,
    ) -> Result<
        NativePassportServerRootRegistrationOutcomeV1,
        NativePassportServerRootRegistrationCoordinatorError,
    > {
        self.validate_intent_challenge_binding(intent)?;

        let root_public_key =
            Ed25519PublicKeyHex::parse(intent.root_public_key.clone()).map_err(|_| {
                NativePassportServerRootRegistrationCoordinatorError::RecoveryIntentInvalid(
                    "invalid root public key",
                )
            })?;

        let challenge_transcript_hash = B3DigestHex::parse(
            "challenge_transcript_hash",
            intent.challenge_transcript_hash.clone(),
        )
        .map_err(|_| {
            NativePassportServerRootRegistrationCoordinatorError::RecoveryIntentInvalid(
                "invalid challenge transcript hash",
            )
        })?;

        let signed_payload = NativeProofSignedPayloadHex::parse(
            "root_registration_proof_signed_payload_hex",
            intent.proof_signed_payload_hex.clone(),
        )
        .map_err(|_| {
            NativePassportServerRootRegistrationCoordinatorError::RecoveryIntentInvalid(
                "invalid root proof signature encoding",
            )
        })?;

        let passport_id = intent.challenge.passport_id.as_ref().ok_or(
            NativePassportServerRootRegistrationCoordinatorError::RecoveryIntentInvalid(
                "pending RegisterRoot missing Passport binding",
            ),
        )?;

        let operation_body_hash = intent.challenge.operation_body_hash.as_ref().ok_or(
            NativePassportServerRootRegistrationCoordinatorError::RecoveryIntentInvalid(
                "pending RegisterRoot missing operation body hash",
            ),
        )?;

        let requested_scopes: Vec<&str> = intent
            .challenge
            .requested_scopes
            .iter()
            .map(|scope| scope.as_str())
            .collect();

        let transcript = RootRegistrationProofTranscriptV1 {
            challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,

            challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,

            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,

            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,

            challenge_id: &intent.challenge.challenge_id,

            network_id: intent.challenge.network_id.as_str(),

            environment: intent.challenge.environment.as_str(),

            audience: intent.challenge.audience.as_str(),

            passport_id,

            root_public_key: &root_public_key,

            root_key_epoch: intent.root_key_epoch,

            device_id: None,

            operation_body_hash,

            challenge_transcript_hash: &challenge_transcript_hash,

            requested_scopes: &requested_scopes,

            challenge_issued_at_ms: intent.challenge.issued_at_ms,

            challenge_expires_at_ms: intent.challenge.expires_at_ms,

            proof_created_at_ms: intent.proof_created_at_ms,
        };

        /*
         * Re-verify the journal's root proof before allowing challenge
         * consumption. A syntactically valid but cryptographically corrupted
         * redo file must never burn a challenge.
         */
        self.registry_runtime
            .validate_root_registration_without_mutation(
                &transcript,
                &signed_payload,
                intent.accepted_at_ms,
            )?;

        match self
            .challenge_runtime
            .consume_durable(&intent.challenge, intent.accepted_at_ms)
            .await
        {
            Ok(()) => {}

            Err(NativePassportServerChallengeRuntimeError::AlreadyConsumed) => {
                /*
                 * This is the expected restart case after the transaction had
                 * durably consumed the exact challenge but crashed before the
                 * idempotent root write or journal completion.
                 */
            }

            Err(error) => {
                return Err(NativePassportServerRootRegistrationCoordinatorError::Challenge(error));
            }
        }

        let outcome = self.registry_runtime.register_root_from_verified_proof(
            &transcript,
            &signed_payload,
            intent.accepted_at_ms,
        )?;

        self.txn_store.complete(intent)?;

        Ok(outcome)
    }

    fn validate_intent_challenge_binding(
        &self,
        intent: &NativePassportRootRegistrationTxnIntentV1,
    ) -> Result<(), NativePassportServerRootRegistrationCoordinatorError> {
        if intent.challenge.purpose != PassportChallengePurposeV1::RegisterRoot {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::RecoveryIntentInvalid(
                    "pending transaction purpose is not RegisterRoot",
                ),
            );
        }

        let expected_hash = passport_challenge_v1_transcript_b3_hex(
            &intent.challenge.signing_payload(),
        )
        .map_err(|_| {
            NativePassportServerRootRegistrationCoordinatorError::ChallengeTranscriptHashInvalid
        })?;

        if expected_hash != intent.challenge_transcript_hash {
            return Err(
                NativePassportServerRootRegistrationCoordinatorError::RecoveryIntentInvalid(
                    "pending challenge transcript hash mismatch",
                ),
            );
        }

        Ok(())
    }
}

#[cfg(all(test, feature = "dev-kms"))]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use ron_auth::native_passport::{
        passport_challenge_v1_transcript_b3_hex, RootRegistrationProofTranscriptV1,
    };

    use ron_proto::{
        B3DigestHex, Ed25519PublicKeyHex, NativePassportContextLabelV1, NativePassportScopeV1,
        PassportChallengePurposeV1, PassportIdV1,
    };

    use crate::{
        kms::client::DevKms,
        native::{
            derive_native_recovery_public_identity_v1,
            sign_native_recovery_root_registration_proof_v1, NativeSecretBytes,
        },
    };

    use super::super::{
        server_challenge_issuer::NativePassportServerChallengeIssueRequestV1,
        server_challenge_runtime::NativePassportServerChallengeRuntimeError,
        server_registry_runtime::NativePassportServerRootRegistrationDispositionV1,
    };

    use super::*;

    const NOW_MS: u64 = 1_000_000;
    const PROOF_AT_MS: u64 = 1_000_010;
    const ACCEPTED_AT_MS: u64 = 1_000_020;
    const CHALLENGE_TTL_MS: u64 = 60_000;
    const REPLAY_RETENTION_MS: u64 = 120_000;

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
                    "svc-passport-root-coordinator-{label}-{}-{stamp}",
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
        recovery_factor: NativeSecretBytes,
        passport_id: PassportIdV1,
        root_public_key: Ed25519PublicKeyHex,
        operation_body_hash: B3DigestHex,
    }

    impl Fixture {
        fn new(byte: u8) -> Self {
            let recovery_factor = NativeSecretBytes::new(vec![byte; 32]).expect("recovery factor");

            let identity = derive_native_recovery_public_identity_v1(&recovery_factor)
                .expect("public root identity");

            Self {
                recovery_factor,

                passport_id: PassportIdV1::parse(identity.passport_id.as_str())
                    .expect("Passport ID"),

                root_public_key: Ed25519PublicKeyHex::parse(identity.root_public_key.as_str())
                    .expect("root public key"),

                operation_body_hash: B3DigestHex::parse("operation_body_hash", "44".repeat(32))
                    .expect("operation body hash"),
            }
        }

        fn request(&self) -> NativePassportServerChallengeIssueRequestV1 {
            NativePassportServerChallengeIssueRequestV1 {
                purpose: PassportChallengePurposeV1::RegisterRoot,

                requested_scopes: vec![scope("identity.read"), scope("profile.read")],

                passport_id: Some(self.passport_id.clone()),

                device_id: None,

                operation_body_hash: Some(self.operation_body_hash.clone()),
            }
        }
    }

    fn context(value: &str) -> NativePassportContextLabelV1 {
        NativePassportContextLabelV1::parse(value).expect("context")
    }

    fn scope(value: &str) -> NativePassportScopeV1 {
        NativePassportScopeV1::parse(value).expect("scope")
    }

    async fn open_coordinator<'a>(
        directory: &TestDirectory,
        kms: &'a DevKms,
    ) -> Result<
        NativePassportServerRootRegistrationCoordinator<'a>,
        NativePassportServerRootRegistrationCoordinatorError,
    > {
        NativePassportServerRootRegistrationCoordinator::open(
            directory.challenge_root(),
            directory.registry_root(),
            directory.transaction_root(),
            kms,
            context("rustyonions-devnet"),
            context("private-beta"),
            context("svc-passport"),
            context("svc-passport"),
            CHALLENGE_TTL_MS,
            REPLAY_RETENTION_MS,
            0,
        )
        .await
    }

    async fn issue(
        coordinator: &mut NativePassportServerRootRegistrationCoordinator<'_>,
        fixture: &Fixture,
    ) -> PassportChallengeV1 {
        coordinator
            .challenge_runtime
            .issue_durable(fixture.request(), NOW_MS)
            .await
            .expect("issue challenge")
    }

    fn challenge_hash(challenge: &PassportChallengeV1) -> B3DigestHex {
        let hash = passport_challenge_v1_transcript_b3_hex(&challenge.signing_payload())
            .expect("challenge transcript hash");

        B3DigestHex::parse("challenge_transcript_hash", hash)
            .expect("typed challenge transcript hash")
    }

    fn root_transcript<'a>(
        fixture: &'a Fixture,
        challenge: &'a PassportChallengeV1,
        challenge_hash: &'a B3DigestHex,
        scopes: &'a [&'a str],
    ) -> RootRegistrationProofTranscriptV1<'a> {
        RootRegistrationProofTranscriptV1 {
            challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,

            challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,

            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,

            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,

            challenge_id: &challenge.challenge_id,

            network_id: challenge.network_id.as_str(),

            environment: challenge.environment.as_str(),

            audience: challenge.audience.as_str(),

            passport_id: challenge.passport_id.as_ref().expect("Passport binding"),

            root_public_key: &fixture.root_public_key,

            root_key_epoch: 0,

            device_id: None,

            operation_body_hash: challenge
                .operation_body_hash
                .as_ref()
                .expect("operation body hash"),

            challenge_transcript_hash: challenge_hash,

            requested_scopes: scopes,

            challenge_issued_at_ms: challenge.issued_at_ms,

            challenge_expires_at_ms: challenge.expires_at_ms,

            proof_created_at_ms: PROOF_AT_MS,
        }
    }

    fn proof(
        fixture: &Fixture,
        transcript: &RootRegistrationProofTranscriptV1<'_>,
    ) -> NativeProofSignedPayloadHex {
        sign_native_recovery_root_registration_proof_v1(&fixture.recovery_factor, transcript)
            .expect("root registration proof")
            .signed_payload_hex
    }

    fn scope_refs(challenge: &PassportChallengeV1) -> Vec<&str> {
        challenge
            .requested_scopes
            .iter()
            .map(|scope| scope.as_str())
            .collect()
    }

    #[tokio::test]
    async fn normal_transaction_consumes_challenge_registers_root_and_clears_intent() {
        let directory = TestDirectory::new("normal");

        let kms = DevKms::new();
        let fixture = Fixture::new(0x71);

        let mut coordinator = open_coordinator(&directory, &kms)
            .await
            .expect("open coordinator");

        let challenge = issue(&mut coordinator, &fixture).await;

        let hash = challenge_hash(&challenge);
        let scopes = scope_refs(&challenge);

        let transcript = root_transcript(&fixture, &challenge, &hash, &scopes);

        let signed = proof(&fixture, &transcript);

        let outcome = coordinator
            .register_root_recoverably(&challenge, &transcript, &signed, ACCEPTED_AT_MS)
            .await
            .expect("recoverable root registration");

        assert_eq!(
            outcome.disposition,
            NativePassportServerRootRegistrationDispositionV1::Registered,
        );

        assert!(coordinator
            .txn_store
            .load_pending()
            .expect("pending intents")
            .is_empty());

        assert_eq!(
            coordinator
                .challenge_runtime
                .consume_durable(&challenge, ACCEPTED_AT_MS + 1,)
                .await,
            Err(NativePassportServerChallengeRuntimeError::AlreadyConsumed)
        );

        let repeated = coordinator
            .registry_runtime
            .register_root_from_verified_proof(&transcript, &signed, ACCEPTED_AT_MS + 1)
            .expect("idempotent registry state");

        assert_eq!(
            repeated.disposition,
            NativePassportServerRootRegistrationDispositionV1::AlreadyRegistered,
        );
    }

    #[tokio::test]
    async fn restart_recovers_crash_after_durable_intent() {
        let directory = TestDirectory::new("after-intent");

        let kms = DevKms::new();
        let fixture = Fixture::new(0x72);

        let mut first = open_coordinator(&directory, &kms)
            .await
            .expect("open first coordinator");

        let challenge = issue(&mut first, &fixture).await;
        let hash = challenge_hash(&challenge);
        let scopes = scope_refs(&challenge);

        let transcript = root_transcript(&fixture, &challenge, &hash, &scopes);

        let signed = proof(&fixture, &transcript);

        first
            .validate_new_transaction(&challenge, &transcript, &signed, ACCEPTED_AT_MS)
            .await
            .expect("preflight");

        let intent = first
            .build_intent(&challenge, &transcript, &signed, ACCEPTED_AT_MS)
            .expect("intent");

        first.txn_store.prepare(&intent).expect("durable intent");

        drop(first);

        let mut restarted = open_coordinator(&directory, &kms)
            .await
            .expect("restart recovery");

        assert!(restarted
            .txn_store
            .load_pending()
            .expect("pending")
            .is_empty());

        assert_eq!(
            restarted
                .challenge_runtime
                .consume_durable(&challenge, ACCEPTED_AT_MS + 1,)
                .await,
            Err(NativePassportServerChallengeRuntimeError::AlreadyConsumed)
        );
    }

    #[tokio::test]
    async fn restart_recovers_crash_after_challenge_consumption() {
        let directory = TestDirectory::new("after-consume");

        let kms = DevKms::new();
        let fixture = Fixture::new(0x73);

        let mut first = open_coordinator(&directory, &kms)
            .await
            .expect("open first coordinator");

        let challenge = issue(&mut first, &fixture).await;
        let hash = challenge_hash(&challenge);
        let scopes = scope_refs(&challenge);

        let transcript = root_transcript(&fixture, &challenge, &hash, &scopes);

        let signed = proof(&fixture, &transcript);

        first
            .validate_new_transaction(&challenge, &transcript, &signed, ACCEPTED_AT_MS)
            .await
            .expect("preflight");

        let intent = first
            .build_intent(&challenge, &transcript, &signed, ACCEPTED_AT_MS)
            .expect("intent");

        first.txn_store.prepare(&intent).expect("prepare");

        first
            .challenge_runtime
            .consume_durable(&challenge, ACCEPTED_AT_MS)
            .await
            .expect("consume before simulated crash");

        drop(first);

        let restarted = open_coordinator(&directory, &kms)
            .await
            .expect("restart recovery");

        assert!(restarted
            .txn_store
            .load_pending()
            .expect("pending")
            .is_empty());
    }

    #[tokio::test]
    async fn restart_recovers_crash_after_root_registration_before_intent_completion() {
        let directory = TestDirectory::new("after-root");

        let kms = DevKms::new();
        let fixture = Fixture::new(0x74);

        let mut first = open_coordinator(&directory, &kms)
            .await
            .expect("open first coordinator");

        let challenge = issue(&mut first, &fixture).await;
        let hash = challenge_hash(&challenge);
        let scopes = scope_refs(&challenge);

        let transcript = root_transcript(&fixture, &challenge, &hash, &scopes);

        let signed = proof(&fixture, &transcript);

        first
            .validate_new_transaction(&challenge, &transcript, &signed, ACCEPTED_AT_MS)
            .await
            .expect("preflight");

        let intent = first
            .build_intent(&challenge, &transcript, &signed, ACCEPTED_AT_MS)
            .expect("intent");

        first.txn_store.prepare(&intent).expect("prepare");

        first
            .challenge_runtime
            .consume_durable(&challenge, ACCEPTED_AT_MS)
            .await
            .expect("consume");

        first
            .registry_runtime
            .register_root_from_verified_proof(&transcript, &signed, ACCEPTED_AT_MS)
            .expect("root register before simulated crash");

        drop(first);

        let restarted = open_coordinator(&directory, &kms)
            .await
            .expect("restart recovery");

        assert!(restarted
            .txn_store
            .load_pending()
            .expect("pending")
            .is_empty());
    }

    #[tokio::test]
    async fn proof_binding_mismatch_never_creates_redo_intent() {
        let directory = TestDirectory::new("binding-mismatch");

        let kms = DevKms::new();
        let fixture = Fixture::new(0x75);

        let mut coordinator = open_coordinator(&directory, &kms)
            .await
            .expect("open coordinator");

        let challenge = issue(&mut coordinator, &fixture).await;

        let hash = challenge_hash(&challenge);
        let scopes = scope_refs(&challenge);

        let wrong_body = B3DigestHex::parse("operation_body_hash", "99".repeat(32))
            .expect("wrong operation body hash");

        let transcript = RootRegistrationProofTranscriptV1 {
            challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,

            challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,

            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,

            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,

            challenge_id: &challenge.challenge_id,

            network_id: challenge.network_id.as_str(),

            environment: challenge.environment.as_str(),

            audience: challenge.audience.as_str(),

            passport_id: challenge.passport_id.as_ref().expect("Passport"),

            root_public_key: &fixture.root_public_key,

            root_key_epoch: 0,

            device_id: None,

            operation_body_hash: &wrong_body,

            challenge_transcript_hash: &hash,

            requested_scopes: &scopes,

            challenge_issued_at_ms: challenge.issued_at_ms,

            challenge_expires_at_ms: challenge.expires_at_ms,

            proof_created_at_ms: PROOF_AT_MS,
        };

        let signed = proof(&fixture, &transcript);

        assert!(matches!(
            coordinator
                .register_root_recoverably(&challenge, &transcript, &signed, ACCEPTED_AT_MS,)
                .await,
            Err(
                NativePassportServerRootRegistrationCoordinatorError::ChallengeProofBindingMismatch(
                    _
                )
            )
        ));

        assert!(
            coordinator
                .txn_store
                .load_pending()
                .expect("pending")
                .is_empty(),
            "binding failure must occur before redo intent",
        );

        coordinator
            .challenge_runtime
            .validate_consumption_without_mutation(&challenge, ACCEPTED_AT_MS)
            .await
            .expect("binding rejection must leave challenge Issued");
    }

    #[test]
    fn private_coordinator_has_no_route_capability_username_or_value_authority() {
        let source = include_str!("server_root_registration_coordinator.rs");

        let implementation = source
            .split("\n#[cfg(all(test")
            .next()
            .expect("implementation");

        for forbidden in [
            "Router::",
            ".route(",
            "issue_capability(",
            "claim_username(",
            "wallet.spend(",
            "ledger.write(",
            "root_private_key:",
            "device_private_key:",
            "recovery_phrase:",
            "raw_pin:",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "coordinator gained forbidden authority pattern {forbidden}",
            );
        }
    }
}
