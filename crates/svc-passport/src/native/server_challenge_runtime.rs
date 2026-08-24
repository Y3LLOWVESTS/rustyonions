//! RO:WHAT — Private durable lifecycle runtime for svc-passport Native Passport V1 server challenges.
//! RO:WHY — CN-4/M1 must never return a usable signed challenge before its exact replay state is durable, and restart must cryptographically revalidate stored signatures rather than trust well-formed JSON.
//! RO:INTERACTS — server_challenge_issuer, server_challenge_store, KmsClient historical-KID verification, ron-auth canonical PassportChallengeV1 transcript, and future one-time consumption/root-registration composition.
//! RO:INVARIANTS — issuer output is persisted before successful return; complete presented-challenge equality is required for lifecycle mutation; the non-mutating consumption preflight proves exact durable binding, lifecycle eligibility, trusted time, and historical-KID signature before a higher-level transaction may journal intent; consumed/expired/cancelled states are durable and monotonic; stale CAS consumers cannot double-consume; restart revalidates historical-KID signatures; state remains hard-bounded; no route, root-registration, capability, username, wallet, or ledger authority is added here.
//! RO:METRICS — none yet; future mounted service orchestration owns bounded operational counters.
//! RO:CONFIG — caller supplies the service-owned store directory, trusted network/environment/audience/service context, protocol-bounded challenge TTL, and non-zero terminal replay-retention duration; retention is service policy and is never challenge-controlled.
//! RO:SECURITY — stored material is public signed challenge data only; KMS private keys never leave the KMS; historical KID verification fails closed; no PIN, recovery factor, root/device private key, capability issuance, or value mutation.
//! RO:TEST — focused unit tests prove durable-before-return issuance, restart verification, historical-KID verification after rotation, signature-corruption rejection, stale-writer rejection, and authority isolation.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use ron_auth::native_passport::canonical_passport_challenge_v1_transcript;
use ron_proto::{NativePassportContextLabelV1, PassportChallengeV1};
use thiserror::Error;

use crate::kms::client::KmsClient;

use super::server_challenge_issuer::{
    NativePassportServerChallengeIssueRequestV1, NativePassportServerChallengeIssuer,
    NativePassportServerChallengeIssuerError,
};
use super::server_challenge_store::{
    LoadedNativePassportServerChallengeStateV1, NativePassportServerChallengeRecordV1,
    NativePassportServerChallengeSnapshotStore, NativePassportServerChallengeStatusV1,
    NativePassportServerChallengeStoreError, MAX_CHALLENGE_RECORDS,
};

/// Failure before a durable challenge may be returned to a future caller.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum NativePassportServerChallengeRuntimeError {
    #[error("Native Passport challenge issuer failed")]
    Issuer(#[from] NativePassportServerChallengeIssuerError),

    #[error("Native Passport challenge store unavailable: {0}")]
    StoreUnavailable(&'static str),

    #[error("Native Passport challenge store corrupt: {0}")]
    StoreCorrupt(&'static str),

    #[error("Native Passport challenge store changed concurrently")]
    ConcurrentChange,

    #[error("Native Passport stored challenge transcript is invalid")]
    StoredChallengeTranscriptInvalid,

    #[error("Native Passport stored challenge KMS verification is unavailable")]
    StoredChallengeVerificationUnavailable,

    #[error("Native Passport stored challenge signature is invalid")]
    StoredChallengeSignatureInvalid,

    #[error("Native Passport challenge state reached its bounded capacity")]
    ChallengeStateFull,

    #[error("Native Passport challenge ID collided with durable state")]
    ChallengeIdCollision,

    #[error("Native Passport challenge is unknown")]
    UnknownChallenge,

    #[error("Native Passport presented challenge does not exactly match durable state")]
    ChallengeBindingMismatch,

    #[error("Native Passport challenge was already consumed")]
    AlreadyConsumed,

    #[error("Native Passport challenge is expired")]
    ChallengeExpired,

    #[error("Native Passport challenge is cancelled")]
    ChallengeCancelled,

    #[error("Native Passport challenge is not yet valid")]
    ChallengeNotYetValid,

    #[error("Native Passport trusted time is invalid")]
    InvalidTrustedTime,

    #[error("Native Passport terminal replay retention must be non-zero")]
    InvalidReplayRetention,

    #[error("Native Passport challenge lifecycle time overflowed")]
    ChallengeTimeOverflow,

    #[error("Native Passport challenge generation overflowed")]
    ChallengeGenerationOverflow,
}

impl From<NativePassportServerChallengeStoreError> for NativePassportServerChallengeRuntimeError {
    fn from(error: NativePassportServerChallengeStoreError) -> Self {
        match error {
            NativePassportServerChallengeStoreError::Unavailable(reason) => {
                Self::StoreUnavailable(reason)
            }
            NativePassportServerChallengeStoreError::Corrupt(reason) => Self::StoreCorrupt(reason),
            NativePassportServerChallengeStoreError::ConcurrentChange => Self::ConcurrentChange,
        }
    }
}

/// Private service lifecycle owner above the raw durable challenge store.
///
/// The KMS reference is retained so restart can verify historical service KIDs.
/// No mutex is held across any asynchronous KMS operation.
pub(super) struct NativePassportServerChallengeRuntime<'a> {
    kms: &'a dyn KmsClient,
    issuer: NativePassportServerChallengeIssuer<'a>,
    store: NativePassportServerChallengeSnapshotStore,
    state: LoadedNativePassportServerChallengeStateV1,
    store_root: PathBuf,
    trusted_network_id: NativePassportContextLabelV1,
    trusted_environment: NativePassportContextLabelV1,
    trusted_audience: NativePassportContextLabelV1,
    trusted_issuing_service_id: NativePassportContextLabelV1,
    replay_retention_ms: u64,
}

impl<'a> NativePassportServerChallengeRuntime<'a> {
    /// Open durable challenge state and cryptographically revalidate every
    /// stored service signature before making the runtime available.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn open(
        root: impl AsRef<Path>,
        kms: &'a dyn KmsClient,
        network_id: NativePassportContextLabelV1,
        environment: NativePassportContextLabelV1,
        audience: NativePassportContextLabelV1,
        issuing_service_id: NativePassportContextLabelV1,
        challenge_ttl_ms: u64,
        replay_retention_ms: u64,
    ) -> Result<Self, NativePassportServerChallengeRuntimeError> {
        if replay_retention_ms == 0 {
            return Err(NativePassportServerChallengeRuntimeError::InvalidReplayRetention);
        }

        let store_root = root.as_ref().to_path_buf();

        let trusted_network_id = network_id.clone();
        let trusted_environment = environment.clone();
        let trusted_audience = audience.clone();
        let trusted_issuing_service_id = issuing_service_id.clone();

        let issuer = NativePassportServerChallengeIssuer::new(
            kms,
            network_id.clone(),
            environment.clone(),
            audience.clone(),
            issuing_service_id.clone(),
            challenge_ttl_ms,
        )?;

        let (store, state) = NativePassportServerChallengeSnapshotStore::open(
            &store_root,
            network_id,
            environment,
            audience,
            issuing_service_id,
        )?;

        /*
         * Storage validation proves structure/context/lifecycle consistency.
         * It deliberately does not own service cryptography. Reconstruct the
         * canonical ron-auth transcript and ask the KMS to verify the exact
         * historical KID carried by each signed challenge.
         *
         * Time validity is intentionally not re-applied here: an expired or
         * consumed replay record may remain durably retained, but its original
         * service signature must still be authentic.
         */
        verify_loaded_state(kms, &state).await?;

        Ok(Self {
            kms,
            issuer,
            store,
            state,
            store_root,
            trusted_network_id,
            trusted_environment,
            trusted_audience,
            trusted_issuing_service_id,
            replay_retention_ms,
        })
    }

    /// Issue one real signed challenge and make its Issued state durable before
    /// successful return.
    ///
    /// A KMS signature may be computed before filesystem CAS, but it never
    /// escapes this method when durable publication loses a concurrent race.
    pub(super) async fn issue_durable(
        &mut self,
        request: NativePassportServerChallengeIssueRequestV1,
        now_ms: u64,
    ) -> Result<PassportChallengeV1, NativePassportServerChallengeRuntimeError> {
        /*
         * Reject capacity before asking KMS to create another signature.
         * B2 will add bounded terminal-state retention/pruning before this
         * ceiling is reached in normal operation.
         */
        let mut next_snapshot = self.compacted_snapshot_for_issue(now_ms)?;

        if next_snapshot.challenges.len() >= MAX_CHALLENGE_RECORDS {
            return Err(NativePassportServerChallengeRuntimeError::ChallengeStateFull);
        }

        let challenge = self.issuer.issue(request, now_ms).await?;

        /*
         * Defense in depth: issuer already strict-cross-verifies using the
         * preselected public identity. Verify through the historical-KID KMS
         * interface too, because this is the exact verification path restart
         * will use after key rotation.
         */
        verify_stored_challenge(self.kms, &challenge).await?;

        let insertion_index = match next_snapshot.challenges.binary_search_by(|record| {
            record
                .challenge
                .challenge_id
                .as_str()
                .cmp(challenge.challenge_id.as_str())
        }) {
            Ok(_) => {
                return Err(NativePassportServerChallengeRuntimeError::ChallengeIdCollision);
            }
            Err(index) => index,
        };

        next_snapshot.challenges.insert(
            insertion_index,
            NativePassportServerChallengeRecordV1 {
                challenge: challenge.clone(),
                status: NativePassportServerChallengeStatusV1::Issued,
                status_changed_at_ms: challenge.issued_at_ms,
            },
        );

        let next_generation = self
            .state
            .generation
            .checked_add(1)
            .ok_or(NativePassportServerChallengeRuntimeError::ChallengeGenerationOverflow)?;

        /*
         * Critical ordering:
         *
         *   signed + verified
         *       -> durable CAS publication
         *       -> in-memory state update
         *       -> return challenge
         *
         * Persist failure therefore never returns an untracked challenge.
         */
        self.store.persist(
            self.state.generation,
            &self.state.snapshot,
            next_generation,
            &next_snapshot,
        )?;

        self.state = LoadedNativePassportServerChallengeStateV1 {
            generation: next_generation,
            snapshot: next_snapshot,
        };

        Ok(challenge)
    }

    fn compacted_snapshot_for_issue(
        &self,
        now_ms: u64,
    ) -> Result<
        super::server_challenge_store::NativePassportServerChallengeSnapshotV1,
        NativePassportServerChallengeRuntimeError,
    > {
        validate_trusted_now(now_ms)?;

        let mut next_snapshot = self.state.snapshot.clone();

        /*
         * An Issued challenge whose protocol lifetime has elapsed becomes an
         * Expired replay tombstone at the deterministic first millisecond
         * after expires_at_ms. This matches the store invariant requiring an
         * Expired status-change timestamp strictly greater than expiry.
         */
        for record in &mut next_snapshot.challenges {
            if record.status == NativePassportServerChallengeStatusV1::Issued
                && now_ms > record.challenge.expires_at_ms
            {
                record.status = NativePassportServerChallengeStatusV1::Expired;

                record.status_changed_at_ms = logical_expired_at_ms(&record.challenge)?;
            }
        }

        let replay_retention_ms = self.replay_retention_ms;

        next_snapshot.challenges.retain(|record| {
            let terminal = matches!(
                record.status,
                NativePassportServerChallengeStatusV1::Consumed
                    | NativePassportServerChallengeStatusV1::Expired
                    | NativePassportServerChallengeStatusV1::Cancelled
            );

            if !terminal {
                return true;
            }

            let Some(retention_deadline_ms) =
                record.status_changed_at_ms.checked_add(replay_retention_ms)
            else {
                /*
                 * Overflow cannot prove the retention period elapsed. Retain
                 * the replay tombstone rather than risk accepting old state.
                 */
                return true;
            };

            now_ms <= retention_deadline_ms
        });

        Ok(next_snapshot)
    }

    /// Prove that one exact durable challenge is currently eligible for
    /// consumption without changing lifecycle state.
    ///
    /// A higher-level crash-recovery coordinator uses this before committing
    /// its write-ahead intent. In particular, observing an expired challenge
    /// here does not persist Expired; only the lifecycle mutation path owns
    /// state transitions.
    pub(super) async fn validate_consumption_without_mutation(
        &self,
        presented: &PassportChallengeV1,
        now_ms: u64,
    ) -> Result<(), NativePassportServerChallengeRuntimeError> {
        validate_trusted_now(now_ms)?;

        let (_, record) = self.bound_record(presented)?;

        match record.status {
            NativePassportServerChallengeStatusV1::Consumed => {
                return Err(NativePassportServerChallengeRuntimeError::AlreadyConsumed);
            }
            NativePassportServerChallengeStatusV1::Expired => {
                return Err(NativePassportServerChallengeRuntimeError::ChallengeExpired);
            }
            NativePassportServerChallengeStatusV1::Cancelled => {
                return Err(NativePassportServerChallengeRuntimeError::ChallengeCancelled);
            }
            NativePassportServerChallengeStatusV1::Issued => {}
        }

        if now_ms < record.challenge.issued_at_ms {
            return Err(NativePassportServerChallengeRuntimeError::ChallengeNotYetValid);
        }

        if now_ms > record.challenge.expires_at_ms {
            return Err(NativePassportServerChallengeRuntimeError::ChallengeExpired);
        }

        /*
         * `bound_record` proved complete equality with durable state.
         * Reconstruct and verify the service-signed canonical transcript using
         * the challenge-bound historical KID before transaction admission.
         */
        verify_stored_challenge(self.kms, presented).await
    }

    /// Consume one exact issued challenge durably.
    ///
    /// A concurrent generation loss reloads durable state exactly once. If
    /// another consumer already won, the loser resolves as AlreadyConsumed
    /// instead of treating the stale in-memory generation as authority.
    pub(super) async fn consume_durable(
        &mut self,
        presented: &PassportChallengeV1,
        now_ms: u64,
    ) -> Result<(), NativePassportServerChallengeRuntimeError> {
        match self.consume_once(presented, now_ms).await {
            Err(NativePassportServerChallengeRuntimeError::ConcurrentChange) => {
                self.reload_after_concurrent_change().await?;
                self.consume_once(presented, now_ms).await
            }
            result => result,
        }
    }

    /// Cancel one exact issued challenge durably.
    pub(super) async fn cancel_durable(
        &mut self,
        presented: &PassportChallengeV1,
        now_ms: u64,
    ) -> Result<(), NativePassportServerChallengeRuntimeError> {
        match self.cancel_once(presented, now_ms).await {
            Err(NativePassportServerChallengeRuntimeError::ConcurrentChange) => {
                self.reload_after_concurrent_change().await?;
                self.cancel_once(presented, now_ms).await
            }
            result => result,
        }
    }

    async fn consume_once(
        &mut self,
        presented: &PassportChallengeV1,
        now_ms: u64,
    ) -> Result<(), NativePassportServerChallengeRuntimeError> {
        validate_trusted_now(now_ms)?;

        let (record_index, record) = self.bound_record(presented)?;

        match record.status {
            NativePassportServerChallengeStatusV1::Consumed => {
                return Err(NativePassportServerChallengeRuntimeError::AlreadyConsumed);
            }
            NativePassportServerChallengeStatusV1::Expired => {
                return Err(NativePassportServerChallengeRuntimeError::ChallengeExpired);
            }
            NativePassportServerChallengeStatusV1::Cancelled => {
                return Err(NativePassportServerChallengeRuntimeError::ChallengeCancelled);
            }
            NativePassportServerChallengeStatusV1::Issued => {}
        }

        if now_ms < record.challenge.issued_at_ms {
            return Err(NativePassportServerChallengeRuntimeError::ChallengeNotYetValid);
        }

        if now_ms > record.challenge.expires_at_ms {
            let expired_at_ms = logical_expired_at_ms(&record.challenge)?;

            self.persist_status_transition(
                record_index,
                NativePassportServerChallengeStatusV1::Expired,
                expired_at_ms,
            )?;

            return Err(NativePassportServerChallengeRuntimeError::ChallengeExpired);
        }

        /*
         * The stored challenge was cryptographically checked when this runtime
         * opened, but reverify the exact presented object immediately before
         * the irreversible one-time replay-state mutation.
         */
        verify_stored_challenge(self.kms, presented).await?;

        self.persist_status_transition(
            record_index,
            NativePassportServerChallengeStatusV1::Consumed,
            now_ms,
        )
    }

    async fn cancel_once(
        &mut self,
        presented: &PassportChallengeV1,
        now_ms: u64,
    ) -> Result<(), NativePassportServerChallengeRuntimeError> {
        validate_trusted_now(now_ms)?;

        let (record_index, record) = self.bound_record(presented)?;

        match record.status {
            NativePassportServerChallengeStatusV1::Consumed => {
                return Err(NativePassportServerChallengeRuntimeError::AlreadyConsumed);
            }
            NativePassportServerChallengeStatusV1::Expired => {
                return Err(NativePassportServerChallengeRuntimeError::ChallengeExpired);
            }
            NativePassportServerChallengeStatusV1::Cancelled => {
                return Err(NativePassportServerChallengeRuntimeError::ChallengeCancelled);
            }
            NativePassportServerChallengeStatusV1::Issued => {}
        }

        if now_ms < record.challenge.issued_at_ms {
            return Err(NativePassportServerChallengeRuntimeError::ChallengeNotYetValid);
        }

        if now_ms > record.challenge.expires_at_ms {
            let expired_at_ms = logical_expired_at_ms(&record.challenge)?;

            self.persist_status_transition(
                record_index,
                NativePassportServerChallengeStatusV1::Expired,
                expired_at_ms,
            )?;

            return Err(NativePassportServerChallengeRuntimeError::ChallengeExpired);
        }

        verify_stored_challenge(self.kms, presented).await?;

        self.persist_status_transition(
            record_index,
            NativePassportServerChallengeStatusV1::Cancelled,
            now_ms,
        )
    }

    fn bound_record(
        &self,
        presented: &PassportChallengeV1,
    ) -> Result<
        (usize, NativePassportServerChallengeRecordV1),
        NativePassportServerChallengeRuntimeError,
    > {
        let record_index = self
            .state
            .snapshot
            .challenges
            .binary_search_by(|record| {
                record
                    .challenge
                    .challenge_id
                    .as_str()
                    .cmp(presented.challenge_id.as_str())
            })
            .map_err(|_| NativePassportServerChallengeRuntimeError::UnknownChallenge)?;

        let record = self.state.snapshot.challenges[record_index].clone();

        /*
         * Challenge ID alone is never consumption authority. Equality binds
         * nonce, purpose, scopes, Passport/Device IDs, operation body hash,
         * service KID, times, algorithm and service signature.
         */
        if record.challenge != *presented {
            return Err(NativePassportServerChallengeRuntimeError::ChallengeBindingMismatch);
        }

        Ok((record_index, record))
    }

    fn persist_status_transition(
        &mut self,
        record_index: usize,
        status: NativePassportServerChallengeStatusV1,
        status_changed_at_ms: u64,
    ) -> Result<(), NativePassportServerChallengeRuntimeError> {
        let mut next_snapshot = self.state.snapshot.clone();

        let record = next_snapshot
            .challenges
            .get_mut(record_index)
            .ok_or(NativePassportServerChallengeRuntimeError::UnknownChallenge)?;

        record.status = status;
        record.status_changed_at_ms = status_changed_at_ms;

        let next_generation = self
            .state
            .generation
            .checked_add(1)
            .ok_or(NativePassportServerChallengeRuntimeError::ChallengeGenerationOverflow)?;

        self.store.persist(
            self.state.generation,
            &self.state.snapshot,
            next_generation,
            &next_snapshot,
        )?;

        self.state = LoadedNativePassportServerChallengeStateV1 {
            generation: next_generation,
            snapshot: next_snapshot,
        };

        Ok(())
    }

    async fn reload_after_concurrent_change(
        &mut self,
    ) -> Result<(), NativePassportServerChallengeRuntimeError> {
        let (store, state) = NativePassportServerChallengeSnapshotStore::open(
            &self.store_root,
            self.trusted_network_id.clone(),
            self.trusted_environment.clone(),
            self.trusted_audience.clone(),
            self.trusted_issuing_service_id.clone(),
        )?;

        /*
         * A conflict reload gets the same cryptographic treatment as restart:
         * structural JSON/store validity alone is not sufficient authority.
         */
        verify_loaded_state(self.kms, &state).await?;

        self.store = store;
        self.state = state;

        Ok(())
    }
}

fn validate_trusted_now(now_ms: u64) -> Result<(), NativePassportServerChallengeRuntimeError> {
    if now_ms == 0 {
        return Err(NativePassportServerChallengeRuntimeError::InvalidTrustedTime);
    }

    Ok(())
}

fn logical_expired_at_ms(
    challenge: &PassportChallengeV1,
) -> Result<u64, NativePassportServerChallengeRuntimeError> {
    challenge
        .expires_at_ms
        .checked_add(1)
        .ok_or(NativePassportServerChallengeRuntimeError::ChallengeTimeOverflow)
}

async fn verify_loaded_state(
    kms: &dyn KmsClient,
    state: &LoadedNativePassportServerChallengeStateV1,
) -> Result<(), NativePassportServerChallengeRuntimeError> {
    for record in &state.snapshot.challenges {
        verify_stored_challenge(kms, &record.challenge).await?;
    }

    Ok(())
}

async fn verify_stored_challenge(
    kms: &dyn KmsClient,
    challenge: &PassportChallengeV1,
) -> Result<(), NativePassportServerChallengeRuntimeError> {
    let payload = challenge.signing_payload();

    let transcript = canonical_passport_challenge_v1_transcript(&payload)
        .map_err(|_| NativePassportServerChallengeRuntimeError::StoredChallengeTranscriptInvalid)?;

    let verified = kms
        .verify(
            challenge.service_key_id.as_str(),
            &transcript,
            challenge.service_signature.as_bytes(),
        )
        .await
        .map_err(|_| {
            NativePassportServerChallengeRuntimeError::StoredChallengeVerificationUnavailable
        })?;

    if !verified {
        return Err(NativePassportServerChallengeRuntimeError::StoredChallengeSignatureInvalid);
    }

    Ok(())
}

#[cfg(all(test, feature = "dev-kms"))]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use ron_proto::{
        B3DigestHex, Ed25519SignatureV1, NativePassportScopeV1, PassportChallengePurposeV1,
        PassportIdV1,
    };
    use serde_json::json;

    use crate::kms::client::{DevKms, KmsClient};

    use super::*;

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const NOW_MS: u64 = 1_000_000;
    const CHALLENGE_TTL_MS: u64 = 60_000;
    const REPLAY_RETENTION_MS: u64 = 120_000;

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();

            let path = std::env::temp_dir().join(format!(
                "svc-passport-native-challenge-runtime-{label}-{}-{stamp}",
                std::process::id(),
            ));

            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn context(value: &str) -> NativePassportContextLabelV1 {
        NativePassportContextLabelV1::parse(value).expect("context")
    }

    fn scope(value: &str) -> NativePassportScopeV1 {
        NativePassportScopeV1::parse(value).expect("scope")
    }

    fn request() -> NativePassportServerChallengeIssueRequestV1 {
        NativePassportServerChallengeIssueRequestV1 {
            purpose: PassportChallengePurposeV1::RegisterRoot,
            requested_scopes: vec![scope("identity.read")],
            passport_id: Some(
                PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_A}"))
                    .expect("passport id"),
            ),
            device_id: None,
            operation_body_hash: Some(
                B3DigestHex::parse("operation_body_hash", HEX_D).expect("operation body hash"),
            ),
        }
    }

    async fn runtime<'a>(
        root: &Path,
        kms: &'a dyn KmsClient,
    ) -> Result<NativePassportServerChallengeRuntime<'a>, NativePassportServerChallengeRuntimeError>
    {
        NativePassportServerChallengeRuntime::open(
            root,
            kms,
            context("rustyonions-devnet"),
            context("private-beta"),
            context("svc-passport"),
            context("svc-passport"),
            CHALLENGE_TTL_MS,
            REPLAY_RETENTION_MS,
        )
        .await
    }

    fn latest_snapshot(root: &Path) -> PathBuf {
        let mut snapshots = fs::read_dir(root)
            .expect("read runtime directory")
            .map(|entry| entry.expect("directory entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("challenge-v1-") && name.ends_with(".json")
                    })
            })
            .collect::<Vec<_>>();

        snapshots.sort();

        snapshots.pop().expect("durable challenge snapshot")
    }

    #[tokio::test]
    async fn durable_issue_survives_restart_with_exact_signed_binding() {
        let directory = TestDirectory::new("restart");
        let kms = DevKms::new();

        let mut first = runtime(directory.path(), &kms)
            .await
            .expect("open empty runtime");

        let challenge = first
            .issue_durable(request(), NOW_MS)
            .await
            .expect("durable challenge");

        assert_eq!(first.state.generation, 1);
        assert_eq!(first.state.snapshot.challenges.len(), 1);
        assert_eq!(
            first.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Issued
        );
        assert_eq!(first.state.snapshot.challenges[0].challenge, challenge);

        drop(first);

        let reopened = runtime(directory.path(), &kms)
            .await
            .expect("reopen cryptographically valid state");

        assert_eq!(reopened.state.generation, 1);
        assert_eq!(reopened.state.snapshot.challenges.len(), 1);
        assert_eq!(reopened.state.snapshot.challenges[0].challenge, challenge);
    }

    #[tokio::test]
    async fn historical_kid_signature_revalidates_after_kms_rotation() {
        let directory = TestDirectory::new("rotation");
        let kms = DevKms::new();

        let mut first = runtime(directory.path(), &kms).await.expect("open runtime");

        let challenge = first
            .issue_durable(request(), NOW_MS)
            .await
            .expect("durable v1 challenge");

        assert_eq!(challenge.service_key_id.as_str(), "ed25519/default/v1");

        drop(first);

        let rotated_kid = kms.rotate().await.expect("rotate test KMS");

        assert_eq!(rotated_kid, "ed25519/default/v2");

        let reopened = runtime(directory.path(), &kms)
            .await
            .expect("historical KID remains verifiable");

        assert_eq!(reopened.state.snapshot.challenges.len(), 1);
        assert_eq!(
            reopened.state.snapshot.challenges[0]
                .challenge
                .service_key_id
                .as_str(),
            "ed25519/default/v1"
        );
    }

    #[tokio::test]
    async fn tampered_service_signature_fails_closed_on_restart() {
        let directory = TestDirectory::new("tamper");
        let kms = DevKms::new();

        let mut first = runtime(directory.path(), &kms).await.expect("open runtime");

        first
            .issue_durable(request(), NOW_MS)
            .await
            .expect("durable challenge");

        drop(first);

        let snapshot_path = latest_snapshot(directory.path());

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&snapshot_path).expect("read snapshot"))
                .expect("snapshot json");

        value["snapshot"]["challenges"][0]["challenge"]["service_signature"] =
            json!(Ed25519SignatureV1::from_bytes([0x99; 64]).to_base64url());

        fs::write(
            &snapshot_path,
            serde_json::to_vec_pretty(&value).expect("serialize tampered snapshot"),
        )
        .expect("write tampered snapshot");

        assert!(matches!(
            runtime(directory.path(), &kms).await,
            Err(NativePassportServerChallengeRuntimeError::StoredChallengeSignatureInvalid)
        ));
    }

    #[tokio::test]
    async fn stale_writer_never_returns_an_unpersisted_signed_challenge() {
        let directory = TestDirectory::new("stale-writer");
        let kms = DevKms::new();

        let mut first = runtime(directory.path(), &kms)
            .await
            .expect("first runtime");

        let mut stale = runtime(directory.path(), &kms)
            .await
            .expect("stale runtime");

        let persisted = first
            .issue_durable(request(), NOW_MS)
            .await
            .expect("first writer wins");

        assert_eq!(first.state.generation, 1);

        let stale_result = stale.issue_durable(request(), NOW_MS + 1).await;

        assert_eq!(
            stale_result,
            Err(NativePassportServerChallengeRuntimeError::ConcurrentChange)
        );

        /*
         * The stale runtime never advanced memory after failed durable CAS,
         * and the signed challenge it computed was never returned.
         */
        assert_eq!(stale.state.generation, 0);
        assert!(stale.state.snapshot.challenges.is_empty());

        let reopened = runtime(directory.path(), &kms)
            .await
            .expect("reopen winning durable state");

        assert_eq!(reopened.state.generation, 1);
        assert_eq!(reopened.state.snapshot.challenges.len(), 1);
        assert_eq!(reopened.state.snapshot.challenges[0].challenge, persisted);
    }

    #[tokio::test]
    async fn consumption_preflight_is_non_mutating_before_real_consume() {
        let directory = TestDirectory::new("consume-preflight");
        let kms = DevKms::new();

        let mut runtime = runtime(directory.path(), &kms).await.expect("open runtime");

        let challenge = runtime
            .issue_durable(request(), NOW_MS)
            .await
            .expect("issue challenge");

        assert_eq!(runtime.state.generation, 1);
        assert_eq!(
            runtime.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Issued
        );

        runtime
            .validate_consumption_without_mutation(&challenge, NOW_MS + 1)
            .await
            .expect("eligible non-mutating preflight");

        assert_eq!(
            runtime.state.generation, 1,
            "preflight must not publish another generation",
        );

        assert_eq!(
            runtime.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Issued,
            "preflight must not consume the challenge",
        );

        runtime
            .consume_durable(&challenge, NOW_MS + 1)
            .await
            .expect("real consume after preflight");

        assert_eq!(runtime.state.generation, 2);

        assert_eq!(
            runtime.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Consumed
        );
    }

    #[tokio::test]
    async fn expired_consumption_preflight_rejects_without_expiry_mutation() {
        let directory = TestDirectory::new("expired-preflight");
        let kms = DevKms::new();

        let mut runtime = runtime(directory.path(), &kms).await.expect("open runtime");

        let challenge = runtime
            .issue_durable(request(), NOW_MS)
            .await
            .expect("issue challenge");

        let expired_now = challenge.expires_at_ms.checked_add(1).expect("expiry time");

        assert_eq!(
            runtime
                .validate_consumption_without_mutation(&challenge, expired_now,)
                .await,
            Err(NativePassportServerChallengeRuntimeError::ChallengeExpired)
        );

        assert_eq!(
            runtime.state.generation, 1,
            "rejected preflight must not publish expiry state",
        );

        assert_eq!(
            runtime.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Issued,
            "preflight must remain non-mutating",
        );

        /*
         * Existing lifecycle behavior remains authoritative: the real consume
         * call notices the same expiry and durably records Expired.
         */
        assert_eq!(
            runtime.consume_durable(&challenge, expired_now).await,
            Err(NativePassportServerChallengeRuntimeError::ChallengeExpired)
        );

        assert_eq!(runtime.state.generation, 2);

        assert_eq!(
            runtime.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Expired
        );
    }

    #[tokio::test]
    async fn one_time_consume_survives_restart_and_replay_rejects() {
        let directory = TestDirectory::new("consume-restart");
        let kms = DevKms::new();

        let mut first = runtime(directory.path(), &kms).await.expect("open runtime");

        let challenge = first
            .issue_durable(request(), NOW_MS)
            .await
            .expect("issue challenge");

        first
            .consume_durable(&challenge, NOW_MS + 1)
            .await
            .expect("first consume");

        assert_eq!(
            first.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Consumed
        );

        drop(first);

        let mut reopened = runtime(directory.path(), &kms)
            .await
            .expect("reopen consumed state");

        assert_eq!(
            reopened.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Consumed
        );

        assert_eq!(
            reopened.consume_durable(&challenge, NOW_MS + 2).await,
            Err(NativePassportServerChallengeRuntimeError::AlreadyConsumed)
        );
    }

    #[tokio::test]
    async fn cancelled_challenge_survives_restart_and_rejects_consumption() {
        let directory = TestDirectory::new("cancel-restart");
        let kms = DevKms::new();

        let mut first = runtime(directory.path(), &kms).await.expect("open runtime");

        let challenge = first
            .issue_durable(request(), NOW_MS)
            .await
            .expect("issue challenge");

        first
            .cancel_durable(&challenge, NOW_MS + 1)
            .await
            .expect("cancel challenge");

        drop(first);

        let mut reopened = runtime(directory.path(), &kms)
            .await
            .expect("reopen cancelled state");

        assert_eq!(
            reopened.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Cancelled
        );

        assert_eq!(
            reopened.consume_durable(&challenge, NOW_MS + 2).await,
            Err(NativePassportServerChallengeRuntimeError::ChallengeCancelled)
        );
    }

    #[tokio::test]
    async fn expiry_transition_is_durable_and_rejects_after_restart() {
        let directory = TestDirectory::new("expiry-restart");
        let kms = DevKms::new();

        let mut first = runtime(directory.path(), &kms).await.expect("open runtime");

        let challenge = first
            .issue_durable(request(), NOW_MS)
            .await
            .expect("issue challenge");

        let expired_now = challenge.expires_at_ms.checked_add(1).expect("expiry time");

        assert_eq!(
            first.consume_durable(&challenge, expired_now).await,
            Err(NativePassportServerChallengeRuntimeError::ChallengeExpired)
        );

        assert_eq!(
            first.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Expired
        );

        assert_eq!(
            first.state.snapshot.challenges[0].status_changed_at_ms,
            expired_now
        );

        drop(first);

        let mut reopened = runtime(directory.path(), &kms)
            .await
            .expect("reopen expired state");

        assert_eq!(
            reopened.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Expired
        );

        assert_eq!(
            reopened.consume_durable(&challenge, expired_now + 1).await,
            Err(NativePassportServerChallengeRuntimeError::ChallengeExpired)
        );
    }

    #[tokio::test]
    async fn full_presented_challenge_binding_mismatch_never_mutates_state() {
        let directory = TestDirectory::new("binding-mismatch");
        let kms = DevKms::new();

        let mut runtime = runtime(directory.path(), &kms).await.expect("open runtime");

        let challenge = runtime
            .issue_durable(request(), NOW_MS)
            .await
            .expect("issue challenge");

        let mut wrong_nonce = challenge.clone();
        wrong_nonce.nonce = B3DigestHex::parse(
            "challenge_nonce",
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        )
        .expect("wrong nonce");

        let mut wrong_purpose = challenge.clone();
        wrong_purpose.purpose = PassportChallengePurposeV1::ClaimUsername;

        let mut wrong_scopes = challenge.clone();
        wrong_scopes.requested_scopes = vec![scope("catalog.read")];

        let mut wrong_body = challenge.clone();
        wrong_body.operation_body_hash = Some(
            B3DigestHex::parse(
                "operation_body_hash",
                "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            )
            .expect("wrong body hash"),
        );

        let mut wrong_signature = challenge.clone();
        wrong_signature.service_signature = Ed25519SignatureV1::from_bytes([0x77; 64]);

        for wrong in [
            wrong_nonce,
            wrong_purpose,
            wrong_scopes,
            wrong_body,
            wrong_signature,
        ] {
            assert_eq!(
                runtime.consume_durable(&wrong, NOW_MS + 1).await,
                Err(NativePassportServerChallengeRuntimeError::ChallengeBindingMismatch)
            );

            assert_eq!(
                runtime.state.snapshot.challenges[0].status,
                NativePassportServerChallengeStatusV1::Issued
            );

            assert_eq!(runtime.state.generation, 1);
        }
    }

    #[tokio::test]
    async fn concurrent_stale_consumers_have_exactly_one_winner() {
        let directory = TestDirectory::new("double-consume");
        let kms = DevKms::new();

        let mut issuing = runtime(directory.path(), &kms)
            .await
            .expect("open issuing runtime");

        let challenge = issuing
            .issue_durable(request(), NOW_MS)
            .await
            .expect("issue challenge");

        drop(issuing);

        let mut first = runtime(directory.path(), &kms)
            .await
            .expect("first consumer");

        let mut second = runtime(directory.path(), &kms)
            .await
            .expect("second consumer");

        first
            .consume_durable(&challenge, NOW_MS + 1)
            .await
            .expect("first consumer wins");

        assert_eq!(
            second.consume_durable(&challenge, NOW_MS + 1).await,
            Err(NativePassportServerChallengeRuntimeError::AlreadyConsumed)
        );

        let reopened = runtime(directory.path(), &kms)
            .await
            .expect("reopen winner");

        assert_eq!(reopened.state.generation, 2);

        assert_eq!(
            reopened.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Consumed
        );
    }

    #[tokio::test]
    async fn unknown_challenge_is_rejected_without_state_mutation() {
        let directory = TestDirectory::new("unknown");
        let kms = DevKms::new();

        let mut runtime = runtime(directory.path(), &kms).await.expect("open runtime");

        let challenge = runtime
            .issue_durable(request(), NOW_MS)
            .await
            .expect("issue challenge");

        let mut unknown = challenge.clone();

        unknown.challenge_id = ron_proto::ChallengeIdV1::parse(
            "challenge:v1:b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .expect("unknown challenge id");

        assert_eq!(
            runtime.consume_durable(&unknown, NOW_MS + 1).await,
            Err(NativePassportServerChallengeRuntimeError::UnknownChallenge)
        );

        assert_eq!(runtime.state.generation, 1);

        assert_eq!(
            runtime.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Issued
        );
    }

    #[tokio::test]
    async fn not_yet_valid_challenge_rejects_without_mutation() {
        let directory = TestDirectory::new("not-yet-valid");
        let kms = DevKms::new();

        let mut runtime = runtime(directory.path(), &kms).await.expect("open runtime");

        let challenge = runtime
            .issue_durable(request(), NOW_MS)
            .await
            .expect("issue challenge");

        assert_eq!(
            runtime.consume_durable(&challenge, NOW_MS - 1).await,
            Err(NativePassportServerChallengeRuntimeError::ChallengeNotYetValid)
        );

        assert_eq!(runtime.state.generation, 1);

        assert_eq!(
            runtime.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Issued
        );
    }

    #[tokio::test]
    async fn zero_trusted_time_rejects_without_state_mutation() {
        let directory = TestDirectory::new("zero-time");
        let kms = DevKms::new();

        let mut runtime = runtime(directory.path(), &kms).await.expect("open runtime");

        let challenge = runtime
            .issue_durable(request(), NOW_MS)
            .await
            .expect("issue challenge");

        assert_eq!(
            runtime.consume_durable(&challenge, 0).await,
            Err(NativePassportServerChallengeRuntimeError::InvalidTrustedTime)
        );

        assert_eq!(runtime.state.generation, 1);

        assert_eq!(
            runtime.state.snapshot.challenges[0].status,
            NativePassportServerChallengeStatusV1::Issued
        );
    }

    #[tokio::test]
    async fn terminal_replay_record_is_retained_through_policy_deadline() {
        let directory = TestDirectory::new("retention-kept");
        let kms = DevKms::new();

        let mut runtime = runtime(directory.path(), &kms).await.expect("open runtime");

        let first = runtime
            .issue_durable(request(), NOW_MS)
            .await
            .expect("first challenge");

        runtime
            .consume_durable(&first, NOW_MS + 1)
            .await
            .expect("consume first challenge");

        let retention_deadline = NOW_MS + 1 + REPLAY_RETENTION_MS;

        runtime
            .issue_durable(request(), retention_deadline)
            .await
            .expect("issue at retention deadline");

        assert_eq!(runtime.state.snapshot.challenges.len(), 2);

        assert!(runtime.state.snapshot.challenges.iter().any(|record| {
            record.challenge == first
                && record.status == NativePassportServerChallengeStatusV1::Consumed
        }));
    }

    #[tokio::test]
    async fn terminal_replay_record_is_pruned_after_policy_deadline() {
        let directory = TestDirectory::new("retention-pruned");
        let kms = DevKms::new();

        let mut runtime = runtime(directory.path(), &kms).await.expect("open runtime");

        let first = runtime
            .issue_durable(request(), NOW_MS)
            .await
            .expect("first challenge");

        runtime
            .consume_durable(&first, NOW_MS + 1)
            .await
            .expect("consume first challenge");

        let after_retention = NOW_MS + 1 + REPLAY_RETENTION_MS + 1;

        let second = runtime
            .issue_durable(request(), after_retention)
            .await
            .expect("issue after retention");

        assert_eq!(runtime.state.snapshot.challenges.len(), 1);

        assert_eq!(runtime.state.snapshot.challenges[0].challenge, second);

        assert_ne!(first.challenge_id, second.challenge_id);
    }

    #[tokio::test]
    async fn zero_terminal_replay_retention_fails_closed() {
        let directory = TestDirectory::new("zero-retention");
        let kms = DevKms::new();

        let result = NativePassportServerChallengeRuntime::open(
            directory.path(),
            &kms,
            context("rustyonions-devnet"),
            context("private-beta"),
            context("svc-passport"),
            context("svc-passport"),
            CHALLENGE_TTL_MS,
            0,
        )
        .await;

        assert!(matches!(
            result,
            Err(NativePassportServerChallengeRuntimeError::InvalidReplayRetention)
        ));
    }

    #[test]
    fn private_runtime_source_has_no_route_registration_capability_username_or_value_authority() {
        let source = include_str!("server_challenge_runtime.rs");

        let implementation = source
            .split("\n#[cfg(all(test")
            .next()
            .expect("implementation");

        for forbidden in [
            "Router::",
            ".route(",
            "register_root_from_verified_proof(",
            "issue_capability(",
            "claim_username(",
            "wallet.spend(",
            "ledger.write(",
            "root_private_key:",
            "device_private_key:",
            "raw_pin:",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "private challenge runtime gained forbidden authority pattern {forbidden}"
            );
        }
    }
}
