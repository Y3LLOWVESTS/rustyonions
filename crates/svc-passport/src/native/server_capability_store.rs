//! RO:WHAT — Private durable immutable-generation storage for Native Passport device-bound capability state.
//! RO:WHY — CN-4 capability issuance must survive restart, reject stale writers, and remain isolated from the root/device registry before any capability can authorize namespace mutation.
//! RO:INTERACTS — ron-proto NativePassportDeviceBoundCapabilityV1/ChallengeIdV1 DTOs and the future server_capability_runtime issuance/revocation coordinator.
//! RO:INVARIANTS — raw persistence stays private; snapshots are strict/versioned/bounded/canonically ordered; generation publication is compare-and-swap; capability IDs are unique; stored audience/environment remain service-bound.
//! RO:METRICS — none; the future runtime owns redacted operational metrics.
//! RO:CONFIG — caller supplies a dedicated service-owned capability directory plus trusted audience/environment.
//! RO:SECURITY — stores public capability authority metadata only; no DeviceKey, root key, PIN, recovery material, bearer secret, route authority, username mutation, wallet, or ledger mutation.
//! RO:TEST — focused unit tests in this file cover restart, CAS, corruption, strict context, and lifecycle shape.

#![forbid(unsafe_code)]

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use ron_proto::{
    ChallengeIdV1, NativePassportContextLabelV1, NativePassportDeviceBoundCapabilityV1,
};
use serde::{Deserialize, Serialize};

const STORE_SCHEMA: &str = "svc-passport.native-passport-server-capabilities.v1";
const STORE_VERSION: u32 = 1;

const SNAPSHOT_PREFIX: &str = "capabilities-v1-";
const SNAPSHOT_SUFFIX: &str = ".json";
const TEMP_PREFIX: &str = ".capabilities-v1-";
const TEMP_SUFFIX: &str = ".tmp";
const GENERATION_DIGITS: usize = 20;

const MAX_SNAPSHOT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CAPABILITIES: usize = 100_000;
const MAX_SNAPSHOTS: usize = 32;

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum NativePassportServerCapabilityStatusV1 {
    Active,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePassportServerCapabilityRecordV1 {
    pub(super) capability: NativePassportDeviceBoundCapabilityV1,
    pub(super) issued_from_challenge_id: ChallengeIdV1,
    pub(super) status: NativePassportServerCapabilityStatusV1,
    pub(super) status_changed_at_ms: u64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) revoked_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePassportServerCapabilitySnapshotV1 {
    pub(super) capabilities: Vec<NativePassportServerCapabilityRecordV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoadedNativePassportServerCapabilityStateV1 {
    pub(super) generation: u64,
    pub(super) snapshot: NativePassportServerCapabilitySnapshotV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativePassportServerCapabilityStoreError {
    Unavailable(&'static str),
    Corrupt(&'static str),
}

#[derive(Debug)]
pub(super) struct NativePassportServerCapabilitySnapshotStore {
    root: PathBuf,
    expected_audience: NativePassportContextLabelV1,
    expected_environment: NativePassportContextLabelV1,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotFileV1 {
    schema: String,
    version: u32,
    generation: u64,
    snapshot: NativePassportServerCapabilitySnapshotV1,
}

fn unavailable(reason: &'static str) -> NativePassportServerCapabilityStoreError {
    NativePassportServerCapabilityStoreError::Unavailable(reason)
}

fn corrupt(reason: &'static str) -> NativePassportServerCapabilityStoreError {
    NativePassportServerCapabilityStoreError::Corrupt(reason)
}

impl NativePassportServerCapabilitySnapshotStore {
    pub(super) fn open(
        root: impl AsRef<Path>,
        expected_audience: NativePassportContextLabelV1,
        expected_environment: NativePassportContextLabelV1,
    ) -> Result<
        (Self, LoadedNativePassportServerCapabilityStateV1),
        NativePassportServerCapabilityStoreError,
    > {
        let root = root.as_ref().to_path_buf();

        ensure_store_directory(&root)?;

        let store = Self {
            root,
            expected_audience,
            expected_environment,
        };

        let snapshots = store.scan_snapshot_paths(true)?;

        if snapshots.len() > MAX_SNAPSHOTS {
            return Err(corrupt(
                "too many durable Native Passport capability snapshots",
            ));
        }

        let mut loaded = LoadedNativePassportServerCapabilityStateV1 {
            generation: 0,
            snapshot: NativePassportServerCapabilitySnapshotV1::default(),
        };

        for (generation, path) in snapshots {
            let file = read_snapshot(
                &path,
                generation,
                &store.expected_audience,
                &store.expected_environment,
            )?;

            loaded = LoadedNativePassportServerCapabilityStateV1 {
                generation: file.generation,
                snapshot: file.snapshot,
            };
        }

        Ok((store, loaded))
    }

    pub(super) fn persist(
        &self,
        expected_generation: u64,
        expected_snapshot: &NativePassportServerCapabilitySnapshotV1,
        next_generation: u64,
        next_snapshot: &NativePassportServerCapabilitySnapshotV1,
    ) -> Result<(), NativePassportServerCapabilityStoreError> {
        let required_generation = expected_generation
            .checked_add(1)
            .ok_or_else(|| unavailable("Native Passport capability generation overflow"))?;

        if next_generation != required_generation {
            return Err(corrupt(
                "non-sequential Native Passport capability generation",
            ));
        }

        validate_snapshot(
            next_snapshot,
            &self.expected_audience,
            &self.expected_environment,
        )?;

        let mut snapshots = self.scan_snapshot_paths(true)?;

        let disk_generation = snapshots
            .last()
            .map(|(generation, _)| *generation)
            .unwrap_or(0);

        if disk_generation != expected_generation {
            return Err(unavailable(
                "Native Passport capability generation changed concurrently",
            ));
        }

        if expected_generation == 0 {
            if expected_snapshot != &NativePassportServerCapabilitySnapshotV1::default() {
                return Err(corrupt(
                    "memory contains Native Passport capability state without a durable generation",
                ));
            }
        } else {
            let (_, latest_path) = snapshots
                .last()
                .ok_or_else(|| corrupt("Native Passport capability generation missing"))?;

            let latest = read_snapshot(
                latest_path,
                expected_generation,
                &self.expected_audience,
                &self.expected_environment,
            )?;

            if &latest.snapshot != expected_snapshot {
                return Err(corrupt(
                    "durable Native Passport capability state disagrees with memory",
                ));
            }
        }

        while snapshots.len() >= MAX_SNAPSHOTS {
            let (_, oldest) = snapshots.remove(0);

            fs::remove_file(oldest)
                .map_err(|_| unavailable("prune old Native Passport capability snapshot"))?;
        }

        let envelope = SnapshotFileV1 {
            schema: STORE_SCHEMA.to_owned(),
            version: STORE_VERSION,
            generation: next_generation,
            snapshot: next_snapshot.clone(),
        };

        let bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|_| unavailable("serialize Native Passport capability snapshot"))?;

        if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
            return Err(unavailable(
                "Native Passport capability snapshot byte bound exceeded",
            ));
        }

        self.publish_snapshot(next_generation, &bytes)
    }

    fn publish_snapshot(
        &self,
        generation: u64,
        bytes: &[u8],
    ) -> Result<(), NativePassportServerCapabilityStoreError> {
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
            .map_err(|_| unavailable("create temporary Native Passport capability snapshot"))?;

        if file.write_all(bytes).is_err() {
            let _ = fs::remove_file(&temp_path);
            return Err(unavailable(
                "write temporary Native Passport capability snapshot",
            ));
        }

        if file.sync_all().is_err() {
            let _ = fs::remove_file(&temp_path);
            return Err(unavailable(
                "sync temporary Native Passport capability snapshot",
            ));
        }

        drop(file);

        if final_path.exists() {
            let _ = fs::remove_file(&temp_path);
            return Err(unavailable(
                "Native Passport capability generation already exists",
            ));
        }

        if fs::hard_link(&temp_path, &final_path).is_err() {
            let _ = fs::remove_file(&temp_path);
            return Err(unavailable(
                "publish immutable Native Passport capability snapshot",
            ));
        }

        let _ = fs::remove_file(&temp_path);

        sync_directory_best_effort(&self.root);

        Ok(())
    }

    fn scan_snapshot_paths(
        &self,
        clean_temps: bool,
    ) -> Result<Vec<(u64, PathBuf)>, NativePassportServerCapabilityStoreError> {
        let mut snapshots = Vec::new();

        for entry in fs::read_dir(&self.root)
            .map_err(|_| unavailable("read Native Passport capability directory"))?
        {
            let entry = entry
                .map_err(|_| unavailable("read Native Passport capability directory entry"))?;

            let file_type = entry
                .file_type()
                .map_err(|_| unavailable("inspect Native Passport capability directory entry"))?;

            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| corrupt("non-Unicode Native Passport capability directory entry"))?;

            if name.starts_with(TEMP_PREFIX) && name.ends_with(TEMP_SUFFIX) {
                if file_type.is_symlink() || !file_type.is_file() {
                    return Err(corrupt(
                        "Native Passport capability temp entry is not a regular file",
                    ));
                }

                if clean_temps {
                    fs::remove_file(entry.path()).map_err(|_| {
                        unavailable("remove stale Native Passport capability temp file")
                    })?;
                }

                continue;
            }

            let generation = parse_snapshot_filename(&name).ok_or_else(|| {
                corrupt("unexpected entry in Native Passport capability directory")
            })?;

            if file_type.is_symlink() || !file_type.is_file() {
                return Err(corrupt(
                    "Native Passport capability snapshot is not a regular file",
                ));
            }

            snapshots.push((generation, entry.path()));
        }

        snapshots.sort_by_key(|(generation, _)| *generation);

        for pair in snapshots.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(corrupt("duplicate Native Passport capability generation"));
            }
        }

        Ok(snapshots)
    }
}

fn validate_snapshot(
    snapshot: &NativePassportServerCapabilitySnapshotV1,
    expected_audience: &NativePassportContextLabelV1,
    expected_environment: &NativePassportContextLabelV1,
) -> Result<(), NativePassportServerCapabilityStoreError> {
    if snapshot.capabilities.len() > MAX_CAPABILITIES {
        return Err(corrupt("Native Passport capability record bound exceeded"));
    }

    let mut previous_id: Option<&str> = None;

    for record in &snapshot.capabilities {
        record
            .capability
            .validate()
            .map_err(|_| corrupt("stored Native Passport capability is invalid"))?;

        if &record.capability.audience != expected_audience {
            return Err(corrupt(
                "stored Native Passport capability audience mismatch",
            ));
        }

        if &record.capability.environment != expected_environment {
            return Err(corrupt(
                "stored Native Passport capability environment mismatch",
            ));
        }

        if record.status_changed_at_ms < record.capability.issued_at_ms {
            return Err(corrupt(
                "Native Passport capability lifecycle time is inconsistent",
            ));
        }

        match record.status {
            NativePassportServerCapabilityStatusV1::Active => {
                if record.revoked_at_ms.is_some()
                    || record.status_changed_at_ms != record.capability.issued_at_ms
                {
                    return Err(corrupt(
                        "active Native Passport capability lifecycle is inconsistent",
                    ));
                }
            }
            NativePassportServerCapabilityStatusV1::Revoked => {
                let revoked_at_ms = record.revoked_at_ms.ok_or_else(|| {
                    corrupt("revoked Native Passport capability is missing revocation time")
                })?;

                if revoked_at_ms != record.status_changed_at_ms
                    || revoked_at_ms < record.capability.issued_at_ms
                {
                    return Err(corrupt(
                        "revoked Native Passport capability lifecycle is inconsistent",
                    ));
                }
            }
        }

        let current_id = record.capability.capability_id.as_str();

        if previous_id.is_some_and(|previous| previous >= current_id) {
            return Err(corrupt(
                "Native Passport capability records are not canonically ordered and unique",
            ));
        }

        previous_id = Some(current_id);
    }

    Ok(())
}

fn ensure_store_directory(root: &Path) -> Result<(), NativePassportServerCapabilityStoreError> {
    fs::create_dir_all(root)
        .map_err(|_| unavailable("create Native Passport capability directory"))?;

    let metadata = fs::symlink_metadata(root)
        .map_err(|_| unavailable("inspect Native Passport capability directory"))?;

    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(corrupt(
            "Native Passport capability store root is not a real directory",
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| unavailable("secure Native Passport capability directory"))?;
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

    if generation == 0 {
        return None;
    }

    Some(generation)
}

fn read_snapshot(
    path: &Path,
    expected_generation: u64,
    expected_audience: &NativePassportContextLabelV1,
    expected_environment: &NativePassportContextLabelV1,
) -> Result<SnapshotFileV1, NativePassportServerCapabilityStoreError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| unavailable("inspect Native Passport capability snapshot"))?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(corrupt(
            "Native Passport capability snapshot is not a regular file",
        ));
    }

    if metadata.len() > MAX_SNAPSHOT_BYTES {
        return Err(corrupt(
            "Native Passport capability snapshot exceeds byte bound",
        ));
    }

    let bytes =
        fs::read(path).map_err(|_| unavailable("read Native Passport capability snapshot"))?;

    let file: SnapshotFileV1 = serde_json::from_slice(&bytes)
        .map_err(|_| corrupt("decode Native Passport capability snapshot"))?;

    if file.schema != STORE_SCHEMA {
        return Err(corrupt(
            "Native Passport capability snapshot schema is unsupported",
        ));
    }

    if file.version != STORE_VERSION {
        return Err(corrupt(
            "Native Passport capability snapshot version is unsupported",
        ));
    }

    if file.generation != expected_generation {
        return Err(corrupt(
            "Native Passport capability snapshot generation does not match filename",
        ));
    }

    validate_snapshot(&file.snapshot, expected_audience, expected_environment)?;

    Ok(file)
}

fn sync_directory_best_effort(root: &Path) {
    #[cfg(unix)]
    {
        if let Ok(directory) = File::open(root) {
            let _ = directory.sync_all();
        }
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
        CapabilityIdV1, ChallengeIdV1, DeviceIdV1, NativePassportDeviceBoundCapabilityV1,
        NativePassportScopeV1, PassportIdV1,
    };
    use serde_json::json;

    use super::*;

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    const ISSUED_AT_MS: u64 = 1_800_000_000_000;
    const EXPIRES_AT_MS: u64 = ISSUED_AT_MS + 3_600_000;

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
                    "svc-passport-capability-store-{label}-{}-{stamp}",
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

    fn capability() -> NativePassportDeviceBoundCapabilityV1 {
        NativePassportDeviceBoundCapabilityV1 {
            version: 1,
            capability_id: CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_A}"))
                .expect("capability ID"),
            passport_id: PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}"))
                .expect("Passport ID"),
            device_id: DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}"))
                .expect("Device ID"),
            audience: context("svc-passport"),
            environment: context("private-beta"),
            scopes: vec![scope("identity.read"), scope("identity.username.claim")],
            issued_at_ms: ISSUED_AT_MS,
            expires_at_ms: EXPIRES_AT_MS,
            policy_version: 1,
            root_key_epoch: Some(0),
        }
    }

    fn snapshot() -> NativePassportServerCapabilitySnapshotV1 {
        NativePassportServerCapabilitySnapshotV1 {
            capabilities: vec![NativePassportServerCapabilityRecordV1 {
                capability: capability(),
                issued_from_challenge_id: ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_D}"))
                    .expect("challenge ID"),
                status: NativePassportServerCapabilityStatusV1::Active,
                status_changed_at_ms: ISSUED_AT_MS,
                revoked_at_ms: None,
            }],
        }
    }

    fn open_store(
        root: &Path,
    ) -> Result<
        (
            NativePassportServerCapabilitySnapshotStore,
            LoadedNativePassportServerCapabilityStateV1,
        ),
        NativePassportServerCapabilityStoreError,
    > {
        NativePassportServerCapabilitySnapshotStore::open(
            root,
            context("svc-passport"),
            context("private-beta"),
        )
    }

    fn latest_snapshot(root: &Path) -> PathBuf {
        let mut paths = fs::read_dir(root)
            .expect("read capability directory")
            .map(|entry| entry.expect("entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with(SNAPSHOT_PREFIX) && name.ends_with(SNAPSHOT_SUFFIX)
                    })
            })
            .collect::<Vec<_>>();

        paths.sort();
        paths.pop().expect("snapshot")
    }

    #[test]
    fn empty_store_opens_and_persistence_stays_private() {
        let directory = TestDirectory::new("empty");

        let (_store, loaded) = open_store(directory.path()).expect("open empty store");

        assert_eq!(loaded.generation, 0);
        assert_eq!(
            loaded.snapshot,
            NativePassportServerCapabilitySnapshotV1::default(),
        );

        let native_mod = include_str!("mod.rs");
        assert!(native_mod.contains("mod server_capability_store;"));
        assert!(!native_mod.contains("pub mod server_capability_store;"));
    }

    #[test]
    fn active_capability_survives_restart_with_private_permissions() {
        let directory = TestDirectory::new("restart");
        let (store, loaded) = open_store(directory.path()).expect("open");

        let next = snapshot();

        store
            .persist(loaded.generation, &loaded.snapshot, 1, &next)
            .expect("persist");

        let (_reopened, reloaded) = open_store(directory.path()).expect("reopen");

        assert_eq!(reloaded.generation, 1);
        assert_eq!(reloaded.snapshot, next);

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

            assert_eq!(
                fs::metadata(latest_snapshot(directory.path()))
                    .expect("snapshot metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600,
            );
        }
    }

    #[test]
    fn compare_and_swap_rejects_lost_update() {
        let directory = TestDirectory::new("cas");

        let (first, first_loaded) = open_store(directory.path()).expect("first");
        let (second, second_loaded) = open_store(directory.path()).expect("second");

        let next = snapshot();

        first
            .persist(0, &first_loaded.snapshot, 1, &next)
            .expect("first wins");

        assert!(matches!(
            second.persist(0, &second_loaded.snapshot, 1, &next),
            Err(NativePassportServerCapabilityStoreError::Unavailable(
                "Native Passport capability generation changed concurrently"
            ))
        ));
    }

    #[test]
    fn context_and_lifecycle_corruption_fail_closed() {
        let directory = TestDirectory::new("validation");
        let (store, loaded) = open_store(directory.path()).expect("open");

        let mut wrong_context = snapshot();
        wrong_context.capabilities[0].capability.environment = context("other-environment");

        assert!(matches!(
            store.persist(0, &loaded.snapshot, 1, &wrong_context),
            Err(NativePassportServerCapabilityStoreError::Corrupt(
                "stored Native Passport capability environment mismatch"
            ))
        ));

        let mut bad_lifecycle = snapshot();
        bad_lifecycle.capabilities[0].revoked_at_ms = Some(ISSUED_AT_MS + 1);

        assert!(matches!(
            store.persist(0, &loaded.snapshot, 1, &bad_lifecycle),
            Err(NativePassportServerCapabilityStoreError::Corrupt(
                "active Native Passport capability lifecycle is inconsistent"
            ))
        ));
    }

    #[test]
    fn unknown_snapshot_field_fails_closed() {
        let directory = TestDirectory::new("unknown-field");
        let (store, loaded) = open_store(directory.path()).expect("open");

        store
            .persist(0, &loaded.snapshot, 1, &snapshot())
            .expect("persist");

        let path = latest_snapshot(directory.path());

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("snapshot json");

        value["caller_authority"] = json!(true);

        fs::write(&path, serde_json::to_vec_pretty(&value).expect("encode")).expect("rewrite");

        assert!(matches!(
            open_store(directory.path()),
            Err(NativePassportServerCapabilityStoreError::Corrupt(
                "decode Native Passport capability snapshot"
            ))
        ));
    }

    #[test]
    fn persistence_source_has_no_route_secret_username_or_value_authority() {
        let source = include_str!("server_capability_store.rs");
        let implementation = source
            .split("\n#[cfg(test)]")
            .next()
            .expect("implementation");

        for forbidden in [
            "Router::",
            ".route(",
            "issue_device_session_challenge",
            "verify_device_session_proof",
            "claim_username(",
            "wallet.spend(",
            "ledger.write(",
            "device_private_key:",
            "root_private_key:",
            "recovery_phrase:",
            "raw_pin:",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "capability store gained forbidden authority pattern {forbidden}"
            );
        }
    }
}
