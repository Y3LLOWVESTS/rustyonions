//! RO:WHAT — Private durable immutable-generation store for one-time Native Passport server challenge lifecycle state.
//! RO:WHY — CN-4/M1 needs restart-safe replay truth before any signed challenge may become public or authorize Passport mutation.
//! RO:INTERACTS — ron-proto PassportChallengeV1, future server_challenge_runtime, service-owned filesystem state, and future proof/registration composition.
//! RO:INVARIANTS — snapshots are strict/versioned/bounded/canonically sorted; generation publication is CAS-style; complete signed challenge bindings are preserved; this raw store stays private beneath the lifecycle runtime; corruption fails closed.
//! RO:METRICS — none; the future mounted runtime owns bounded operational counters.
//! RO:CONFIG — caller supplies one service-owned directory plus fixed network/environment/audience/service context.
//! RO:SECURITY — stores public signed challenge material and lifecycle metadata only; no root/device secret, PIN, capability, username, wallet, or ledger authority.
//! RO:TEST — focused unit tests below prove restart round-trip, stale-generation rejection, bounds, strict schema/context validation, and authority isolation.

#![forbid(unsafe_code)]

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use ron_proto::{NativePassportContextLabelV1, PassportChallengeV1};
use serde::{Deserialize, Serialize};

const STORE_SCHEMA: &str = "svc-passport.native-passport-server-challenges.v1";
const STORE_VERSION: u32 = 1;

const SNAPSHOT_PREFIX: &str = "challenge-v1-";
const SNAPSHOT_SUFFIX: &str = ".json";
const TEMP_PREFIX: &str = ".challenge-v1-";
const TEMP_SUFFIX: &str = ".tmp";
const GENERATION_DIGITS: usize = 20;

const MAX_SNAPSHOT_BYTES: u64 = 16 * 1024 * 1024;
pub(super) const MAX_CHALLENGE_RECORDS: usize = 4_096;
const MAX_SNAPSHOTS: usize = 32;
const MAX_TEMP_FILES: usize = 64;

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum NativePassportServerChallengeStatusV1 {
    Issued,
    Consumed,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePassportServerChallengeRecordV1 {
    pub(super) challenge: PassportChallengeV1,
    pub(super) status: NativePassportServerChallengeStatusV1,
    pub(super) status_changed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePassportServerChallengeSnapshotV1 {
    pub(super) challenges: Vec<NativePassportServerChallengeRecordV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoadedNativePassportServerChallengeStateV1 {
    pub(super) generation: u64,
    pub(super) snapshot: NativePassportServerChallengeSnapshotV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativePassportServerChallengeStoreError {
    Unavailable(&'static str),
    Corrupt(&'static str),
    ConcurrentChange,
}

#[derive(Debug)]
pub(super) struct NativePassportServerChallengeSnapshotStore {
    root: PathBuf,
    expected_network_id: NativePassportContextLabelV1,
    expected_environment: NativePassportContextLabelV1,
    expected_audience: NativePassportContextLabelV1,
    expected_issuing_service_id: NativePassportContextLabelV1,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotFileV1 {
    schema: String,
    version: u32,
    generation: u64,
    snapshot: NativePassportServerChallengeSnapshotV1,
}

fn unavailable(reason: &'static str) -> NativePassportServerChallengeStoreError {
    NativePassportServerChallengeStoreError::Unavailable(reason)
}

fn corrupt(reason: &'static str) -> NativePassportServerChallengeStoreError {
    NativePassportServerChallengeStoreError::Corrupt(reason)
}

impl NativePassportServerChallengeSnapshotStore {
    pub(super) fn open(
        root: impl AsRef<Path>,
        expected_network_id: NativePassportContextLabelV1,
        expected_environment: NativePassportContextLabelV1,
        expected_audience: NativePassportContextLabelV1,
        expected_issuing_service_id: NativePassportContextLabelV1,
    ) -> Result<
        (Self, LoadedNativePassportServerChallengeStateV1),
        NativePassportServerChallengeStoreError,
    > {
        let root = root.as_ref().to_path_buf();

        ensure_store_directory(&root)?;

        let store = Self {
            root,
            expected_network_id,
            expected_environment,
            expected_audience,
            expected_issuing_service_id,
        };

        let snapshots = store.scan_snapshot_paths()?;

        if snapshots.len() > MAX_SNAPSHOTS {
            return Err(corrupt(
                "too many durable Native Passport challenge snapshots",
            ));
        }

        let mut loaded = LoadedNativePassportServerChallengeStateV1 {
            generation: 0,
            snapshot: NativePassportServerChallengeSnapshotV1::default(),
        };

        for (generation, path) in snapshots {
            let file = read_snapshot(
                &path,
                generation,
                &store.expected_network_id,
                &store.expected_environment,
                &store.expected_audience,
                &store.expected_issuing_service_id,
            )?;

            loaded = LoadedNativePassportServerChallengeStateV1 {
                generation: file.generation,
                snapshot: file.snapshot,
            };
        }

        Ok((store, loaded))
    }

    pub(super) fn persist(
        &self,
        expected_generation: u64,
        expected_snapshot: &NativePassportServerChallengeSnapshotV1,
        next_generation: u64,
        next_snapshot: &NativePassportServerChallengeSnapshotV1,
    ) -> Result<(), NativePassportServerChallengeStoreError> {
        let required_generation = expected_generation
            .checked_add(1)
            .ok_or_else(|| unavailable("Native Passport challenge generation overflow"))?;

        if next_generation != required_generation {
            return Err(corrupt(
                "non-sequential Native Passport challenge generation",
            ));
        }

        validate_snapshot(
            next_snapshot,
            &self.expected_network_id,
            &self.expected_environment,
            &self.expected_audience,
            &self.expected_issuing_service_id,
        )?;

        let mut snapshots = self.scan_snapshot_paths()?;

        let disk_generation = snapshots
            .last()
            .map(|(generation, _)| *generation)
            .unwrap_or(0);

        if disk_generation != expected_generation {
            return Err(NativePassportServerChallengeStoreError::ConcurrentChange);
        }

        if expected_generation == 0 {
            if expected_snapshot != &NativePassportServerChallengeSnapshotV1::default() {
                return Err(corrupt(
                    "memory contains Native Passport challenge state without durable generation",
                ));
            }
        } else {
            let (_, latest_path) = snapshots
                .last()
                .ok_or_else(|| corrupt("Native Passport challenge generation missing"))?;

            let latest = read_snapshot(
                latest_path,
                expected_generation,
                &self.expected_network_id,
                &self.expected_environment,
                &self.expected_audience,
                &self.expected_issuing_service_id,
            )?;

            if &latest.snapshot != expected_snapshot {
                return Err(corrupt(
                    "durable Native Passport challenge state disagrees with memory",
                ));
            }
        }

        while snapshots.len() >= MAX_SNAPSHOTS {
            let (_, oldest) = snapshots.remove(0);

            fs::remove_file(oldest)
                .map_err(|_| unavailable("prune old Native Passport challenge snapshot"))?;
        }

        let envelope = SnapshotFileV1 {
            schema: STORE_SCHEMA.to_owned(),
            version: STORE_VERSION,
            generation: next_generation,
            snapshot: next_snapshot.clone(),
        };

        let bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|_| unavailable("serialize Native Passport challenge snapshot"))?;

        if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
            return Err(unavailable(
                "Native Passport challenge snapshot byte bound exceeded",
            ));
        }

        self.publish_snapshot(next_generation, &bytes)
    }

    fn publish_snapshot(
        &self,
        generation: u64,
        bytes: &[u8],
    ) -> Result<(), NativePassportServerChallengeStoreError> {
        let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);

        let temp_path = self.root.join(format!(
            "{TEMP_PREFIX}{generation:020}-{}-{nonce}{TEMP_SUFFIX}",
            std::process::id(),
        ));

        let final_path = self.root.join(snapshot_filename(generation));

        let mut options = OpenOptions::new();

        options.write(true).create_new(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;

            options.mode(0o600);
        }

        let mut file = options
            .open(&temp_path)
            .map_err(|_| unavailable("create temporary Native Passport challenge snapshot"))?;

        if file.write_all(bytes).is_err() {
            let _ = fs::remove_file(&temp_path);

            return Err(unavailable(
                "write temporary Native Passport challenge snapshot",
            ));
        }

        if file.sync_all().is_err() {
            let _ = fs::remove_file(&temp_path);

            return Err(unavailable(
                "sync temporary Native Passport challenge snapshot",
            ));
        }

        drop(file);

        match fs::hard_link(&temp_path, &final_path) {
            Ok(()) => {}

            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temp_path);

                return Err(NativePassportServerChallengeStoreError::ConcurrentChange);
            }

            Err(_) => {
                let _ = fs::remove_file(&temp_path);

                return Err(unavailable(
                    "publish immutable Native Passport challenge snapshot",
                ));
            }
        }

        /*
         * The temp inode was synced before publication. Reopening the published
         * path and syncing it again makes success depend on the exact immutable
         * pathname that future restarts will load.
         */
        let published = File::open(&final_path)
            .map_err(|_| unavailable("reopen published Native Passport challenge snapshot"))?;

        published
            .sync_all()
            .map_err(|_| unavailable("sync published Native Passport challenge snapshot"))?;

        fs::remove_file(&temp_path)
            .map_err(|_| unavailable("remove temporary Native Passport challenge snapshot"))?;

        /*
         * Rust's portable filesystem API does not expose identical directory
         * durability semantics on every target. Unix directory sync is used
         * where available, but failure here does not weaken the already-synced
         * immutable snapshot into fake application success.
         */
        sync_directory_best_effort(&self.root);

        Ok(())
    }

    fn scan_snapshot_paths(
        &self,
    ) -> Result<Vec<(u64, PathBuf)>, NativePassportServerChallengeStoreError> {
        let metadata = fs::symlink_metadata(&self.root)
            .map_err(|_| unavailable("inspect Native Passport challenge directory"))?;

        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(corrupt(
                "Native Passport challenge root is not a real directory",
            ));
        }

        let entries = fs::read_dir(&self.root)
            .map_err(|_| unavailable("read Native Passport challenge directory"))?;

        let mut snapshots = Vec::new();
        let mut temp_files = 0_usize;

        for entry in entries {
            let entry = entry.map_err(|_| unavailable("read Native Passport challenge entry"))?;

            let file_type = entry
                .file_type()
                .map_err(|_| unavailable("inspect Native Passport challenge entry"))?;

            let name = entry.file_name();

            let name = name
                .to_str()
                .ok_or_else(|| corrupt("Native Passport challenge directory has non-UTF8 entry"))?;

            if name.starts_with(TEMP_PREFIX) && name.ends_with(TEMP_SUFFIX) {
                if file_type.is_symlink() || !file_type.is_file() {
                    return Err(corrupt(
                        "Native Passport challenge temp entry has invalid type",
                    ));
                }

                temp_files = temp_files.checked_add(1).ok_or_else(|| {
                    unavailable("Native Passport challenge temp-file count overflow")
                })?;

                if temp_files > MAX_TEMP_FILES {
                    return Err(unavailable(
                        "too many Native Passport challenge temporary files",
                    ));
                }

                /*
                 * Never delete another writer's in-flight temp file. Normal
                 * publishers remove their own temp. Crash leftovers are
                 * harmless and bounded for later operator cleanup.
                 */
                continue;
            }

            let generation = parse_snapshot_filename(name).ok_or_else(|| {
                corrupt("unexpected entry in Native Passport challenge directory")
            })?;

            if file_type.is_symlink() || !file_type.is_file() {
                return Err(corrupt(
                    "Native Passport challenge snapshot is not a regular file",
                ));
            }

            snapshots.push((generation, entry.path()));
        }

        snapshots.sort_by_key(|(generation, _)| *generation);

        for pair in snapshots.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(corrupt("duplicate Native Passport challenge generation"));
            }
        }

        Ok(snapshots)
    }
}

fn validate_snapshot(
    snapshot: &NativePassportServerChallengeSnapshotV1,
    expected_network_id: &NativePassportContextLabelV1,
    expected_environment: &NativePassportContextLabelV1,
    expected_audience: &NativePassportContextLabelV1,
    expected_issuing_service_id: &NativePassportContextLabelV1,
) -> Result<(), NativePassportServerChallengeStoreError> {
    if snapshot.challenges.len() > MAX_CHALLENGE_RECORDS {
        return Err(corrupt("Native Passport challenge record bound exceeded"));
    }

    for pair in snapshot.challenges.windows(2) {
        if pair[0].challenge.challenge_id.as_str() >= pair[1].challenge.challenge_id.as_str() {
            return Err(corrupt(
                "Native Passport challenge records are not strictly sorted and unique",
            ));
        }
    }

    for record in &snapshot.challenges {
        validate_record(
            record,
            expected_network_id,
            expected_environment,
            expected_audience,
            expected_issuing_service_id,
        )?;
    }

    Ok(())
}

fn validate_record(
    record: &NativePassportServerChallengeRecordV1,
    expected_network_id: &NativePassportContextLabelV1,
    expected_environment: &NativePassportContextLabelV1,
    expected_audience: &NativePassportContextLabelV1,
    expected_issuing_service_id: &NativePassportContextLabelV1,
) -> Result<(), NativePassportServerChallengeStoreError> {
    /*
     * Raw storage validates canonical protocol structure and trusted context.
     * Cryptographic service-signature verification stays in ron-auth and the
     * lifecycle runtime because KMS historical-key lookup is not storage
     * authority.
     */
    record
        .challenge
        .validate()
        .map_err(|_| corrupt("Native Passport durable challenge structure is invalid"))?;

    if &record.challenge.network_id != expected_network_id {
        return Err(corrupt(
            "Native Passport durable challenge network mismatch",
        ));
    }

    if &record.challenge.environment != expected_environment {
        return Err(corrupt(
            "Native Passport durable challenge environment mismatch",
        ));
    }

    if &record.challenge.audience != expected_audience {
        return Err(corrupt(
            "Native Passport durable challenge audience mismatch",
        ));
    }

    if &record.challenge.issuing_service_id != expected_issuing_service_id {
        return Err(corrupt(
            "Native Passport durable challenge issuing-service mismatch",
        ));
    }

    if record.status_changed_at_ms < record.challenge.issued_at_ms {
        return Err(corrupt(
            "Native Passport durable challenge status time precedes issuance",
        ));
    }

    match record.status {
        NativePassportServerChallengeStatusV1::Issued => {
            if record.status_changed_at_ms != record.challenge.issued_at_ms {
                return Err(corrupt(
                    "Native Passport issued challenge status time is inconsistent",
                ));
            }
        }

        NativePassportServerChallengeStatusV1::Consumed => {
            if record.status_changed_at_ms > record.challenge.expires_at_ms {
                return Err(corrupt(
                    "Native Passport challenge was consumed after expiry",
                ));
            }
        }

        NativePassportServerChallengeStatusV1::Expired => {
            if record.status_changed_at_ms <= record.challenge.expires_at_ms {
                return Err(corrupt(
                    "Native Passport expired challenge status time is inconsistent",
                ));
            }
        }

        NativePassportServerChallengeStatusV1::Cancelled => {
            if record.status_changed_at_ms > record.challenge.expires_at_ms {
                return Err(corrupt(
                    "Native Passport challenge was cancelled after expiry",
                ));
            }
        }
    }

    Ok(())
}

fn ensure_store_directory(root: &Path) -> Result<(), NativePassportServerChallengeStoreError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(corrupt(
                    "Native Passport challenge root is not a real directory",
                ));
            }
        }

        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(root)
                .map_err(|_| unavailable("create Native Passport challenge directory"))?;
        }

        Err(_) => {
            return Err(unavailable("inspect Native Passport challenge directory"));
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| unavailable("secure Native Passport challenge directory"))?;
    }

    Ok(())
}

fn snapshot_filename(generation: u64) -> String {
    format!("{SNAPSHOT_PREFIX}{generation:020}{SNAPSHOT_SUFFIX}")
}

fn parse_snapshot_filename(name: &str) -> Option<u64> {
    let digits = name
        .strip_prefix(SNAPSHOT_PREFIX)?
        .strip_suffix(SNAPSHOT_SUFFIX)?;

    if digits.len() != GENERATION_DIGITS || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let generation = digits.parse::<u64>().ok()?;

    (generation != 0).then_some(generation)
}

fn read_snapshot(
    path: &Path,
    expected_generation: u64,
    expected_network_id: &NativePassportContextLabelV1,
    expected_environment: &NativePassportContextLabelV1,
    expected_audience: &NativePassportContextLabelV1,
    expected_issuing_service_id: &NativePassportContextLabelV1,
) -> Result<SnapshotFileV1, NativePassportServerChallengeStoreError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| unavailable("inspect Native Passport challenge snapshot"))?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(corrupt(
            "Native Passport challenge snapshot is not a regular file",
        ));
    }

    if metadata.len() == 0 || metadata.len() > MAX_SNAPSHOT_BYTES {
        return Err(corrupt(
            "Native Passport challenge snapshot size is invalid",
        ));
    }

    let bytes =
        fs::read(path).map_err(|_| unavailable("read Native Passport challenge snapshot"))?;

    if bytes.len() as u64 != metadata.len() {
        return Err(corrupt(
            "Native Passport challenge snapshot changed while reading",
        ));
    }

    let file = serde_json::from_slice::<SnapshotFileV1>(&bytes)
        .map_err(|_| corrupt("Native Passport challenge snapshot JSON is invalid"))?;

    if file.schema != STORE_SCHEMA {
        return Err(corrupt(
            "Native Passport challenge snapshot schema is unsupported",
        ));
    }

    if file.version != STORE_VERSION {
        return Err(corrupt(
            "Native Passport challenge snapshot version is unsupported",
        ));
    }

    if file.generation != expected_generation {
        return Err(corrupt(
            "Native Passport challenge snapshot generation does not match filename",
        ));
    }

    validate_snapshot(
        &file.snapshot,
        expected_network_id,
        expected_environment,
        expected_audience,
        expected_issuing_service_id,
    )?;

    Ok(file)
}

fn sync_directory_best_effort(root: &Path) {
    #[cfg(unix)]
    {
        if let Ok(directory) = File::open(root) {
            let _ = directory.sync_all();
        }
    }

    #[cfg(not(unix))]
    {
        let _ = root;
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use ron_proto::{
        B3DigestHex, ChallengeIdV1, Ed25519SignatureV1, NativePassportScopeV1,
        PassportChallengePurposeV1, PassportIdV1, ServiceKeyIdV1, PASSPORT_CHALLENGE_V1_VERSION,
    };
    use serde_json::json;

    use super::*;

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const ISSUED_AT_MS: u64 = 1_000_000;
    const EXPIRES_AT_MS: u64 = 1_060_000;

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
                "svc-passport-native-challenge-store-{label}-{}-{stamp}",
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

    fn challenge(index: u64) -> PassportChallengeV1 {
        let challenge = PassportChallengeV1 {
            version: PASSPORT_CHALLENGE_V1_VERSION,

            challenge_id: ChallengeIdV1::parse(format!("challenge:v1:b3:{index:064x}"))
                .expect("challenge id"),

            network_id: context("rustyonions-devnet"),
            environment: context("private-beta"),
            audience: context("svc-passport"),
            issuing_service_id: context("svc-passport"),

            service_key_id: ServiceKeyIdV1::parse("ed25519/default/v1").expect("service key id"),

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

            nonce: B3DigestHex::parse("challenge_nonce", format!("{:064x}", index + 1))
                .expect("nonce"),

            issued_at_ms: ISSUED_AT_MS,
            expires_at_ms: EXPIRES_AT_MS,

            /*
             * Storage is not cryptographic verification authority.
             * The lifecycle runtime will accept only issuer-produced,
             * ron-auth-verified challenges and will revalidate stored
             * signatures with KMS historical-key verification on load.
             */
            service_signature: Ed25519SignatureV1::from_bytes([0x55; 64]),
        };

        challenge.validate().expect("valid challenge fixture");

        challenge
    }

    fn record(index: u64) -> NativePassportServerChallengeRecordV1 {
        NativePassportServerChallengeRecordV1 {
            challenge: challenge(index),
            status: NativePassportServerChallengeStatusV1::Issued,
            status_changed_at_ms: ISSUED_AT_MS,
        }
    }

    fn open_store(
        root: &Path,
    ) -> Result<
        (
            NativePassportServerChallengeSnapshotStore,
            LoadedNativePassportServerChallengeStateV1,
        ),
        NativePassportServerChallengeStoreError,
    > {
        NativePassportServerChallengeSnapshotStore::open(
            root,
            context("rustyonions-devnet"),
            context("private-beta"),
            context("svc-passport"),
            context("svc-passport"),
        )
    }

    #[test]
    fn issued_snapshot_round_trips_complete_challenge_across_restart() {
        let directory = TestDirectory::new("restart");

        let (store, loaded) = open_store(directory.path()).expect("open empty store");

        let next_snapshot = NativePassportServerChallengeSnapshotV1 {
            challenges: vec![record(1)],
        };

        store
            .persist(loaded.generation, &loaded.snapshot, 1, &next_snapshot)
            .expect("persist issued state");

        let (_, reopened) = open_store(directory.path()).expect("reopen durable store");

        assert_eq!(reopened.generation, 1);
        assert_eq!(reopened.snapshot, next_snapshot);
    }

    #[test]
    fn stale_generation_compare_and_swap_is_rejected() {
        let directory = TestDirectory::new("cas");

        let (first_store, first_loaded) = open_store(directory.path()).expect("first open");

        let (second_store, second_loaded) = open_store(directory.path()).expect("second open");

        let first_snapshot = NativePassportServerChallengeSnapshotV1 {
            challenges: vec![record(1)],
        };

        let second_snapshot = NativePassportServerChallengeSnapshotV1 {
            challenges: vec![record(2)],
        };

        first_store
            .persist(
                first_loaded.generation,
                &first_loaded.snapshot,
                1,
                &first_snapshot,
            )
            .expect("first writer wins");

        assert_eq!(
            second_store.persist(
                second_loaded.generation,
                &second_loaded.snapshot,
                1,
                &second_snapshot,
            ),
            Err(NativePassportServerChallengeStoreError::ConcurrentChange)
        );
    }

    #[test]
    fn challenge_record_bound_is_enforced_before_unbounded_growth() {
        let fixture = record(1);

        let snapshot = NativePassportServerChallengeSnapshotV1 {
            challenges: vec![fixture; MAX_CHALLENGE_RECORDS + 1],
        };

        assert_eq!(
            validate_snapshot(
                &snapshot,
                &context("rustyonions-devnet"),
                &context("private-beta"),
                &context("svc-passport"),
                &context("svc-passport"),
            ),
            Err(NativePassportServerChallengeStoreError::Corrupt(
                "Native Passport challenge record bound exceeded"
            ))
        );
    }

    #[test]
    fn unknown_fields_and_context_tampering_fail_closed() {
        let unknown_directory = TestDirectory::new("unknown-field");

        let (store, loaded) = open_store(unknown_directory.path()).expect("open unknown store");

        let snapshot = NativePassportServerChallengeSnapshotV1 {
            challenges: vec![record(1)],
        };

        store
            .persist(loaded.generation, &loaded.snapshot, 1, &snapshot)
            .expect("persist snapshot");

        let path = unknown_directory.path().join(snapshot_filename(1));

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read snapshot")).expect("json");

        value["unexpected"] = json!(true);

        fs::write(&path, serde_json::to_vec_pretty(&value).expect("encode"))
            .expect("write unknown field");

        assert!(matches!(
            open_store(unknown_directory.path()),
            Err(NativePassportServerChallengeStoreError::Corrupt(_))
        ));

        let context_directory = TestDirectory::new("context-tamper");

        let (store, loaded) = open_store(context_directory.path()).expect("open context store");

        store
            .persist(loaded.generation, &loaded.snapshot, 1, &snapshot)
            .expect("persist context snapshot");

        let path = context_directory.path().join(snapshot_filename(1));

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read snapshot")).expect("json");

        value["snapshot"]["challenges"][0]["challenge"]["network_id"] = json!("rustyonions-wrong");

        fs::write(&path, serde_json::to_vec_pretty(&value).expect("encode"))
            .expect("write context tamper");

        assert!(matches!(
            open_store(context_directory.path()),
            Err(NativePassportServerChallengeStoreError::Corrupt(_))
        ));
    }

    #[test]
    fn private_store_source_has_no_route_capability_username_value_or_registration_authority() {
        let source = include_str!("server_challenge_store.rs");

        let implementation = source
            .split("\n#[cfg(test)]")
            .next()
            .expect("implementation");

        for forbidden in [
            "Router::",
            ".route(",
            "issue_capability(",
            "claim_username(",
            "wallet.spend(",
            "ledger.write(",
            "register_root_from_verified_proof(",
            "root_private_key:",
            "device_private_key:",
            "raw_pin:",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "private challenge store gained forbidden authority pattern {forbidden}"
            );
        }
    }
}
