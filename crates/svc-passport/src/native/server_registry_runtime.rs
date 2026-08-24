//! RO:WHAT — Private svc-passport state machine for cryptographically verified durable Native Passport root registration.
//! RO:WHY — CrabNode M1 needs backend Passport truth to originate from real root control before physical `@testmac` identity can receive device-bound authority.
//! RO:INTERACTS — ron-auth canonical RegisterRoot verifier, Native Passport-ID derivation, proof signature DTO, and private immutable-generation server_registry_store.
//! RO:INVARIANTS — server-trusted network/environment/audience/initial epoch override caller intent; concrete root key must derive the claimed Passport ID; the same non-mutating cryptographic preflight gates both future transaction journaling and durable registration; only a strict canonical Ed25519 RegisterRoot proof may create durable root state; identical registration is idempotent; conflicts and lost updates fail closed.
//! RO:METRICS — none yet; public server orchestration will own operation metrics.
//! RO:CONFIG — trusted network/environment and initial root epoch are constructor inputs from service composition, never from untrusted client UI state or a public registration payload.
//! RO:SECURITY — private/unmounted until signed one-time challenge issuance and atomic replay consumption wrap this mutation; no root secret, recovery factor, PIN, capability, username, wallet, or ledger authority is stored here.
//! RO:TEST — focused unit tests below cover persistence, restart idempotency, wrong proof/context/epoch/time, Passport↔root binding, concurrent CAS, and source authority boundaries.

#![forbid(unsafe_code)]

use std::path::Path;

use ron_auth::native_passport::{
    verify_root_registration_proof_v1_strict, RootRegistrationProofTranscriptV1,
};

use ron_proto::NativePassportContextLabelV1;

use super::{
    dto::Ed25519PublicKeyHex as NativeEd25519PublicKeyHex,
    passport_id::derive_native_passport_id_v1,
    proof_signing_adapter::NativeProofSignedPayloadHex,
    server_registry_store::{
        LoadedNativePassportServerRegistryV1, NativePassportServerRegistrySnapshotStore,
        NativePassportServerRegistryStoreError, NativePassportServerRootRecordV1,
    },
    PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN, PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,
    PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
};

const ROOT_REGISTRATION_AUDIENCE: &str = "svc-passport";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativePassportServerRootRegistrationDispositionV1 {
    Registered,
    AlreadyRegistered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct NativePassportServerRootRegistrationOutcomeV1 {
    pub(super) disposition: NativePassportServerRootRegistrationDispositionV1,

    pub(super) durable_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NativePassportServerRootRegistrationError {
    ChallengeContractMismatch,
    ProofContractMismatch,
    TrustedNetworkMismatch,
    TrustedEnvironmentMismatch,
    AudienceMismatch,
    InitialRootEpochMismatch,
    UnexpectedDeviceBinding,
    InvalidRegistrationTime,
    InvalidSignedPayload,
    RootProofVerificationFailed,
    RootPublicKeyInvalid,
    PassportIdDerivationFailed,
    PassportRootBindingMismatch,
    RootRegistrationConflict,
    GenerationOverflow,
    StorageUnavailable(&'static str),
    StorageCorrupt(&'static str),
}

/// Private root-registration mutation authority.
///
/// This type deliberately has no HTTP/Tauri/public native export. A future
/// server-issued challenge/replay runtime must become its only production
/// caller before root registration is exposed through CrabNode ingress.
pub(super) struct NativePassportServerRootRegistryRuntime {
    store: NativePassportServerRegistrySnapshotStore,
    loaded: LoadedNativePassportServerRegistryV1,

    expected_network_id: String,
    expected_environment: String,

    trusted_initial_root_key_epoch: u64,
}

impl NativePassportServerRootRegistryRuntime {
    pub(super) fn open(
        root: impl AsRef<Path>,
        expected_network_id: NativePassportContextLabelV1,
        expected_environment: NativePassportContextLabelV1,
        trusted_initial_root_key_epoch: u64,
    ) -> Result<Self, NativePassportServerRootRegistrationError> {
        let expected_network_id_text = expected_network_id.as_str().to_owned();

        let expected_environment_text = expected_environment.as_str().to_owned();

        let (store, loaded) = NativePassportServerRegistrySnapshotStore::open(
            root,
            expected_network_id,
            expected_environment,
        )
        .map_err(map_store_error)?;

        Ok(Self {
            store,
            loaded,
            expected_network_id: expected_network_id_text,
            expected_environment: expected_environment_text,
            trusted_initial_root_key_epoch,
        })
    }

    /// Verify and durably record a Passport root.
    ///
    /// This method authenticates the concrete root signature and all fixed
    /// server context again at the mutation boundary. It does *not* itself
    /// prove that the challenge was issued/consumed by this server; therefore
    /// the containing module remains private until the challenge/replay state
    /// machine is wired around it.
    pub(super) fn register_root_from_verified_proof(
        &mut self,
        transcript: &RootRegistrationProofTranscriptV1<'_>,
        signed_payload: &NativeProofSignedPayloadHex,
        registered_at_ms: u64,
    ) -> Result<
        NativePassportServerRootRegistrationOutcomeV1,
        NativePassportServerRootRegistrationError,
    > {
        self.validate_root_registration_without_mutation(
            transcript,
            signed_payload,
            registered_at_ms,
        )?;

        if let Some(existing) = self
            .loaded
            .snapshot
            .passports
            .iter()
            .find(|record| record.passport_id.as_str() == transcript.passport_id.as_str())
        {
            if existing.root_public_key.as_str() == transcript.root_public_key.as_str()
                && existing.root_key_epoch == transcript.root_key_epoch
            {
                return Ok(NativePassportServerRootRegistrationOutcomeV1 {
                    disposition:
                        NativePassportServerRootRegistrationDispositionV1::AlreadyRegistered,

                    durable_generation: self.loaded.generation,
                });
            }

            return Err(NativePassportServerRootRegistrationError::RootRegistrationConflict);
        }

        let next_generation = self
            .loaded
            .generation
            .checked_add(1)
            .ok_or(NativePassportServerRootRegistrationError::GenerationOverflow)?;

        let mut next_snapshot = self.loaded.snapshot.clone();

        next_snapshot
            .passports
            .push(NativePassportServerRootRecordV1 {
                passport_id: transcript.passport_id.clone(),

                root_public_key: transcript.root_public_key.clone(),

                root_key_epoch: transcript.root_key_epoch,

                registered_at_ms,
            });

        next_snapshot
            .passports
            .sort_by(|left, right| left.passport_id.as_str().cmp(right.passport_id.as_str()));

        self.store
            .persist(
                self.loaded.generation,
                &self.loaded.snapshot,
                next_generation,
                &next_snapshot,
            )
            .map_err(map_store_error)?;

        self.loaded = LoadedNativePassportServerRegistryV1 {
            generation: next_generation,
            snapshot: next_snapshot,
        };

        Ok(NativePassportServerRootRegistrationOutcomeV1 {
            disposition: NativePassportServerRootRegistrationDispositionV1::Registered,

            durable_generation: next_generation,
        })
    }

    /// Validate the complete RegisterRoot proof without mutating durable state.
    ///
    /// The crash-recovery coordinator must call this before committing its
    /// write-ahead redo intent. The mutating registration path calls this same
    /// function so journal admission and registry mutation cannot drift into
    /// separate cryptographic/context policies.
    pub(super) fn validate_root_registration_without_mutation(
        &self,
        transcript: &RootRegistrationProofTranscriptV1<'_>,
        signed_payload: &NativeProofSignedPayloadHex,
        registered_at_ms: u64,
    ) -> Result<(), NativePassportServerRootRegistrationError> {
        self.validate_server_context(transcript, registered_at_ms)?;

        let signature = decode_signature(signed_payload)?;

        verify_root_registration_proof_v1_strict(transcript, &signature)
            .map_err(|_| NativePassportServerRootRegistrationError::RootProofVerificationFailed)?;

        self.validate_passport_root_binding(transcript)?;

        Ok(())
    }

    fn validate_server_context(
        &self,
        transcript: &RootRegistrationProofTranscriptV1<'_>,
        registered_at_ms: u64,
    ) -> Result<(), NativePassportServerRootRegistrationError> {
        if transcript.challenge_contract_domain != PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN
            || transcript.challenge_contract_version != PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION
        {
            return Err(NativePassportServerRootRegistrationError::ChallengeContractMismatch);
        }

        if transcript.proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
            || transcript.proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
        {
            return Err(NativePassportServerRootRegistrationError::ProofContractMismatch);
        }

        if transcript.network_id != self.expected_network_id {
            return Err(NativePassportServerRootRegistrationError::TrustedNetworkMismatch);
        }

        if transcript.environment != self.expected_environment {
            return Err(NativePassportServerRootRegistrationError::TrustedEnvironmentMismatch);
        }

        if transcript.audience != ROOT_REGISTRATION_AUDIENCE {
            return Err(NativePassportServerRootRegistrationError::AudienceMismatch);
        }

        if transcript.root_key_epoch != self.trusted_initial_root_key_epoch {
            return Err(NativePassportServerRootRegistrationError::InitialRootEpochMismatch);
        }

        if transcript.device_id.is_some() {
            return Err(NativePassportServerRootRegistrationError::UnexpectedDeviceBinding);
        }

        if registered_at_ms == 0
            || registered_at_ms < transcript.challenge_issued_at_ms
            || registered_at_ms < transcript.proof_created_at_ms
            || registered_at_ms > transcript.challenge_expires_at_ms
        {
            return Err(NativePassportServerRootRegistrationError::InvalidRegistrationTime);
        }

        Ok(())
    }

    fn validate_passport_root_binding(
        &self,
        transcript: &RootRegistrationProofTranscriptV1<'_>,
    ) -> Result<(), NativePassportServerRootRegistrationError> {
        let native_root = NativeEd25519PublicKeyHex::parse(transcript.root_public_key.as_str())
            .map_err(|_| NativePassportServerRootRegistrationError::RootPublicKeyInvalid)?;

        let derived_passport_id = derive_native_passport_id_v1(&native_root)
            .map_err(|_| NativePassportServerRootRegistrationError::PassportIdDerivationFailed)?;

        if derived_passport_id.as_str() != transcript.passport_id.as_str() {
            return Err(NativePassportServerRootRegistrationError::PassportRootBindingMismatch);
        }

        Ok(())
    }
}

fn map_store_error(
    error: NativePassportServerRegistryStoreError,
) -> NativePassportServerRootRegistrationError {
    match error {
        NativePassportServerRegistryStoreError::Unavailable(reason) => {
            NativePassportServerRootRegistrationError::StorageUnavailable(reason)
        }

        NativePassportServerRegistryStoreError::Corrupt(reason) => {
            NativePassportServerRootRegistrationError::StorageCorrupt(reason)
        }
    }
}

fn decode_signature(
    value: &NativeProofSignedPayloadHex,
) -> Result<[u8; 64], NativePassportServerRootRegistrationError> {
    let source = value.as_str().as_bytes();

    if source.len() != 128 {
        return Err(NativePassportServerRootRegistrationError::InvalidSignedPayload);
    }

    let mut output = [0_u8; 64];

    for index in 0..64 {
        let high = hex_nibble(source[index * 2])
            .ok_or(NativePassportServerRootRegistrationError::InvalidSignedPayload)?;

        let low = hex_nibble(source[index * 2 + 1])
            .ok_or(NativePassportServerRootRegistrationError::InvalidSignedPayload)?;

        output[index] = (high << 4) | low;
    }

    Ok(output)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use ron_auth::native_passport::RootRegistrationProofTranscriptV1;

    use ron_proto::{
        B3DigestHex, ChallengeIdV1, Ed25519PublicKeyHex as ProtoEd25519PublicKeyHex,
        NativePassportContextLabelV1, PassportIdV1 as ProtoPassportIdV1,
    };

    use crate::native::{
        derive_native_recovery_public_identity_v1, sign_native_recovery_root_registration_proof_v1,
        NativeProofSignedPayloadHex, NativeSecretBytes,
    };

    use super::*;

    const ISSUED_AT_MS: u64 = 1_000_000;
    const PROOF_AT_MS: u64 = 1_030_000;
    const EXPIRES_AT_MS: u64 = 1_300_000;
    const REGISTERED_AT_MS: u64 = 1_040_000;

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
                "svc-passport-root-registry-runtime-{label}-{}-{stamp}",
                std::process::id(),
            ));

            Self { path }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    struct Fixture {
        recovery_factor: NativeSecretBytes,

        passport_id: ProtoPassportIdV1,
        root_public_key: ProtoEd25519PublicKeyHex,

        challenge_id: ChallengeIdV1,
        operation_body_hash: B3DigestHex,
        challenge_transcript_hash: B3DigestHex,
    }

    impl Fixture {
        fn new(byte: u8) -> Self {
            let recovery_factor =
                NativeSecretBytes::new(vec![byte; 32]).expect("recovery factor fixture");

            let root = derive_native_recovery_public_identity_v1(&recovery_factor)
                .expect("public recovery identity");

            Self {
                recovery_factor,

                passport_id: ProtoPassportIdV1::parse(root.passport_id.as_str())
                    .expect("proto Passport ID"),

                root_public_key: ProtoEd25519PublicKeyHex::parse(root.root_public_key.as_str())
                    .expect("proto root public key"),

                challenge_id: ChallengeIdV1::parse(format!(
                    "challenge:v1:b3:{}",
                    format!("{byte:02x}").repeat(32),
                ))
                .expect("challenge ID"),

                operation_body_hash: B3DigestHex::parse("operation_body_hash", "22".repeat(32))
                    .expect("operation body hash"),

                challenge_transcript_hash: B3DigestHex::parse(
                    "challenge_transcript_hash",
                    "33".repeat(32),
                )
                .expect("challenge transcript hash"),
            }
        }

        fn transcript<'a>(
            &'a self,
            scopes: &'a [&'a str],
        ) -> RootRegistrationProofTranscriptV1<'a> {
            RootRegistrationProofTranscriptV1 {
                challenge_contract_domain: PHASE8A_PROOF_CHALLENGE_CONTRACT_DOMAIN,

                challenge_contract_version: PHASE8A_PROOF_CHALLENGE_CONTRACT_VERSION,

                proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,

                proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,

                challenge_id: &self.challenge_id,

                network_id: "rustyonions-devnet",
                environment: "private-beta",
                audience: "svc-passport",

                passport_id: &self.passport_id,
                root_public_key: &self.root_public_key,
                root_key_epoch: 0,

                device_id: None,

                operation_body_hash: &self.operation_body_hash,

                challenge_transcript_hash: &self.challenge_transcript_hash,

                requested_scopes: scopes,

                challenge_issued_at_ms: ISSUED_AT_MS,
                challenge_expires_at_ms: EXPIRES_AT_MS,
                proof_created_at_ms: PROOF_AT_MS,
            }
        }

        fn signed(
            &self,
            transcript: &RootRegistrationProofTranscriptV1<'_>,
        ) -> NativeProofSignedPayloadHex {
            sign_native_recovery_root_registration_proof_v1(&self.recovery_factor, transcript)
                .expect("recovery root proof")
                .signed_payload_hex
        }
    }

    fn open_runtime(root: &Path) -> NativePassportServerRootRegistryRuntime {
        NativePassportServerRootRegistryRuntime::open(
            root,
            NativePassportContextLabelV1::parse("rustyonions-devnet").expect("network"),
            NativePassportContextLabelV1::parse("private-beta").expect("environment"),
            0,
        )
        .expect("open root registry runtime")
    }

    fn snapshot_count(root: &Path) -> usize {
        fs::read_dir(root)
            .expect("registry directory")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.starts_with("registry-v1-") && name.ends_with(".json"))
                    .unwrap_or(false)
            })
            .count()
    }

    #[test]
    fn valid_root_preflight_performs_no_registry_mutation() {
        let directory = TestDirectory::new("preflight-no-mutation");

        let fixture = Fixture::new(0x71);
        let scopes = ["catalog.read", "identity.read"];

        let transcript = fixture.transcript(&scopes);
        let signed = fixture.signed(&transcript);

        let mut runtime = open_runtime(&directory.path);

        runtime
            .validate_root_registration_without_mutation(&transcript, &signed, REGISTERED_AT_MS)
            .expect("valid non-mutating preflight");

        assert_eq!(
            snapshot_count(&directory.path),
            0,
            "preflight must not publish durable registry state",
        );

        let outcome = runtime
            .register_root_from_verified_proof(&transcript, &signed, REGISTERED_AT_MS)
            .expect("registration after preflight");

        assert_eq!(
            outcome.disposition,
            NativePassportServerRootRegistrationDispositionV1::Registered,
        );

        assert_eq!(
            snapshot_count(&directory.path),
            1,
            "only the mutation call may publish registry state",
        );
    }

    #[test]
    fn verified_root_proof_persists_and_restart_is_idempotent() {
        let directory = TestDirectory::new("restart");

        let fixture = Fixture::new(0x61);
        let scopes = ["catalog.read", "identity.read"];

        let transcript = fixture.transcript(&scopes);

        let signed = fixture.signed(&transcript);

        let mut first = open_runtime(&directory.path);

        let outcome = first
            .register_root_from_verified_proof(&transcript, &signed, REGISTERED_AT_MS)
            .expect("first root registration");

        assert_eq!(
            outcome.disposition,
            NativePassportServerRootRegistrationDispositionV1::Registered,
        );

        assert_eq!(outcome.durable_generation, 1,);

        assert_eq!(snapshot_count(&directory.path), 1,);

        drop(first);

        let mut restarted = open_runtime(&directory.path);

        let repeated = restarted
            .register_root_from_verified_proof(&transcript, &signed, REGISTERED_AT_MS + 1)
            .expect("idempotent registration");

        assert_eq!(
            repeated.disposition,
            NativePassportServerRootRegistrationDispositionV1::AlreadyRegistered,
        );

        assert_eq!(repeated.durable_generation, 1,);

        assert_eq!(snapshot_count(&directory.path), 1,);
    }

    #[test]
    fn invalid_signature_never_creates_durable_root() {
        let directory = TestDirectory::new("bad-signature");

        let fixture = Fixture::new(0x62);
        let scopes = ["identity.read"];
        let transcript = fixture.transcript(&scopes);

        let forged = NativeProofSignedPayloadHex::parse(
            "forged_root_registration_signature",
            "00".repeat(64),
        )
        .expect("typed forged signature");

        let mut runtime = open_runtime(&directory.path);

        assert!(matches!(
            runtime.register_root_from_verified_proof(&transcript, &forged, REGISTERED_AT_MS,),
            Err(NativePassportServerRootRegistrationError::RootProofVerificationFailed)
        ));

        assert_eq!(snapshot_count(&directory.path), 0,);
    }

    #[test]
    fn trusted_context_epoch_device_and_time_fail_closed() {
        let directory = TestDirectory::new("trusted-context");

        let fixture = Fixture::new(0x63);
        let scopes = ["identity.read"];

        let mut wrong_network = fixture.transcript(&scopes);

        wrong_network.network_id = "rustyonions-other";

        let wrong_network_signed = fixture.signed(&wrong_network);

        let mut runtime = open_runtime(&directory.path);

        assert!(matches!(
            runtime.register_root_from_verified_proof(
                &wrong_network,
                &wrong_network_signed,
                REGISTERED_AT_MS,
            ),
            Err(NativePassportServerRootRegistrationError::TrustedNetworkMismatch)
        ));

        let mut wrong_epoch = fixture.transcript(&scopes);

        wrong_epoch.root_key_epoch = 1;

        let wrong_epoch_signed = fixture.signed(&wrong_epoch);

        assert!(matches!(
            runtime.register_root_from_verified_proof(
                &wrong_epoch,
                &wrong_epoch_signed,
                REGISTERED_AT_MS,
            ),
            Err(NativePassportServerRootRegistrationError::InitialRootEpochMismatch)
        ));

        let valid = fixture.transcript(&scopes);

        let valid_signed = fixture.signed(&valid);

        assert!(matches!(
            runtime.register_root_from_verified_proof(&valid, &valid_signed, EXPIRES_AT_MS + 1,),
            Err(NativePassportServerRootRegistrationError::InvalidRegistrationTime)
        ));

        assert_eq!(snapshot_count(&directory.path), 0,);
    }

    #[test]
    fn passport_id_must_derive_from_verified_root_public_key() {
        let directory = TestDirectory::new("passport-binding");

        let fixture = Fixture::new(0x64);
        let scopes = ["identity.read"];

        let wrong_passport =
            ProtoPassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{}", "aa".repeat(32),))
                .expect("wrong Passport ID");

        let mut transcript = fixture.transcript(&scopes);

        transcript.passport_id = &wrong_passport;

        /*
         * The purpose-specific recovery signer correctly refuses to sign this
         * invalid binding, so use a valid proof and mutate only the transcript.
         * Strict verification must reject before durable persistence.
         */
        let valid_transcript = fixture.transcript(&scopes);

        let valid_signed = fixture.signed(&valid_transcript);

        let mut runtime = open_runtime(&directory.path);

        assert!(matches!(
            runtime
                .register_root_from_verified_proof(&transcript, &valid_signed, REGISTERED_AT_MS,),
            Err(NativePassportServerRootRegistrationError::RootProofVerificationFailed)
        ));

        assert_eq!(snapshot_count(&directory.path), 0,);
    }

    #[test]
    fn concurrent_stale_generation_cannot_overwrite_first_root() {
        let directory = TestDirectory::new("concurrent-cas");

        let first_fixture = Fixture::new(0x65);

        let second_fixture = Fixture::new(0x66);

        let scopes = ["identity.read"];

        let first_transcript = first_fixture.transcript(&scopes);

        let second_transcript = second_fixture.transcript(&scopes);

        let first_signed = first_fixture.signed(&first_transcript);

        let second_signed = second_fixture.signed(&second_transcript);

        let mut first = open_runtime(&directory.path);

        let mut stale_second = open_runtime(&directory.path);

        first
            .register_root_from_verified_proof(&first_transcript, &first_signed, REGISTERED_AT_MS)
            .expect("first writer");

        assert!(matches!(
            stale_second.register_root_from_verified_proof(
                &second_transcript,
                &second_signed,
                REGISTERED_AT_MS,
            ),
            Err(
                NativePassportServerRootRegistrationError::StorageUnavailable(
                    "Native Passport registry generation changed concurrently"
                )
            )
        ));

        assert_eq!(snapshot_count(&directory.path), 1,);
    }

    #[test]
    fn registration_runtime_has_no_route_capability_username_or_value_authority() {
        let (implementation, _tests) = include_str!("server_registry_runtime.rs")
            .split_once("\n#[cfg(test)]")
            .expect("implementation precedes tests");

        for forbidden in [
            "Router::new",
            ".route(",
            "issue_capability(",
            "claim_username(",
            "wallet.spend(",
            "ledger.write(",
            "root_private_key:",
            "recovery_phrase:",
            "raw_pin:",
            "WebView",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "root registry runtime gained forbidden authority pattern {forbidden}",
            );
        }
    }
}
