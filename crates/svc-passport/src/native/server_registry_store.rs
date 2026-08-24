//! RO:WHAT — Private durable immutable-generation storage for Native Passport server root/device registry state.
//! RO:WHY — Phase 10 needs restart-safe server identity state before registration/challenge routes may grant authority.
//! RO:INTERACTS — ron-proto canonical authorization DTOs, ron-auth strict verification, Native Passport-ID derivation, future proof-gated svc-passport registry runtime.
//! RO:INVARIANTS — raw persistence stays private; snapshots are strict/versioned/bounded/canonically ordered; generation publication is compare-and-swap; every device record binds to and verifies against one durable root record.
//! RO:METRICS — none; the future registry runtime owns metrics and redacted audit evidence.
//! RO:CONFIG — caller supplies a service-owned directory plus fixed network/environment context.
//! RO:SECURITY — stores public authority metadata only; no PIN, recovery material, private key, capability token, username authority, wallet mutation, or ledger mutation; corruption fails closed.
//! RO:TEST — focused unit tests in this file.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use ron_auth::native_passport::{
    verify_device_authorization_v1_strict, DeviceAuthorizationVerificationContextV1,
};
use ron_proto::{
    DeviceAuthorizationV1, DeviceIdV1 as ProtoDeviceIdV1,
    Ed25519PublicKeyHex as ProtoEd25519PublicKeyHex, NativePassportContextLabelV1,
    PassportIdV1 as ProtoPassportIdV1,
};
use serde::{Deserialize, Serialize};

use super::{
    dto::Ed25519PublicKeyHex as NativeEd25519PublicKeyHex,
    passport_id::derive_native_passport_id_v1,
};

const STORE_SCHEMA: &str = "svc-passport.native-passport-server-registry.v1";
const STORE_VERSION: u32 = 1;

const SNAPSHOT_PREFIX: &str = "registry-v1-";
const SNAPSHOT_SUFFIX: &str = ".json";
const TEMP_PREFIX: &str = ".registry-v1-";
const TEMP_SUFFIX: &str = ".tmp";
const GENERATION_DIGITS: usize = 20;

const MAX_SNAPSHOT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PASSPORTS: usize = 50_000;
const MAX_DEVICES: usize = 100_000;
const MAX_SNAPSHOTS: usize = 32;

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePassportServerRootRecordV1 {
    pub(super) passport_id: ProtoPassportIdV1,
    pub(super) root_public_key: ProtoEd25519PublicKeyHex,
    pub(super) root_key_epoch: u64,
    pub(super) registered_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum NativePassportServerDeviceStatusV1 {
    Authorized,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePassportServerDeviceRecordV1 {
    pub(super) passport_id: ProtoPassportIdV1,
    pub(super) device_id: ProtoDeviceIdV1,
    pub(super) authorization: DeviceAuthorizationV1,
    pub(super) status: NativePassportServerDeviceStatusV1,
    pub(super) registered_at_ms: u64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) revoked_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePassportServerRegistrySnapshotV1 {
    pub(super) passports: Vec<NativePassportServerRootRecordV1>,
    pub(super) devices: Vec<NativePassportServerDeviceRecordV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoadedNativePassportServerRegistryV1 {
    pub(super) generation: u64,
    pub(super) snapshot: NativePassportServerRegistrySnapshotV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativePassportServerRegistryStoreError {
    Unavailable(&'static str),
    Corrupt(&'static str),
}

#[derive(Debug)]
pub(super) struct NativePassportServerRegistrySnapshotStore {
    root: PathBuf,
    expected_network_id: NativePassportContextLabelV1,
    expected_environment: NativePassportContextLabelV1,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotFileV1 {
    schema: String,
    version: u32,
    generation: u64,
    snapshot: NativePassportServerRegistrySnapshotV1,
}

fn unavailable(reason: &'static str) -> NativePassportServerRegistryStoreError {
    NativePassportServerRegistryStoreError::Unavailable(reason)
}

fn corrupt(reason: &'static str) -> NativePassportServerRegistryStoreError {
    NativePassportServerRegistryStoreError::Corrupt(reason)
}

impl NativePassportServerRegistrySnapshotStore {
    pub(super) fn open(
        root: impl AsRef<Path>,
        expected_network_id: NativePassportContextLabelV1,
        expected_environment: NativePassportContextLabelV1,
    ) -> Result<(Self, LoadedNativePassportServerRegistryV1), NativePassportServerRegistryStoreError>
    {
        let root = root.as_ref().to_path_buf();

        ensure_store_directory(&root)?;

        let store = Self {
            root,
            expected_network_id,
            expected_environment,
        };

        let snapshots = store.scan_snapshot_paths(true)?;

        if snapshots.len() > MAX_SNAPSHOTS {
            return Err(corrupt(
                "too many durable Native Passport registry snapshots",
            ));
        }

        let mut loaded = LoadedNativePassportServerRegistryV1 {
            generation: 0,
            snapshot: NativePassportServerRegistrySnapshotV1::default(),
        };

        for (generation, path) in snapshots {
            let file = read_snapshot(
                &path,
                generation,
                &store.expected_network_id,
                &store.expected_environment,
            )?;

            loaded = LoadedNativePassportServerRegistryV1 {
                generation: file.generation,
                snapshot: file.snapshot,
            };
        }

        Ok((store, loaded))
    }

    pub(super) fn persist(
        &self,
        expected_generation: u64,
        expected_snapshot: &NativePassportServerRegistrySnapshotV1,
        next_generation: u64,
        next_snapshot: &NativePassportServerRegistrySnapshotV1,
    ) -> Result<(), NativePassportServerRegistryStoreError> {
        let required_generation = expected_generation
            .checked_add(1)
            .ok_or_else(|| unavailable("Native Passport registry generation overflow"))?;

        if next_generation != required_generation {
            return Err(corrupt(
                "non-sequential Native Passport registry generation",
            ));
        }

        validate_snapshot(
            next_snapshot,
            &self.expected_network_id,
            &self.expected_environment,
        )?;

        let mut snapshots = self.scan_snapshot_paths(true)?;

        let disk_generation = snapshots
            .last()
            .map(|(generation, _)| *generation)
            .unwrap_or(0);

        if disk_generation != expected_generation {
            return Err(unavailable(
                "Native Passport registry generation changed concurrently",
            ));
        }

        if expected_generation == 0 {
            if expected_snapshot != &NativePassportServerRegistrySnapshotV1::default() {
                return Err(corrupt(
                    "memory contains Native Passport registry state without a durable generation",
                ));
            }
        } else {
            let (_, latest_path) = snapshots
                .last()
                .ok_or_else(|| corrupt("Native Passport registry generation missing"))?;

            let latest = read_snapshot(
                latest_path,
                expected_generation,
                &self.expected_network_id,
                &self.expected_environment,
            )?;

            if &latest.snapshot != expected_snapshot {
                return Err(corrupt(
                    "durable Native Passport registry disagrees with memory",
                ));
            }
        }

        while snapshots.len() >= MAX_SNAPSHOTS {
            let (_, oldest) = snapshots.remove(0);

            fs::remove_file(oldest)
                .map_err(|_| unavailable("prune old Native Passport registry snapshot"))?;
        }

        let envelope = SnapshotFileV1 {
            schema: STORE_SCHEMA.to_owned(),
            version: STORE_VERSION,
            generation: next_generation,
            snapshot: next_snapshot.clone(),
        };

        let bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|_| unavailable("serialize Native Passport registry snapshot"))?;

        if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
            return Err(unavailable(
                "Native Passport registry snapshot byte bound exceeded",
            ));
        }

        self.publish_snapshot(next_generation, &bytes)
    }

    fn publish_snapshot(
        &self,
        generation: u64,
        bytes: &[u8],
    ) -> Result<(), NativePassportServerRegistryStoreError> {
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
            .map_err(|_| unavailable("create temporary Native Passport registry snapshot"))?;

        if file.write_all(bytes).is_err() {
            let _ = fs::remove_file(&temp_path);

            return Err(unavailable(
                "write temporary Native Passport registry snapshot",
            ));
        }

        if file.sync_all().is_err() {
            let _ = fs::remove_file(&temp_path);

            return Err(unavailable(
                "sync temporary Native Passport registry snapshot",
            ));
        }

        drop(file);

        if final_path.exists() {
            let _ = fs::remove_file(&temp_path);

            return Err(unavailable(
                "Native Passport registry generation already exists",
            ));
        }

        if fs::hard_link(&temp_path, &final_path).is_err() {
            let _ = fs::remove_file(&temp_path);

            return Err(unavailable(
                "publish immutable Native Passport registry snapshot",
            ));
        }

        let _ = fs::remove_file(&temp_path);

        sync_directory_best_effort(&self.root);

        Ok(())
    }

    fn scan_snapshot_paths(
        &self,
        clean_temps: bool,
    ) -> Result<Vec<(u64, PathBuf)>, NativePassportServerRegistryStoreError> {
        let metadata = fs::symlink_metadata(&self.root)
            .map_err(|_| unavailable("inspect Native Passport registry directory"))?;

        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(corrupt(
                "Native Passport registry root is not a real directory",
            ));
        }

        let entries = fs::read_dir(&self.root)
            .map_err(|_| unavailable("read Native Passport registry directory"))?;

        let mut snapshots = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|_| unavailable("read Native Passport registry entry"))?;

            let file_type = entry
                .file_type()
                .map_err(|_| unavailable("inspect Native Passport registry entry"))?;

            let name = entry.file_name();

            let name = name
                .to_str()
                .ok_or_else(|| corrupt("Native Passport registry contains non-UTF8 entry"))?;

            if name.starts_with(TEMP_PREFIX) && name.ends_with(TEMP_SUFFIX) {
                if file_type.is_symlink() || !file_type.is_file() {
                    return Err(corrupt(
                        "Native Passport registry temp entry has invalid type",
                    ));
                }

                if clean_temps {
                    fs::remove_file(entry.path()).map_err(|_| {
                        unavailable("remove stale Native Passport registry temp file")
                    })?;
                }

                continue;
            }

            let generation = parse_snapshot_filename(name)
                .ok_or_else(|| corrupt("unexpected entry in Native Passport registry directory"))?;

            if file_type.is_symlink() || !file_type.is_file() {
                return Err(corrupt(
                    "Native Passport registry snapshot is not a regular file",
                ));
            }

            snapshots.push((generation, entry.path()));
        }

        snapshots.sort_by_key(|(generation, _)| *generation);

        for pair in snapshots.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(corrupt("duplicate Native Passport registry generation"));
            }
        }

        Ok(snapshots)
    }
}

fn validate_snapshot(
    snapshot: &NativePassportServerRegistrySnapshotV1,
    expected_network_id: &NativePassportContextLabelV1,
    expected_environment: &NativePassportContextLabelV1,
) -> Result<(), NativePassportServerRegistryStoreError> {
    if snapshot.passports.len() > MAX_PASSPORTS {
        return Err(corrupt("Native Passport registry Passport bound exceeded"));
    }

    if snapshot.devices.len() > MAX_DEVICES {
        return Err(corrupt("Native Passport registry device bound exceeded"));
    }

    for pair in snapshot.passports.windows(2) {
        if pair[0].passport_id.as_str() >= pair[1].passport_id.as_str() {
            return Err(corrupt(
                "Native Passport registry Passport records are not strictly sorted and unique",
            ));
        }
    }

    for record in &snapshot.passports {
        validate_root_record(record)?;
    }

    for pair in snapshot.devices.windows(2) {
        let left = (pair[0].passport_id.as_str(), pair[0].device_id.as_str());

        let right = (pair[1].passport_id.as_str(), pair[1].device_id.as_str());

        if left >= right {
            return Err(corrupt(
                "Native Passport registry device records are not strictly sorted and unique",
            ));
        }
    }

    let roots = snapshot
        .passports
        .iter()
        .map(|record| (record.passport_id.as_str(), record))
        .collect::<BTreeMap<_, _>>();

    for record in &snapshot.devices {
        let root = roots.get(record.passport_id.as_str()).ok_or_else(|| {
            corrupt("Native Passport registry device has no matching root record")
        })?;

        validate_device_record(record, root, expected_network_id, expected_environment)?;
    }

    Ok(())
}

fn validate_root_record(
    record: &NativePassportServerRootRecordV1,
) -> Result<(), NativePassportServerRegistryStoreError> {
    if record.registered_at_ms == 0 {
        return Err(corrupt("Native Passport root registration time is invalid"));
    }

    let native_root = NativeEd25519PublicKeyHex::parse(record.root_public_key.as_str())
        .map_err(|_| corrupt("Native Passport root public key is invalid"))?;

    let derived = derive_native_passport_id_v1(&native_root)
        .map_err(|_| corrupt("Native Passport root Passport ID derivation failed"))?;

    if derived.as_str() != record.passport_id.as_str() {
        return Err(corrupt(
            "Native Passport ID/root public-key binding mismatch",
        ));
    }

    Ok(())
}

fn validate_device_record(
    record: &NativePassportServerDeviceRecordV1,
    root: &NativePassportServerRootRecordV1,
    expected_network_id: &NativePassportContextLabelV1,
    expected_environment: &NativePassportContextLabelV1,
) -> Result<(), NativePassportServerRegistryStoreError> {
    if record.authorization.passport_id.as_str() != record.passport_id.as_str()
        || record.authorization.device_id.as_str() != record.device_id.as_str()
    {
        return Err(corrupt("Native Passport device record binding mismatch"));
    }

    if record.registered_at_ms == 0
        || record.registered_at_ms < record.authorization.issued_at_ms
        || record.registered_at_ms < root.registered_at_ms
    {
        return Err(corrupt(
            "Native Passport device registration time is invalid",
        ));
    }

    match (record.status, record.revoked_at_ms) {
        (NativePassportServerDeviceStatusV1::Authorized, None) => {}

        (NativePassportServerDeviceStatusV1::Revoked, Some(revoked_at_ms))
            if revoked_at_ms >= record.registered_at_ms => {}

        _ => {
            return Err(corrupt(
                "Native Passport device revocation state is inconsistent",
            ));
        }
    }

    /*
     * Persistence validation uses the authorization's own issuance instant.
     * This proves durable cryptographic/context integrity without incorrectly
     * treating an old-but-legitimate revoked record as corrupt after expiry.
     * Live-use expiry is checked again by the future registry/proof runtime.
     */
    verify_device_authorization_v1_strict(
        &record.authorization,
        DeviceAuthorizationVerificationContextV1 {
            trusted_passport_id: &root.passport_id,
            trusted_root_public_key: &root.root_public_key,
            trusted_root_key_epoch: root.root_key_epoch,
            expected_network_id,
            expected_environment,
            now_ms: record.authorization.issued_at_ms,
            max_clock_skew_ms: 0,
        },
    )
    .map_err(|_| corrupt("Native Passport durable device authorization verification failed"))?;

    Ok(())
}

fn ensure_store_directory(root: &Path) -> Result<(), NativePassportServerRegistryStoreError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(corrupt(
                    "Native Passport registry root is not a real directory",
                ));
            }
        }

        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(root)
                .map_err(|_| unavailable("create Native Passport registry directory"))?;
        }

        Err(_) => {
            return Err(unavailable("inspect Native Passport registry directory"));
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| unavailable("secure Native Passport registry directory"))?;
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
    expected_network_id: &NativePassportContextLabelV1,
    expected_environment: &NativePassportContextLabelV1,
) -> Result<SnapshotFileV1, NativePassportServerRegistryStoreError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| unavailable("inspect Native Passport registry snapshot"))?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(corrupt(
            "Native Passport registry snapshot is not a regular file",
        ));
    }

    if metadata.len() == 0 || metadata.len() > MAX_SNAPSHOT_BYTES {
        return Err(corrupt("Native Passport registry snapshot size is invalid"));
    }

    let bytes =
        fs::read(path).map_err(|_| unavailable("read Native Passport registry snapshot"))?;

    if bytes.len() as u64 != metadata.len() {
        return Err(corrupt(
            "Native Passport registry snapshot changed while reading",
        ));
    }

    let file = serde_json::from_slice::<SnapshotFileV1>(&bytes)
        .map_err(|_| corrupt("Native Passport registry snapshot JSON is invalid"))?;

    if file.schema != STORE_SCHEMA {
        return Err(corrupt(
            "Native Passport registry snapshot schema is unsupported",
        ));
    }

    if file.version != STORE_VERSION {
        return Err(corrupt(
            "Native Passport registry snapshot version is unsupported",
        ));
    }

    if file.generation != expected_generation {
        return Err(corrupt(
            "Native Passport registry snapshot generation does not match filename",
        ));
    }

    validate_snapshot(&file.snapshot, expected_network_id, expected_environment)?;

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
        DeviceAuthorizationNonceV1, DeviceAuthorizationScopeCeilingV1,
        DeviceAuthorizationSigningPayloadV1, DeviceClassV1, Ed25519SignatureV1,
        NativePassportScopeV1, DEVICE_AUTHORIZATION_V1_VERSION,
    };
    use serde_json::json;

    use crate::native::{
        derive_native_device_public_identity_v1, derive_native_recovery_public_identity_v1,
        sign_native_recovery_device_authorization_v1, NativeSecretBytes,
    };

    use super::*;

    const ISSUED_AT_MS: u64 = 1_720_000_000_000;
    const ROOT_REGISTERED_AT_MS: u64 = ISSUED_AT_MS + 1;
    const DEVICE_REGISTERED_AT_MS: u64 = ISSUED_AT_MS + 2;
    const ROOT_KEY_EPOCH: u64 = 7;

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
                "svc-passport-native-registry-{label}-{}-{stamp}",
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

    fn network() -> NativePassportContextLabelV1 {
        NativePassportContextLabelV1::parse("rustyonions-devnet").expect("network")
    }

    fn environment() -> NativePassportContextLabelV1 {
        NativePassportContextLabelV1::parse("private-beta").expect("environment")
    }

    fn recovery_factor(byte: u8) -> NativeSecretBytes {
        NativeSecretBytes::new(vec![byte; 32]).expect("nonphysical recovery factor")
    }

    fn device_seed() -> NativeSecretBytes {
        NativeSecretBytes::new(vec![0x42; 32]).expect("nonphysical device seed")
    }

    fn scope(value: &str) -> NativePassportScopeV1 {
        NativePassportScopeV1::parse(value).expect("scope")
    }

    fn scope_ceiling() -> DeviceAuthorizationScopeCeilingV1 {
        DeviceAuthorizationScopeCeilingV1::new(vec![
            scope("capability.revoke_self"),
            scope("catalog.read"),
            scope("content.read"),
            scope("identity.read"),
        ])
        .expect("canonical scopes")
    }

    fn root_record(factor_byte: u8, registered_at_ms: u64) -> NativePassportServerRootRecordV1 {
        let factor = recovery_factor(factor_byte);

        let root = derive_native_recovery_public_identity_v1(&factor).expect("public root");

        NativePassportServerRootRecordV1 {
            passport_id: ProtoPassportIdV1::parse(root.passport_id.as_str()).expect("Passport ID"),

            root_public_key: ProtoEd25519PublicKeyHex::parse(root.root_public_key.as_str())
                .expect("root public key"),

            root_key_epoch: ROOT_KEY_EPOCH,
            registered_at_ms,
        }
    }

    fn valid_snapshot() -> NativePassportServerRegistrySnapshotV1 {
        let factor = recovery_factor(0x22);

        let root = derive_native_recovery_public_identity_v1(&factor).expect("public root");

        let device =
            derive_native_device_public_identity_v1(&device_seed()).expect("device identity");

        let payload = DeviceAuthorizationSigningPayloadV1 {
            version: DEVICE_AUTHORIZATION_V1_VERSION,

            network_id: network(),
            environment: environment(),

            passport_id: ProtoPassportIdV1::parse(root.passport_id.as_str()).expect("Passport ID"),

            root_key_epoch: ROOT_KEY_EPOCH,

            device_id: ProtoDeviceIdV1::parse(device.device_id.as_str()).expect("Device ID"),

            device_public_key: ProtoEd25519PublicKeyHex::parse(device.device_public_key.as_str())
                .expect("device public key"),

            device_class: DeviceClassV1::RootAdminDesktop,

            authorized_scope_ceiling: scope_ceiling(),

            authorization_nonce: DeviceAuthorizationNonceV1::from_bytes([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f,
            ]),

            issued_at_ms: ISSUED_AT_MS,
            expires_at_ms: None,
        };

        let authorization = sign_native_recovery_device_authorization_v1(&factor, payload)
            .expect("signed authorization");

        NativePassportServerRegistrySnapshotV1 {
            passports: vec![NativePassportServerRootRecordV1 {
                passport_id: ProtoPassportIdV1::parse(root.passport_id.as_str())
                    .expect("Passport ID"),

                root_public_key: ProtoEd25519PublicKeyHex::parse(root.root_public_key.as_str())
                    .expect("root public key"),

                root_key_epoch: ROOT_KEY_EPOCH,
                registered_at_ms: ROOT_REGISTERED_AT_MS,
            }],

            devices: vec![NativePassportServerDeviceRecordV1 {
                passport_id: authorization.passport_id.clone(),
                device_id: authorization.device_id.clone(),
                authorization,

                status: NativePassportServerDeviceStatusV1::Authorized,

                registered_at_ms: DEVICE_REGISTERED_AT_MS,

                revoked_at_ms: None,
            }],
        }
    }

    fn open_store(
        root: &Path,
    ) -> Result<
        (
            NativePassportServerRegistrySnapshotStore,
            LoadedNativePassportServerRegistryV1,
        ),
        NativePassportServerRegistryStoreError,
    > {
        NativePassportServerRegistrySnapshotStore::open(root, network(), environment())
    }

    fn latest_snapshot(root: &Path) -> PathBuf {
        let mut snapshots = fs::read_dir(root)
            .expect("read registry directory")
            .map(|entry| entry.expect("registry entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| {
                        name.starts_with(SNAPSHOT_PREFIX) && name.ends_with(SNAPSHOT_SUFFIX)
                    })
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        snapshots.sort();

        snapshots.pop().expect("durable registry snapshot")
    }

    #[test]
    fn empty_store_opens_and_raw_persistence_stays_private() {
        let directory = TestDirectory::new("empty");

        let (_store, loaded) = open_store(&directory.path).expect("open empty store");

        assert_eq!(loaded.generation, 0);

        assert_eq!(
            loaded.snapshot,
            NativePassportServerRegistrySnapshotV1::default(),
        );

        let native_mod = include_str!("mod.rs");

        assert!(native_mod.contains("mod server_registry_store;"),);

        assert!(!native_mod.contains("pub mod server_registry_store;"),);
    }

    #[test]
    fn signed_root_and_device_survive_restart_with_private_permissions() {
        let directory = TestDirectory::new("roundtrip");

        let (store, loaded) = open_store(&directory.path).expect("open store");

        let snapshot = valid_snapshot();

        store
            .persist(loaded.generation, &loaded.snapshot, 1, &snapshot)
            .expect("persist registry");

        let (_reopened, reloaded) = open_store(&directory.path).expect("reopen registry");

        assert_eq!(reloaded.generation, 1);
        assert_eq!(reloaded.snapshot, snapshot);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                fs::metadata(&directory.path)
                    .expect("directory metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o700,
            );

            assert_eq!(
                fs::metadata(latest_snapshot(&directory.path))
                    .expect("snapshot metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600,
            );
        }
    }

    #[test]
    fn strict_signature_corruption_fails_closed_after_restart() {
        let directory = TestDirectory::new("signature-corruption");

        let (store, loaded) = open_store(&directory.path).expect("open store");

        let snapshot = valid_snapshot();

        store
            .persist(0, &loaded.snapshot, 1, &snapshot)
            .expect("persist registry");

        let path = latest_snapshot(&directory.path);

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read snapshot"))
                .expect("snapshot JSON");

        value["snapshot"]["devices"][0]["authorization"]["root_signature"] =
            serde_json::to_value(Ed25519SignatureV1::from_bytes([0x11; 64]))
                .expect("signature JSON");

        fs::write(
            &path,
            serde_json::to_vec_pretty(&value).expect("serialize corruption"),
        )
        .expect("write corruption");

        assert!(matches!(
            open_store(&directory.path),
            Err(NativePassportServerRegistryStoreError::Corrupt(
                "Native Passport durable device authorization verification failed"
            ))
        ));
    }

    #[test]
    fn binding_ordering_and_revocation_inconsistency_reject_before_commit() {
        let directory = TestDirectory::new("validation");

        let (store, loaded) = open_store(&directory.path).expect("open store");

        let mut wrong_root = valid_snapshot();

        wrong_root.passports[0].root_public_key =
            root_record(0x33, ROOT_REGISTERED_AT_MS).root_public_key;

        assert!(matches!(
            store.persist(0, &loaded.snapshot, 1, &wrong_root,),
            Err(NativePassportServerRegistryStoreError::Corrupt(
                "Native Passport ID/root public-key binding mismatch"
            ))
        ));

        let mut roots = vec![
            root_record(0x22, ROOT_REGISTERED_AT_MS),
            root_record(0x33, ROOT_REGISTERED_AT_MS),
        ];

        roots.sort_by(|left, right| left.passport_id.as_str().cmp(right.passport_id.as_str()));

        roots.reverse();

        let unsorted = NativePassportServerRegistrySnapshotV1 {
            passports: roots,
            devices: Vec::new(),
        };

        assert!(matches!(
            store.persist(0, &loaded.snapshot, 1, &unsorted,),
            Err(NativePassportServerRegistryStoreError::Corrupt(
                "Native Passport registry Passport records are not strictly sorted and unique"
            ))
        ));

        let mut bad_revocation = valid_snapshot();

        bad_revocation.devices[0].status = NativePassportServerDeviceStatusV1::Revoked;

        assert!(matches!(
            store.persist(0, &loaded.snapshot, 1, &bad_revocation,),
            Err(NativePassportServerRegistryStoreError::Corrupt(
                "Native Passport device revocation state is inconsistent"
            ))
        ));

        assert!(fs::read_dir(&directory.path)
            .expect("validation directory")
            .next()
            .is_none(),);
    }

    #[test]
    fn compare_and_swap_generation_rejects_lost_update() {
        let directory = TestDirectory::new("cas");

        let (first, first_loaded) = open_store(&directory.path).expect("first open");

        let (second, second_loaded) = open_store(&directory.path).expect("second open");

        let snapshot = valid_snapshot();

        first
            .persist(0, &first_loaded.snapshot, 1, &snapshot)
            .expect("first writer");

        assert!(matches!(
            second.persist(0, &second_loaded.snapshot, 1, &snapshot,),
            Err(NativePassportServerRegistryStoreError::Unavailable(
                "Native Passport registry generation changed concurrently"
            ))
        ));
    }

    #[test]
    fn unknown_snapshot_field_fails_closed() {
        let directory = TestDirectory::new("unknown-field");

        let (store, loaded) = open_store(&directory.path).expect("open store");

        let snapshot = valid_snapshot();

        store
            .persist(0, &loaded.snapshot, 1, &snapshot)
            .expect("persist registry");

        let path = latest_snapshot(&directory.path);

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read snapshot"))
                .expect("snapshot JSON");

        value
            .as_object_mut()
            .expect("snapshot object")
            .insert("unexpected".to_owned(), json!(true));

        fs::write(
            &path,
            serde_json::to_vec_pretty(&value).expect("serialize corruption"),
        )
        .expect("write corruption");

        assert!(matches!(
            open_store(&directory.path),
            Err(NativePassportServerRegistryStoreError::Corrupt(
                "Native Passport registry snapshot JSON is invalid"
            ))
        ));
    }

    #[test]
    fn valid_revocation_state_survives_restart() {
        let directory = TestDirectory::new("revocation");

        let (store, loaded) = open_store(&directory.path).expect("open store");

        let mut snapshot = valid_snapshot();

        snapshot.devices[0].status = NativePassportServerDeviceStatusV1::Revoked;

        snapshot.devices[0].revoked_at_ms = Some(DEVICE_REGISTERED_AT_MS + 1);

        store
            .persist(0, &loaded.snapshot, 1, &snapshot)
            .expect("persist revoked state");

        let (_reopened, reloaded) = open_store(&directory.path).expect("reopen revoked registry");

        assert_eq!(reloaded.snapshot, snapshot);
    }

    #[test]
    fn persistence_source_has_no_route_capability_namespace_or_secret_authority() {
        let (source, _tests) = include_str!("server_registry_store.rs")
            .split_once("\n#[cfg(test)]")
            .expect("implementation source precedes test module");

        for forbidden in [
            "Router::new",
            ".route(",
            "issue_capability(",
            "claim_username(",
            "wallet.spend(",
            "ledger.write(",
            "root_private_key:",
            "device_private_key:",
            "recovery_phrase:",
            "raw_pin:",
        ] {
            assert!(
                !source.contains(forbidden),
                "raw persistence gained forbidden authority pattern {forbidden}",
            );
        }
    }
}
