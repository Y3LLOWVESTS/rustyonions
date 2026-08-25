//! RO:WHAT — Private durable one-time replay storage for accepted Native Passport protected-request nonces.
//! RO:WHY — CN-4 protected username mutation must reject a successfully used PassportRequestProofV1 after restart without making the capability, gateway, or profile layer replay authority.
//! RO:INTERACTS — ron-proto CapabilityIdV1/B3DigestHex request-proof identifiers, ron-auth V1 request-proof freshness ceiling, future server_request_proof_runtime, and service-owned filesystem state.
//! RO:INVARIANTS — replay identity is the exact capability ID + request nonce pair; publication is immutable and atomic; malformed/tampered state fails closed; records remain bounded and are retained across the full two-sided request-proof freshness window.
//! RO:METRICS — none; the future protected-request runtime owns bounded outcome metrics without identity-bearing labels.
//! RO:CONFIG — caller supplies a dedicated replay directory and a retention interval no shorter than twice the frozen ron-auth request-proof clock-skew ceiling because V1 admits both past and future skew.
//! RO:SECURITY — stores public replay metadata only; no DeviceKey/root secret, signature creation, PIN, RecoveryRoot, capability issuance, username mutation, wallet, ledger, HTTP, or Tauri authority.
//! RO:TEST — focused unit tests below prove restart replay rejection, capability separation, retention, corruption failure, bounds, and private permissions.

#![forbid(unsafe_code)]

use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use ron_auth::native_passport::PASSPORT_REQUEST_PROOF_V1_MAX_CLOCK_SKEW_MS;

const MIN_REQUEST_REPLAY_RETENTION_MS: u64 = PASSPORT_REQUEST_PROOF_V1_MAX_CLOCK_SKEW_MS * 2;
use ron_proto::{B3DigestHex, CapabilityIdV1};
use serde::{Deserialize, Serialize};

const CN4_REQUEST_PROOF_REPLAY_STORE_V1: &str =
    "svc-passport.native-passport-request-proof-replay.v1";

const STORE_VERSION: u32 = 1;

const RECORD_PREFIX: &str = "request-replay-v1-";
const RECORD_SUFFIX: &str = ".json";
const TEMP_PREFIX: &str = ".request-replay-v1-";
const TEMP_SUFFIX: &str = ".tmp";

const RECORD_KEY_DOMAIN: &str = "rustyonions.native-passport.request-proof-replay-key.v1";

const MAX_RECORD_BYTES: u64 = 4 * 1024;
const MAX_RECORDS: usize = 65_536;
const MAX_DIRECTORY_ENTRIES: usize = MAX_RECORDS + 256;

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativePassportRequestReplayRecordV1 {
    schema: String,
    version: u32,
    capability_id: CapabilityIdV1,
    request_nonce: B3DigestHex,
    consumed_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativePassportRequestReplayStoreError {
    InvalidConfig,
    InvalidTrustedTime,
    AlreadyConsumed,
    StateFull,
    Unavailable(&'static str),
    Corrupt(&'static str),
}

#[derive(Debug)]
pub(super) struct NativePassportRequestReplayStore {
    root: PathBuf,
    retention_ms: u64,
}

impl NativePassportRequestReplayStore {
    pub(super) fn open(
        root: impl AsRef<Path>,
        retention_ms: u64,
        now_ms: u64,
    ) -> Result<Self, NativePassportRequestReplayStoreError> {
        if retention_ms < MIN_REQUEST_REPLAY_RETENTION_MS {
            return Err(NativePassportRequestReplayStoreError::InvalidConfig);
        }

        if now_ms == 0 {
            return Err(NativePassportRequestReplayStoreError::InvalidTrustedTime);
        }

        let root = root.as_ref().to_path_buf();

        ensure_private_directory(&root)?;

        let store = Self { root, retention_ms };

        store.prune_and_validate(now_ms)?;

        Ok(store)
    }

    pub(super) fn consume(
        &self,
        capability_id: &CapabilityIdV1,
        request_nonce: &B3DigestHex,
        consumed_at_ms: u64,
    ) -> Result<(), NativePassportRequestReplayStoreError> {
        if consumed_at_ms == 0 {
            return Err(NativePassportRequestReplayStoreError::InvalidTrustedTime);
        }

        let active_count = self.prune_and_validate(consumed_at_ms)?;

        if active_count >= MAX_RECORDS {
            return Err(NativePassportRequestReplayStoreError::StateFull);
        }

        let key = record_key_hex(capability_id, request_nonce);

        let final_path = self
            .root
            .join(format!("{RECORD_PREFIX}{key}{RECORD_SUFFIX}"));

        if final_path.exists() {
            validate_existing_binding(&final_path, capability_id, request_nonce, consumed_at_ms)?;

            return Err(NativePassportRequestReplayStoreError::AlreadyConsumed);
        }

        let record = NativePassportRequestReplayRecordV1 {
            schema: CN4_REQUEST_PROOF_REPLAY_STORE_V1.to_owned(),
            version: STORE_VERSION,
            capability_id: capability_id.clone(),
            request_nonce: request_nonce.clone(),
            consumed_at_ms,
        };

        let encoded = serde_json::to_vec(&record).map_err(|_| {
            NativePassportRequestReplayStoreError::Unavailable(
                "encode Native Passport request replay record",
            )
        })?;

        if encoded.len() as u64 > MAX_RECORD_BYTES {
            return Err(NativePassportRequestReplayStoreError::Corrupt(
                "Native Passport request replay record exceeds size bound",
            ));
        }

        let temp_id = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);

        let temp_path = self.root.join(format!(
            "{TEMP_PREFIX}{}-{temp_id}{TEMP_SUFFIX}",
            std::process::id(),
        ));

        let mut options = OpenOptions::new();

        options.write(true).create_new(true);

        #[cfg(unix)]
        options.mode(0o600);

        let mut file = options.open(&temp_path).map_err(|_| {
            NativePassportRequestReplayStoreError::Unavailable(
                "create Native Passport request replay temp file",
            )
        })?;

        if let Err(_error) = file.write_all(&encoded) {
            let _ = fs::remove_file(&temp_path);

            return Err(NativePassportRequestReplayStoreError::Unavailable(
                "write Native Passport request replay temp file",
            ));
        }

        if let Err(_error) = file.sync_all() {
            let _ = fs::remove_file(&temp_path);

            return Err(NativePassportRequestReplayStoreError::Unavailable(
                "sync Native Passport request replay temp file",
            ));
        }

        drop(file);

        match fs::hard_link(&temp_path, &final_path) {
            Ok(()) => {
                let _ = fs::remove_file(&temp_path);

                sync_directory_best_effort(&self.root)?;

                let count_after_publish = self.prune_and_validate(consumed_at_ms)?;

                if count_after_publish > MAX_RECORDS {
                    let _ = fs::remove_file(&final_path);
                    let _ = sync_directory_best_effort(&self.root);

                    return Err(NativePassportRequestReplayStoreError::StateFull);
                }

                Ok(())
            }

            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temp_path);

                validate_existing_binding(
                    &final_path,
                    capability_id,
                    request_nonce,
                    consumed_at_ms,
                )?;

                Err(NativePassportRequestReplayStoreError::AlreadyConsumed)
            }

            Err(_error) => {
                let _ = fs::remove_file(&temp_path);

                Err(NativePassportRequestReplayStoreError::Unavailable(
                    "publish Native Passport request replay record",
                ))
            }
        }
    }

    fn prune_and_validate(
        &self,
        now_ms: u64,
    ) -> Result<usize, NativePassportRequestReplayStoreError> {
        if now_ms == 0 {
            return Err(NativePassportRequestReplayStoreError::InvalidTrustedTime);
        }

        let entries = fs::read_dir(&self.root).map_err(|_| {
            NativePassportRequestReplayStoreError::Unavailable(
                "read Native Passport request replay directory",
            )
        })?;

        let mut seen_entries = 0usize;
        let mut active_records = 0usize;
        let mut removed_any = false;

        for entry in entries {
            let entry = entry.map_err(|_| {
                NativePassportRequestReplayStoreError::Unavailable(
                    "read Native Passport request replay directory entry",
                )
            })?;

            seen_entries = seen_entries
                .checked_add(1)
                .ok_or(NativePassportRequestReplayStoreError::StateFull)?;

            if seen_entries > MAX_DIRECTORY_ENTRIES {
                return Err(NativePassportRequestReplayStoreError::StateFull);
            }

            let path = entry.path();

            let metadata = fs::symlink_metadata(&path).map_err(|_| {
                NativePassportRequestReplayStoreError::Unavailable(
                    "stat Native Passport request replay entry",
                )
            })?;

            if !metadata.file_type().is_file() {
                return Err(NativePassportRequestReplayStoreError::Corrupt(
                    "unexpected Native Passport request replay entry type",
                ));
            }

            let file_name = entry.file_name().into_string().map_err(|_| {
                NativePassportRequestReplayStoreError::Corrupt(
                    "non-UTF8 Native Passport request replay filename",
                )
            })?;

            if file_name.starts_with(TEMP_PREFIX) && file_name.ends_with(TEMP_SUFFIX) {
                fs::remove_file(&path).map_err(|_| {
                    NativePassportRequestReplayStoreError::Unavailable(
                        "remove stale Native Passport request replay temp file",
                    )
                })?;

                removed_any = true;
                continue;
            }

            if !file_name.starts_with(RECORD_PREFIX) || !file_name.ends_with(RECORD_SUFFIX) {
                return Err(NativePassportRequestReplayStoreError::Corrupt(
                    "unexpected file in Native Passport request replay directory",
                ));
            }

            let record = read_record(&path)?;

            if record.consumed_at_ms == 0 || record.consumed_at_ms > now_ms {
                return Err(NativePassportRequestReplayStoreError::Corrupt(
                    "Native Passport request replay time is invalid",
                ));
            }

            let expected_key = record_key_hex(&record.capability_id, &record.request_nonce);

            let expected_name = format!("{RECORD_PREFIX}{expected_key}{RECORD_SUFFIX}");

            if file_name != expected_name {
                return Err(NativePassportRequestReplayStoreError::Corrupt(
                    "Native Passport request replay filename binding mismatch",
                ));
            }

            let age_ms = now_ms - record.consumed_at_ms;

            if age_ms > self.retention_ms {
                fs::remove_file(&path).map_err(|_| {
                    NativePassportRequestReplayStoreError::Unavailable(
                        "prune expired Native Passport request replay record",
                    )
                })?;

                removed_any = true;
                continue;
            }

            active_records = active_records
                .checked_add(1)
                .ok_or(NativePassportRequestReplayStoreError::StateFull)?;

            if active_records > MAX_RECORDS {
                return Err(NativePassportRequestReplayStoreError::StateFull);
            }
        }

        if removed_any {
            sync_directory_best_effort(&self.root)?;
        }

        Ok(active_records)
    }
}

fn validate_existing_binding(
    path: &Path,
    capability_id: &CapabilityIdV1,
    request_nonce: &B3DigestHex,
    now_ms: u64,
) -> Result<(), NativePassportRequestReplayStoreError> {
    let record = read_record(path)?;

    if record.capability_id != *capability_id || record.request_nonce != *request_nonce {
        return Err(NativePassportRequestReplayStoreError::Corrupt(
            "Native Passport request replay key collision",
        ));
    }

    if record.consumed_at_ms == 0 || record.consumed_at_ms > now_ms {
        return Err(NativePassportRequestReplayStoreError::Corrupt(
            "Native Passport request replay time is invalid",
        ));
    }

    Ok(())
}

fn read_record(
    path: &Path,
) -> Result<NativePassportRequestReplayRecordV1, NativePassportRequestReplayStoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        NativePassportRequestReplayStoreError::Unavailable(
            "stat Native Passport request replay record",
        )
    })?;

    if !metadata.file_type().is_file() {
        return Err(NativePassportRequestReplayStoreError::Corrupt(
            "Native Passport request replay record is not a regular file",
        ));
    }

    if metadata.len() > MAX_RECORD_BYTES {
        return Err(NativePassportRequestReplayStoreError::Corrupt(
            "Native Passport request replay record exceeds size bound",
        ));
    }

    #[cfg(unix)]
    {
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(NativePassportRequestReplayStoreError::Corrupt(
                "Native Passport request replay record permissions are too broad",
            ));
        }
    }

    let mut file = File::open(path).map_err(|_| {
        NativePassportRequestReplayStoreError::Unavailable(
            "open Native Passport request replay record",
        )
    })?;

    let mut bytes =
        Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(MAX_RECORD_BYTES as usize));

    file.read_to_end(&mut bytes).map_err(|_| {
        NativePassportRequestReplayStoreError::Unavailable(
            "read Native Passport request replay record",
        )
    })?;

    let record: NativePassportRequestReplayRecordV1 =
        serde_json::from_slice(&bytes).map_err(|_| {
            NativePassportRequestReplayStoreError::Corrupt(
                "decode Native Passport request replay record",
            )
        })?;

    if record.schema != CN4_REQUEST_PROOF_REPLAY_STORE_V1 || record.version != STORE_VERSION {
        return Err(NativePassportRequestReplayStoreError::Corrupt(
            "Native Passport request replay schema/version mismatch",
        ));
    }

    Ok(record)
}

fn ensure_private_directory(root: &Path) -> Result<(), NativePassportRequestReplayStoreError> {
    if let Ok(metadata) = fs::symlink_metadata(root) {
        if metadata.file_type().is_symlink() {
            return Err(NativePassportRequestReplayStoreError::Corrupt(
                "Native Passport request replay root must not be a symlink",
            ));
        }
    }

    fs::create_dir_all(root).map_err(|_| {
        NativePassportRequestReplayStoreError::Unavailable(
            "create Native Passport request replay directory",
        )
    })?;

    let metadata = fs::symlink_metadata(root).map_err(|_| {
        NativePassportRequestReplayStoreError::Unavailable(
            "stat Native Passport request replay directory",
        )
    })?;

    if !metadata.file_type().is_dir() {
        return Err(NativePassportRequestReplayStoreError::Corrupt(
            "Native Passport request replay root is not a directory",
        ));
    }

    #[cfg(unix)]
    {
        fs::set_permissions(root, fs::Permissions::from_mode(0o700)).map_err(|_| {
            NativePassportRequestReplayStoreError::Unavailable(
                "secure Native Passport request replay directory permissions",
            )
        })?;
    }

    Ok(())
}

fn record_key_hex(capability_id: &CapabilityIdV1, request_nonce: &B3DigestHex) -> String {
    let mut hasher = blake3::Hasher::new();

    hash_component(&mut hasher, RECORD_KEY_DOMAIN.as_bytes());
    hash_component(&mut hasher, capability_id.as_str().as_bytes());
    hash_component(&mut hasher, request_nonce.as_str().as_bytes());

    hasher.finalize().to_hex().to_string()
}

fn hash_component(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    let length = u64::try_from(bytes.len()).expect("bounded replay-key component length");

    hasher.update(&length.to_be_bytes());
    hasher.update(bytes);
}

fn sync_directory_best_effort(root: &Path) -> Result<(), NativePassportRequestReplayStoreError> {
    #[cfg(unix)]
    {
        let directory = File::open(root).map_err(|_| {
            NativePassportRequestReplayStoreError::Unavailable(
                "open Native Passport request replay directory for sync",
            )
        })?;

        directory.sync_all().map_err(|_| {
            NativePassportRequestReplayStoreError::Unavailable(
                "sync Native Passport request replay directory",
            )
        })?;
    }

    #[cfg(not(unix))]
    {
        let _ = root;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_MS: u64 = 1_787_600_000_000;
    const RETENTION_MS: u64 = 3_630_000;

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();

            Self {
                path: std::env::temp_dir().join(format!(
                    "svc-passport-request-replay-{label}-{}-{stamp}",
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

    fn capability(hex: &str) -> CapabilityIdV1 {
        CapabilityIdV1::parse(format!("capability:v1:b3:{hex}")).expect("capability ID")
    }

    fn nonce(hex: &str) -> B3DigestHex {
        B3DigestHex::parse("request_nonce", hex).expect("request nonce")
    }

    #[test]
    fn accepted_nonce_survives_restart_and_replay_is_rejected() {
        let directory = TestDirectory::new("restart");
        let capability = capability(HEX_A);
        let nonce = nonce(HEX_B);

        let store = NativePassportRequestReplayStore::open(directory.path(), RETENTION_MS, NOW_MS)
            .expect("open");

        store
            .consume(&capability, &nonce, NOW_MS)
            .expect("first consumption");

        drop(store);

        let reopened =
            NativePassportRequestReplayStore::open(directory.path(), RETENTION_MS, NOW_MS + 1)
                .expect("reopen");

        assert_eq!(
            reopened.consume(&capability, &nonce, NOW_MS + 1,),
            Err(NativePassportRequestReplayStoreError::AlreadyConsumed),
        );
    }

    #[test]
    fn replay_identity_is_capability_and_nonce_pair() {
        let directory = TestDirectory::new("binding");

        let first_capability = capability(HEX_A);
        let second_capability = capability(HEX_C);
        let request_nonce = nonce(HEX_B);

        let store = NativePassportRequestReplayStore::open(directory.path(), RETENTION_MS, NOW_MS)
            .expect("open");

        store
            .consume(&first_capability, &request_nonce, NOW_MS)
            .expect("first capability");

        store
            .consume(&second_capability, &request_nonce, NOW_MS + 1)
            .expect("separate capability binding");

        assert_eq!(
            store.consume(&first_capability, &request_nonce, NOW_MS + 2,),
            Err(NativePassportRequestReplayStoreError::AlreadyConsumed),
        );
    }

    #[test]
    fn retention_cannot_be_shorter_than_full_two_sided_freshness_window() {
        let directory = TestDirectory::new("retention");

        assert_eq!(
            NativePassportRequestReplayStore::open(
                directory.path(),
                MIN_REQUEST_REPLAY_RETENTION_MS.saturating_sub(1),
                NOW_MS,
            )
            .unwrap_err(),
            NativePassportRequestReplayStoreError::InvalidConfig,
        );
    }

    #[test]
    fn exact_full_two_sided_freshness_window_is_accepted() {
        let directory = TestDirectory::new("exact-retention");

        NativePassportRequestReplayStore::open(
            directory.path(),
            MIN_REQUEST_REPLAY_RETENTION_MS,
            NOW_MS,
        )
        .expect("exact minimum replay retention must be accepted");
    }

    #[test]
    fn future_skewed_proof_cannot_replay_before_full_window_elapses() {
        let directory = TestDirectory::new("future-skew-window");
        let capability = capability(HEX_A);
        let request_nonce = nonce(HEX_B);

        let store = NativePassportRequestReplayStore::open(
            directory.path(),
            MIN_REQUEST_REPLAY_RETENTION_MS,
            NOW_MS,
        )
        .expect("open");

        store
            .consume(&capability, &request_nonce, NOW_MS)
            .expect("consume");

        drop(store);

        let still_fresh_boundary = NOW_MS
            .checked_add(MIN_REQUEST_REPLAY_RETENTION_MS)
            .expect("boundary time");

        let reopened = NativePassportRequestReplayStore::open(
            directory.path(),
            MIN_REQUEST_REPLAY_RETENTION_MS,
            still_fresh_boundary,
        )
        .expect("reopen at full freshness boundary");

        assert_eq!(
            reopened.consume(&capability, &request_nonce, still_fresh_boundary,),
            Err(NativePassportRequestReplayStoreError::AlreadyConsumed,),
        );
    }

    #[test]
    fn expired_records_are_pruned_after_retention() {
        let directory = TestDirectory::new("prune");
        let capability = capability(HEX_A);
        let request_nonce = nonce(HEX_B);

        let store = NativePassportRequestReplayStore::open(directory.path(), RETENTION_MS, NOW_MS)
            .expect("open");

        store
            .consume(&capability, &request_nonce, NOW_MS)
            .expect("consume");

        drop(store);

        let after_retention = NOW_MS
            .checked_add(RETENTION_MS)
            .and_then(|value| value.checked_add(1))
            .expect("time");

        let reopened =
            NativePassportRequestReplayStore::open(directory.path(), RETENTION_MS, after_retention)
                .expect("reopen");

        reopened
            .consume(&capability, &request_nonce, after_retention)
            .expect("record was safely pruned");
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let directory = TestDirectory::new("corrupt");
        let capability = capability(HEX_A);
        let request_nonce = nonce(HEX_B);

        let store = NativePassportRequestReplayStore::open(directory.path(), RETENTION_MS, NOW_MS)
            .expect("open");

        store
            .consume(&capability, &request_nonce, NOW_MS)
            .expect("consume");

        let key = record_key_hex(&capability, &request_nonce);

        let path = directory
            .path()
            .join(format!("{RECORD_PREFIX}{key}{RECORD_SUFFIX}",));

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("json");

        value["caller_authority"] = serde_json::json!(true);

        fs::write(&path, serde_json::to_vec(&value).expect("encode")).expect("rewrite");

        assert_eq!(
            NativePassportRequestReplayStore::open(directory.path(), RETENTION_MS, NOW_MS + 1,)
                .unwrap_err(),
            NativePassportRequestReplayStoreError::Corrupt(
                "decode Native Passport request replay record",
            ),
        );
    }

    #[cfg(unix)]
    #[test]
    fn directory_and_record_permissions_are_private() {
        let directory = TestDirectory::new("permissions");
        let capability = capability(HEX_A);
        let request_nonce = nonce(HEX_B);

        let store = NativePassportRequestReplayStore::open(directory.path(), RETENTION_MS, NOW_MS)
            .expect("open");

        store
            .consume(&capability, &request_nonce, NOW_MS)
            .expect("consume");

        assert_eq!(
            fs::metadata(directory.path())
                .expect("directory metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700,
        );

        let key = record_key_hex(&capability, &request_nonce);

        let record_path = directory
            .path()
            .join(format!("{RECORD_PREFIX}{key}{RECORD_SUFFIX}",));

        assert_eq!(
            fs::metadata(record_path)
                .expect("record metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600,
        );
    }

    #[test]
    fn source_has_no_route_signing_username_or_value_authority() {
        let source = include_str!("server_request_replay_store.rs");

        let implementation = source
            .split("\n#[cfg(test)]")
            .next()
            .expect("implementation");

        for forbidden in [
            "Router::",
            ".route(",
            "claim_main_username(",
            "claim_username(",
            "sign(",
            "device_private_key",
            "root_private_key",
            "recovery_phrase",
            "raw_pin",
            "issue_capability(",
            "wallet.spend(",
            "ledger.write(",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "request replay store gained forbidden authority pattern {forbidden}",
            );
        }
    }
}
