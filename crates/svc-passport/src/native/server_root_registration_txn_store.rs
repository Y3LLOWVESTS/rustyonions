//! RO:WHAT — Private crash-recovery write-ahead intent store for Native Passport RegisterRoot transactions.
//! RO:WHY — Challenge consumption and root registration live in separate durable CAS stores; an immutable redo intent must survive crashes so neither independent write is falsely treated as an atomic transaction.
//! RO:INTERACTS — PassportChallengeV1, ron-auth challenge/root-proof transcript bindings, server_challenge_runtime, server_registry_runtime, and the future private root-registration coordinator.
//! RO:INVARIANTS — intent is durable before either authoritative mutation; one challenge ID names at most one immutable intent; identical prepare is idempotent; conflicting prepare fails closed; completion only removes the exact intent after both authorities commit; pending state is bounded by the challenge-store bound.
//! RO:METRICS — none; future coordinator owns transaction/recovery counters.
//! RO:CONFIG — caller supplies a service-owned private journal directory.
//! RO:SECURITY — stores public signed challenge/root-proof material only; no root/recovery secret, PIN, device private key, capability, username, wallet, or ledger authority; symlink/non-file/corrupt state fails closed.
//! RO:TEST — focused tests prove restart durability, idempotent prepare, conflict rejection, corruption rejection, exact completion, private permissions, and authority isolation.

#![forbid(unsafe_code)]

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use ron_proto::{
    B3DigestHex, ChallengeIdV1, Ed25519PublicKeyHex, PassportChallengePurposeV1,
    PassportChallengeV1,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::server_challenge_store::MAX_CHALLENGE_RECORDS;

const STORE_SCHEMA: &str = "svc-passport.native-passport-root-registration-redo.v1";
const STORE_VERSION: u32 = 1;

const INTENT_PREFIX: &str = "root-registration-v1-";
const INTENT_SUFFIX: &str = ".json";

const TEMP_PREFIX: &str = "root-registration-v1-tmp-";
const TEMP_SUFFIX: &str = ".json";

const MAX_INTENT_BYTES: u64 = 128 * 1024;
const MAX_TEMP_FILES: usize = 64;
const MAX_PENDING_INTENTS: usize = MAX_CHALLENGE_RECORDS;

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

/// Immutable public evidence required to deterministically redo one accepted
/// RegisterRoot transaction after a process or machine crash.
///
/// `accepted_at_ms` is service-trusted transaction time. Recovery reuses this
/// logical time instead of current wall-clock time, allowing a transaction
/// whose durable intent committed before challenge expiry to finish after a
/// later restart without weakening challenge validity rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePassportRootRegistrationTxnIntentV1 {
    pub(super) challenge: PassportChallengeV1,

    pub(super) challenge_transcript_hash: String,

    pub(super) root_public_key: String,

    pub(super) root_key_epoch: u64,

    pub(super) proof_signed_payload_hex: String,

    pub(super) proof_created_at_ms: u64,

    pub(super) accepted_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativePassportRootRegistrationTxnPrepareDispositionV1 {
    Prepared,
    AlreadyPrepared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativePassportRootRegistrationTxnCompleteDispositionV1 {
    Completed,
    AlreadyCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum NativePassportRootRegistrationTxnStoreError {
    #[error("root registration transaction store unavailable: {0}")]
    Unavailable(&'static str),

    #[error("root registration transaction store corrupt: {0}")]
    Corrupt(&'static str),

    #[error("root registration transaction intent conflicts with durable state")]
    IntentConflict,

    #[error("root registration transaction pending-state bound exceeded")]
    PendingIntentBoundExceeded,
}

#[derive(Debug)]
pub(super) struct NativePassportRootRegistrationTxnStore {
    root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntentFileV1 {
    schema: String,
    version: u32,
    intent: NativePassportRootRegistrationTxnIntentV1,
}

impl NativePassportRootRegistrationTxnStore {
    pub(super) fn open(
        root: impl AsRef<Path>,
    ) -> Result<
        (Self, Vec<NativePassportRootRegistrationTxnIntentV1>),
        NativePassportRootRegistrationTxnStoreError,
    > {
        let root = root.as_ref().to_path_buf();

        ensure_private_directory(&root)?;

        let store = Self { root };

        let pending = store.load_pending()?;

        Ok((store, pending))
    }

    /// Durably establish the immutable redo decision before either authoritative
    /// root/challenge store may mutate.
    pub(super) fn prepare(
        &self,
        intent: &NativePassportRootRegistrationTxnIntentV1,
    ) -> Result<
        NativePassportRootRegistrationTxnPrepareDispositionV1,
        NativePassportRootRegistrationTxnStoreError,
    > {
        validate_intent(intent)?;

        let final_path = self.intent_path(&intent.challenge.challenge_id)?;

        if let Some(existing) = self.read_if_present(&final_path)? {
            if existing == *intent {
                return Ok(NativePassportRootRegistrationTxnPrepareDispositionV1::AlreadyPrepared);
            }

            return Err(NativePassportRootRegistrationTxnStoreError::IntentConflict);
        }

        let pending = self.load_pending()?;

        if pending.len() >= MAX_PENDING_INTENTS {
            return Err(NativePassportRootRegistrationTxnStoreError::PendingIntentBoundExceeded);
        }

        let envelope = IntentFileV1 {
            schema: STORE_SCHEMA.to_owned(),
            version: STORE_VERSION,
            intent: intent.clone(),
        };

        let bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|_| unavailable("serialize root registration transaction intent"))?;

        if bytes.len() as u64 > MAX_INTENT_BYTES {
            return Err(unavailable(
                "root registration transaction intent byte bound exceeded",
            ));
        }

        let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);

        let temp_path = self.root.join(format!(
            "{TEMP_PREFIX}{}-{nonce}{TEMP_SUFFIX}",
            std::process::id(),
        ));

        let mut options = OpenOptions::new();

        options.write(true).create_new(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;

            options.mode(0o600);
        }

        let mut file = options
            .open(&temp_path)
            .map_err(|_| unavailable("create root registration transaction temp file"))?;

        if file.write_all(&bytes).is_err() {
            let _ = fs::remove_file(&temp_path);

            return Err(unavailable("write root registration transaction temp file"));
        }

        if file.sync_all().is_err() {
            let _ = fs::remove_file(&temp_path);

            return Err(unavailable("sync root registration transaction temp file"));
        }

        drop(file);

        if let Err(link_error) = fs::hard_link(&temp_path, &final_path) {
            let _ = fs::remove_file(&temp_path);

            if self.read_if_present(&final_path)?.is_some() {
                let existing = read_intent_file(&final_path)?;

                if existing == *intent {
                    return Ok(
                        NativePassportRootRegistrationTxnPrepareDispositionV1::AlreadyPrepared,
                    );
                }

                return Err(NativePassportRootRegistrationTxnStoreError::IntentConflict);
            }

            let _ = link_error;

            return Err(unavailable("publish root registration transaction intent"));
        }

        let final_file = File::open(&final_path)
            .map_err(|_| unavailable("reopen published root registration transaction intent"))?;

        final_file
            .sync_all()
            .map_err(|_| unavailable("sync published root registration transaction intent"))?;

        fs::remove_file(&temp_path)
            .map_err(|_| unavailable("remove root registration transaction temp file"))?;

        sync_directory_best_effort(&self.root);

        Ok(NativePassportRootRegistrationTxnPrepareDispositionV1::Prepared)
    }

    /// Remove the exact redo intent only after the coordinator has proved both
    /// authoritative stores reached the intended outcome.
    pub(super) fn complete(
        &self,
        intent: &NativePassportRootRegistrationTxnIntentV1,
    ) -> Result<
        NativePassportRootRegistrationTxnCompleteDispositionV1,
        NativePassportRootRegistrationTxnStoreError,
    > {
        validate_intent(intent)?;

        let final_path = self.intent_path(&intent.challenge.challenge_id)?;

        let Some(existing) = self.read_if_present(&final_path)? else {
            return Ok(NativePassportRootRegistrationTxnCompleteDispositionV1::AlreadyCompleted);
        };

        if existing != *intent {
            return Err(NativePassportRootRegistrationTxnStoreError::IntentConflict);
        }

        fs::remove_file(&final_path)
            .map_err(|_| unavailable("remove completed root registration transaction intent"))?;

        sync_directory_best_effort(&self.root);

        Ok(NativePassportRootRegistrationTxnCompleteDispositionV1::Completed)
    }

    pub(super) fn load_pending(
        &self,
    ) -> Result<
        Vec<NativePassportRootRegistrationTxnIntentV1>,
        NativePassportRootRegistrationTxnStoreError,
    > {
        let mut intent_paths = Vec::new();
        let mut temp_count = 0_usize;

        for entry in fs::read_dir(&self.root)
            .map_err(|_| unavailable("scan root registration transaction directory"))?
        {
            let entry = entry
                .map_err(|_| unavailable("read root registration transaction directory entry"))?;

            let file_type = entry.file_type().map_err(|_| {
                unavailable("inspect root registration transaction directory entry")
            })?;

            if file_type.is_symlink() {
                return Err(corrupt(
                    "symlink inside root registration transaction directory",
                ));
            }

            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                return Err(corrupt("non-UTF8 root registration transaction filename"));
            };

            if name.starts_with(TEMP_PREFIX) && name.ends_with(TEMP_SUFFIX) {
                if !file_type.is_file() {
                    return Err(corrupt(
                        "root registration transaction temp path is not a regular file",
                    ));
                }

                temp_count = temp_count
                    .checked_add(1)
                    .ok_or_else(|| corrupt("root registration temp count overflow"))?;

                if temp_count > MAX_TEMP_FILES {
                    return Err(corrupt("too many root registration transaction temp files"));
                }

                /*
                 * Never delete another writer's in-flight temp file. Temps are
                 * bounded and ignored until their immutable final link exists.
                 */
                continue;
            }

            if name.starts_with(INTENT_PREFIX) && name.ends_with(INTENT_SUFFIX) {
                if !file_type.is_file() {
                    return Err(corrupt(
                        "root registration transaction intent path is not a regular file",
                    ));
                }

                let challenge_hex = intent_hex_from_filename(name)?;

                intent_paths.push((challenge_hex.to_owned(), entry.path()));
            }
        }

        if intent_paths.len() > MAX_PENDING_INTENTS {
            return Err(NativePassportRootRegistrationTxnStoreError::PendingIntentBoundExceeded);
        }

        intent_paths.sort_by(|(left, _), (right, _)| left.cmp(right));

        let mut pending = Vec::with_capacity(intent_paths.len());

        for (expected_hex, path) in intent_paths {
            let intent = read_intent_file(&path)?;

            if challenge_hex(&intent.challenge.challenge_id)? != expected_hex {
                return Err(corrupt(
                    "root registration transaction filename does not match challenge ID",
                ));
            }

            pending.push(intent);
        }

        Ok(pending)
    }

    fn intent_path(
        &self,
        challenge_id: &ChallengeIdV1,
    ) -> Result<PathBuf, NativePassportRootRegistrationTxnStoreError> {
        Ok(self.root.join(format!(
            "{INTENT_PREFIX}{}{INTENT_SUFFIX}",
            challenge_hex(challenge_id)?,
        )))
    }

    fn read_if_present(
        &self,
        path: &Path,
    ) -> Result<
        Option<NativePassportRootRegistrationTxnIntentV1>,
        NativePassportRootRegistrationTxnStoreError,
    > {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,

            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }

            Err(_) => {
                return Err(unavailable("inspect root registration transaction intent"));
            }
        };

        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(corrupt(
                "root registration transaction intent is not a regular file",
            ));
        }

        Ok(Some(read_intent_file(path)?))
    }
}

fn validate_intent(
    intent: &NativePassportRootRegistrationTxnIntentV1,
) -> Result<(), NativePassportRootRegistrationTxnStoreError> {
    intent
        .challenge
        .validate()
        .map_err(|_| corrupt("invalid challenge in root registration transaction intent"))?;

    if intent.challenge.purpose != PassportChallengePurposeV1::RegisterRoot {
        return Err(corrupt(
            "root registration transaction contains non-RegisterRoot challenge",
        ));
    }

    if intent.challenge.passport_id.is_none()
        || intent.challenge.device_id.is_some()
        || intent.challenge.operation_body_hash.is_none()
    {
        return Err(corrupt(
            "root registration transaction challenge binding is incomplete",
        ));
    }

    B3DigestHex::parse(
        "challenge_transcript_hash",
        &intent.challenge_transcript_hash,
    )
    .map_err(|_| corrupt("invalid root registration challenge transcript hash"))?;

    Ed25519PublicKeyHex::parse(&intent.root_public_key)
        .map_err(|_| corrupt("invalid root registration public key"))?;

    if intent.proof_signed_payload_hex.len() != 128
        || intent
            .proof_signed_payload_hex
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        return Err(corrupt(
            "invalid root registration proof signature encoding",
        ));
    }

    if intent.proof_created_at_ms < intent.challenge.issued_at_ms
        || intent.proof_created_at_ms > intent.challenge.expires_at_ms
    {
        return Err(corrupt(
            "root registration proof time outside challenge window",
        ));
    }

    if intent.accepted_at_ms == 0
        || intent.accepted_at_ms < intent.proof_created_at_ms
        || intent.accepted_at_ms < intent.challenge.issued_at_ms
        || intent.accepted_at_ms > intent.challenge.expires_at_ms
    {
        return Err(corrupt(
            "root registration accepted time outside challenge window",
        ));
    }

    let _ = challenge_hex(&intent.challenge.challenge_id)?;

    Ok(())
}

fn read_intent_file(
    path: &Path,
) -> Result<NativePassportRootRegistrationTxnIntentV1, NativePassportRootRegistrationTxnStoreError>
{
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| unavailable("inspect root registration transaction intent file"))?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(corrupt(
            "root registration transaction intent is not a regular file",
        ));
    }

    if metadata.len() == 0 || metadata.len() > MAX_INTENT_BYTES {
        return Err(corrupt(
            "root registration transaction intent byte length invalid",
        ));
    }

    let mut file =
        File::open(path).map_err(|_| unavailable("open root registration transaction intent"))?;

    let capacity = usize::try_from(metadata.len())
        .map_err(|_| corrupt("root registration transaction intent size overflow"))?;

    let mut bytes = Vec::with_capacity(capacity);

    file.read_to_end(&mut bytes)
        .map_err(|_| unavailable("read root registration transaction intent"))?;

    if bytes.len() as u64 != metadata.len() {
        return Err(corrupt(
            "root registration transaction intent length changed while reading",
        ));
    }

    let envelope: IntentFileV1 = serde_json::from_slice(&bytes)
        .map_err(|_| corrupt("invalid root registration transaction JSON"))?;

    if envelope.schema != STORE_SCHEMA || envelope.version != STORE_VERSION {
        return Err(corrupt(
            "root registration transaction schema/version mismatch",
        ));
    }

    validate_intent(&envelope.intent)?;

    Ok(envelope.intent)
}

fn challenge_hex(
    challenge_id: &ChallengeIdV1,
) -> Result<&str, NativePassportRootRegistrationTxnStoreError> {
    let Some(value) = challenge_id.as_str().strip_prefix("challenge:v1:b3:") else {
        return Err(corrupt(
            "root registration transaction challenge ID prefix mismatch",
        ));
    };

    validate_lower_hex_64(value)?;

    Ok(value)
}

fn intent_hex_from_filename(
    name: &str,
) -> Result<&str, NativePassportRootRegistrationTxnStoreError> {
    let value = name
        .strip_prefix(INTENT_PREFIX)
        .and_then(|value| value.strip_suffix(INTENT_SUFFIX))
        .ok_or_else(|| corrupt("invalid root registration transaction filename"))?;

    validate_lower_hex_64(value)?;

    Ok(value)
}

fn validate_lower_hex_64(value: &str) -> Result<(), NativePassportRootRegistrationTxnStoreError> {
    if value.len() != 64
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        return Err(corrupt(
            "root registration transaction challenge digest is not canonical lower hex",
        ));
    }

    Ok(())
}

fn ensure_private_directory(
    root: &Path,
) -> Result<(), NativePassportRootRegistrationTxnStoreError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(corrupt(
                    "root registration transaction root is not a real directory",
                ));
            }
        }

        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(root)
                .map_err(|_| unavailable("create root registration transaction directory"))?;
        }

        Err(_) => {
            return Err(unavailable(
                "inspect root registration transaction directory",
            ));
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| unavailable("set root registration transaction directory permissions"))?;
    }

    Ok(())
}

fn sync_directory_best_effort(root: &Path) {
    if let Ok(directory) = File::open(root) {
        let _ = directory.sync_all();
    }
}

fn unavailable(reason: &'static str) -> NativePassportRootRegistrationTxnStoreError {
    NativePassportRootRegistrationTxnStoreError::Unavailable(reason)
}

fn corrupt(reason: &'static str) -> NativePassportRootRegistrationTxnStoreError {
    NativePassportRootRegistrationTxnStoreError::Corrupt(reason)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use ron_proto::{
        B3DigestHex, ChallengeIdV1, Ed25519SignatureV1, NativePassportContextLabelV1,
        NativePassportScopeV1, PassportIdV1, ServiceKeyIdV1, PASSPORT_CHALLENGE_V1_VERSION,
    };
    use serde_json::json;

    use super::*;

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    const ISSUED_AT_MS: u64 = 1_000_000;
    const EXPIRES_AT_MS: u64 = 1_060_000;
    const PROOF_AT_MS: u64 = 1_020_000;
    const ACCEPTED_AT_MS: u64 = 1_030_000;

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();

            Self {
                path: std::env::temp_dir().join(format!(
                    "svc-passport-root-registration-txn-{label}-{}-{stamp}",
                    std::process::id(),
                )),
            }
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

    fn challenge() -> PassportChallengeV1 {
        let challenge = PassportChallengeV1 {
            version: PASSPORT_CHALLENGE_V1_VERSION,

            challenge_id: ChallengeIdV1::parse(format!("challenge:v1:b3:{}", "11".repeat(32),))
                .expect("challenge ID"),

            network_id: context("rustyonions-devnet"),
            environment: context("private-beta"),
            audience: context("svc-passport"),
            issuing_service_id: context("svc-passport"),

            service_key_id: ServiceKeyIdV1::parse("ed25519/default/v1").expect("service key ID"),

            purpose: PassportChallengePurposeV1::RegisterRoot,

            requested_scopes: vec![scope("identity.read")],

            passport_id: Some(
                PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_A}",))
                    .expect("Passport ID"),
            ),

            device_id: None,

            operation_body_hash: Some(
                B3DigestHex::parse("operation_body_hash", HEX_D).expect("operation body hash"),
            ),

            nonce: B3DigestHex::parse("challenge_nonce", "22".repeat(32)).expect("challenge nonce"),

            issued_at_ms: ISSUED_AT_MS,
            expires_at_ms: EXPIRES_AT_MS,

            service_signature: Ed25519SignatureV1::from_bytes([0x55; 64]),
        };

        challenge.validate().expect("valid challenge");

        challenge
    }

    fn intent() -> NativePassportRootRegistrationTxnIntentV1 {
        NativePassportRootRegistrationTxnIntentV1 {
            challenge: challenge(),

            challenge_transcript_hash: "33".repeat(32),

            root_public_key: "44".repeat(32),

            root_key_epoch: 0,

            proof_signed_payload_hex: "55".repeat(64),

            proof_created_at_ms: PROOF_AT_MS,

            accepted_at_ms: ACCEPTED_AT_MS,
        }
    }

    #[test]
    fn immutable_intent_survives_restart_and_prepare_is_idempotent() {
        let directory = TestDirectory::new("restart");

        let (store, pending) = NativePassportRootRegistrationTxnStore::open(directory.path())
            .expect("open transaction store");

        assert!(pending.is_empty());

        let intent = intent();

        assert_eq!(
            store.prepare(&intent),
            Ok(NativePassportRootRegistrationTxnPrepareDispositionV1::Prepared)
        );

        assert_eq!(
            store.prepare(&intent),
            Ok(NativePassportRootRegistrationTxnPrepareDispositionV1::AlreadyPrepared)
        );

        drop(store);

        let (_reopened, pending) = NativePassportRootRegistrationTxnStore::open(directory.path())
            .expect("reopen transaction store");

        assert_eq!(pending, vec![intent]);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                fs::metadata(directory.path())
                    .expect("directory metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o700,
            );

            let intent_path = fs::read_dir(directory.path())
                .expect("journal directory")
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            name.starts_with(INTENT_PREFIX) && !name.starts_with(TEMP_PREFIX)
                        })
                })
                .expect("intent path");

            assert_eq!(
                fs::metadata(intent_path)
                    .expect("intent metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600,
            );
        }
    }

    #[test]
    fn conflicting_prepare_for_same_challenge_fails_closed() {
        let directory = TestDirectory::new("conflict");

        let (store, _) = NativePassportRootRegistrationTxnStore::open(directory.path())
            .expect("open transaction store");

        let first = intent();

        store.prepare(&first).expect("prepare first");

        let mut conflicting = first;

        conflicting.proof_signed_payload_hex = "66".repeat(64);

        assert_eq!(
            store.prepare(&conflicting),
            Err(NativePassportRootRegistrationTxnStoreError::IntentConflict)
        );
    }

    #[test]
    fn exact_completion_removes_intent_and_is_idempotent() {
        let directory = TestDirectory::new("complete");

        let (store, _) = NativePassportRootRegistrationTxnStore::open(directory.path())
            .expect("open transaction store");

        let intent = intent();

        store.prepare(&intent).expect("prepare");

        assert_eq!(
            store.complete(&intent),
            Ok(NativePassportRootRegistrationTxnCompleteDispositionV1::Completed)
        );

        assert_eq!(
            store.complete(&intent),
            Ok(NativePassportRootRegistrationTxnCompleteDispositionV1::AlreadyCompleted)
        );

        drop(store);

        let (_reopened, pending) =
            NativePassportRootRegistrationTxnStore::open(directory.path()).expect("reopen");

        assert!(pending.is_empty());
    }

    #[test]
    fn unknown_fields_and_truncation_fail_closed() {
        let unknown_directory = TestDirectory::new("unknown-field");

        let (store, _) = NativePassportRootRegistrationTxnStore::open(unknown_directory.path())
            .expect("open unknown-field store");

        let unknown_intent = intent();

        store.prepare(&unknown_intent).expect("prepare");

        let path = store
            .intent_path(&unknown_intent.challenge.challenge_id)
            .expect("intent path");

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read intent")).expect("intent JSON");

        value["unexpected"] = json!(true);

        fs::write(
            &path,
            serde_json::to_vec_pretty(&value).expect("encode changed intent"),
        )
        .expect("write changed intent");

        assert!(matches!(
            NativePassportRootRegistrationTxnStore::open(unknown_directory.path(),),
            Err(NativePassportRootRegistrationTxnStoreError::Corrupt(_))
        ));

        let truncated_directory = TestDirectory::new("truncated");

        let (store, _) = NativePassportRootRegistrationTxnStore::open(truncated_directory.path())
            .expect("open truncated store");

        let truncated_intent = intent();

        store.prepare(&truncated_intent).expect("prepare");

        let path = store
            .intent_path(&truncated_intent.challenge.challenge_id)
            .expect("intent path");

        fs::write(path, b"{").expect("truncate intent");

        assert!(matches!(
            NativePassportRootRegistrationTxnStore::open(truncated_directory.path(),),
            Err(NativePassportRootRegistrationTxnStoreError::Corrupt(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_intent_fails_closed() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new("symlink");

        let (store, _) =
            NativePassportRootRegistrationTxnStore::open(directory.path()).expect("open store");

        let intent = intent();

        let path = store
            .intent_path(&intent.challenge.challenge_id)
            .expect("intent path");

        let target = directory.path().join("target");

        fs::write(&target, b"{}").expect("target");

        symlink(&target, &path).expect("symlink");

        assert!(matches!(
            store.prepare(&intent),
            Err(NativePassportRootRegistrationTxnStoreError::Corrupt(_))
        ));
    }

    #[test]
    fn transaction_store_has_no_root_challenge_route_or_value_mutation_authority() {
        let source = include_str!("server_root_registration_txn_store.rs");

        let implementation = source
            .split("\n#[cfg(test)]")
            .next()
            .expect("implementation");

        for forbidden in [
            "Router::",
            ".route(",
            "register_root_from_verified_proof(",
            "consume_durable(",
            "issue_capability(",
            "claim_username(",
            "wallet.spend(",
            "ledger.write(",
            "root_private_key:",
            "recovery_phrase:",
            "raw_pin:",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "transaction store gained forbidden authority pattern {forbidden}"
            );
        }
    }
}
