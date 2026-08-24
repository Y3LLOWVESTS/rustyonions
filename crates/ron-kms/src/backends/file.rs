//! RO:WHAT — Minimal durable single-key Ed25519 backend for service-owned signing identity.
//! RO:WHY — Durable protocol artifacts must remain verifiable after process restart; process-local KMS keys cannot satisfy that invariant.
//! RO:INTERACTS — ron-kms Ed25519 primitives, `KeyId`, `Signer`/`Verifier`, and later svc-passport `KmsClient` adaptation.
//! RO:INVARIANTS — one persisted version-1 key; exact `KeyId` required; successful creation is fsynced before return; corrupt state fails closed and is never silently regenerated.
//! RO:METRICS — none.
//! RO:CONFIG — caller-owned private state directory plus fixed tenant/purpose identity.
//! RO:SECURITY — secret seed never has a public getter or Debug/Clone surface; Unix directory/file permissions are forced to 0700/0600; serialized secret buffers and in-memory seed are zeroized.
//! RO:TEST — reopen preserves identity, pre-restart signatures verify after reopen, corrupt state fails closed, and Unix permissions remain private.

#![forbid(unsafe_code)]

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroize;

use crate::{
    backends::ed25519,
    error::KmsError,
    traits::{Signer, Verifier},
    types::{Alg, KeyId},
};

const SERVICE_KEY_SCHEMA: &str = "ron-kms.durable-ed25519-service-key.v1";
const SERVICE_KEY_VERSION: u32 = 1;
const SERVICE_KEY_FILE_NAME: &str = "service-ed25519-v1.json";
const TEMP_FILE_PREFIX: &str = ".service-ed25519-v1-";
const TEMP_FILE_SUFFIX: &str = ".tmp";
const MAX_SERVICE_KEY_FILE_BYTES: u64 = 16 * 1024;
const MAX_STALE_TEMP_FILES: usize = 64;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedServiceKeyV1 {
    schema: String,
    version: u32,
    key_id: KeyId,
    public_key: [u8; 32],
    secret_seed: [u8; 32],
}

/// Durable single-version Ed25519 service key.
///
/// This deliberately does not implement general key creation or rotation.
/// CN-4 needs one stable service signing identity across restart; later KMS
/// lifecycle work can add versioned durable rotation without changing this
/// restart-safety invariant.
pub struct DurableEd25519ServiceKey {
    key_id: KeyId,
    public_key: [u8; 32],
    secret_seed: [u8; 32],
}

impl DurableEd25519ServiceKey {
    /// Open the existing service key or create it exactly once.
    ///
    /// A corrupt existing final record is never replaced. Temporary files from
    /// an interrupted pre-publication creation may be removed because the key
    /// was never published to a successful caller.
    pub fn open_or_create(
        root: impl AsRef<Path>,
        tenant: &str,
        purpose: &str,
    ) -> Result<Self, DurableServiceKeyError> {
        validate_identity_label("tenant", tenant)?;
        validate_identity_label("purpose", purpose)?;

        let root = root.as_ref();

        prepare_private_root(root)?;

        let final_path = root.join(SERVICE_KEY_FILE_NAME);

        match fs::symlink_metadata(&final_path) {
            Ok(_) => return Self::load_existing(&final_path, tenant, purpose),

            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}

            Err(error) => {
                return Err(DurableServiceKeyError::Io(error));
            }
        }

        cleanup_unpublished_temp_files(root)?;

        let (public_key, secret_seed) = ed25519::generate();

        let mut record = PersistedServiceKeyV1 {
            schema: SERVICE_KEY_SCHEMA.to_owned(),
            version: SERVICE_KEY_VERSION,
            key_id: KeyId::new(tenant, purpose, Alg::Ed25519),
            public_key,
            secret_seed,
        };

        let mut encoded = serde_json::to_vec(&record)?;

        if u64::try_from(encoded.len()).unwrap_or(u64::MAX) > MAX_SERVICE_KEY_FILE_BYTES {
            encoded.zeroize();
            record.secret_seed.zeroize();

            return Err(DurableServiceKeyError::InvalidState(
                "encoded service key exceeds bounded file size",
            ));
        }

        let temp_path = root.join(format!(
            "{TEMP_FILE_PREFIX}{}{TEMP_FILE_SUFFIX}",
            uuid::Uuid::new_v4(),
        ));

        let publish_result = publish_new_record(root, &temp_path, &final_path, &encoded);

        encoded.zeroize();
        record.secret_seed.zeroize();

        publish_result?;

        Self::load_existing(&final_path, tenant, purpose)
    }

    /// Stable internal ron-kms identifier for this persisted key.
    #[must_use]
    pub fn key_id(&self) -> &KeyId {
        &self.key_id
    }

    /// Public Ed25519 verification key.
    #[must_use]
    pub const fn public_key(&self) -> [u8; 32] {
        self.public_key
    }

    fn load_existing(
        final_path: &Path,
        expected_tenant: &str,
        expected_purpose: &str,
    ) -> Result<Self, DurableServiceKeyError> {
        let metadata = fs::symlink_metadata(final_path)?;

        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(DurableServiceKeyError::InvalidState(
                "service key path must be a regular non-symlink file",
            ));
        }

        #[cfg(unix)]
        enforce_private_file_permissions(final_path)?;

        if metadata.len() > MAX_SERVICE_KEY_FILE_BYTES {
            return Err(DurableServiceKeyError::InvalidState(
                "service key file exceeds bounded size",
            ));
        }

        let file = File::open(final_path)?;

        let mut encoded = Vec::new();

        file.take(MAX_SERVICE_KEY_FILE_BYTES + 1)
            .read_to_end(&mut encoded)?;

        if u64::try_from(encoded.len()).unwrap_or(u64::MAX) > MAX_SERVICE_KEY_FILE_BYTES {
            encoded.zeroize();

            return Err(DurableServiceKeyError::InvalidState(
                "service key file exceeded bounded read size",
            ));
        }

        let decoded = serde_json::from_slice::<PersistedServiceKeyV1>(&encoded);

        encoded.zeroize();

        let mut record = decoded?;

        if let Err(error) = validate_record(&record, expected_tenant, expected_purpose) {
            record.secret_seed.zeroize();
            return Err(error);
        }

        let secret_seed = record.secret_seed;

        record.secret_seed.zeroize();

        Ok(Self {
            key_id: record.key_id,
            public_key: record.public_key,
            secret_seed,
        })
    }
}

impl Drop for DurableEd25519ServiceKey {
    fn drop(&mut self) {
        self.secret_seed.zeroize();
    }
}

impl Signer for DurableEd25519ServiceKey {
    fn sign(&self, kid: &KeyId, msg: &[u8]) -> Result<Vec<u8>, KmsError> {
        if kid.alg != Alg::Ed25519 {
            return Err(KmsError::AlgUnavailable);
        }

        if kid != &self.key_id {
            return Err(KmsError::NoSuchKey);
        }

        Ok(ed25519::sign(&self.secret_seed, msg).to_vec())
    }
}

impl Verifier for DurableEd25519ServiceKey {
    fn verify(&self, kid: &KeyId, msg: &[u8], sig: &[u8]) -> Result<bool, KmsError> {
        if kid.alg != Alg::Ed25519 {
            return Err(KmsError::AlgUnavailable);
        }

        if kid != &self.key_id {
            return Err(KmsError::NoSuchKey);
        }

        let signature: [u8; 64] = sig.try_into().map_err(|_| KmsError::VerifyFailed)?;

        Ok(ed25519::verify(&self.public_key, msg, &signature))
    }
}

/// Durable service-key open/create failure.
#[derive(Debug, Error)]
pub enum DurableServiceKeyError {
    #[error("durable service-key I/O failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("durable service-key encoding failed: {0}")]
    Codec(#[from] serde_json::Error),

    #[error("durable service-key state invalid: {0}")]
    InvalidState(&'static str),

    #[error("durable service-key identity label {field} is invalid")]
    InvalidIdentityLabel { field: &'static str },
}

fn validate_identity_label(field: &'static str, value: &str) -> Result<(), DurableServiceKeyError> {
    let trimmed = value.trim();

    if trimmed.is_empty()
        || trimmed != value
        || value.len() > 128
        || value.chars().any(char::is_control)
    {
        return Err(DurableServiceKeyError::InvalidIdentityLabel { field });
    }

    Ok(())
}

fn validate_record(
    record: &PersistedServiceKeyV1,
    expected_tenant: &str,
    expected_purpose: &str,
) -> Result<(), DurableServiceKeyError> {
    if record.schema != SERVICE_KEY_SCHEMA {
        return Err(DurableServiceKeyError::InvalidState(
            "unexpected service key schema",
        ));
    }

    if record.version != SERVICE_KEY_VERSION {
        return Err(DurableServiceKeyError::InvalidState(
            "unexpected service key version",
        ));
    }

    if record.key_id.tenant != expected_tenant || record.key_id.purpose != expected_purpose {
        return Err(DurableServiceKeyError::InvalidState(
            "persisted service key identity does not match requested owner",
        ));
    }

    if record.key_id.alg != Alg::Ed25519 {
        return Err(DurableServiceKeyError::InvalidState(
            "persisted service key algorithm is not Ed25519",
        ));
    }

    if record.key_id.version != 1 {
        return Err(DurableServiceKeyError::InvalidState(
            "single-key backend accepts only version 1",
        ));
    }

    if ed25519::public_key(&record.secret_seed) != record.public_key {
        return Err(DurableServiceKeyError::InvalidState(
            "persisted public key does not match secret seed",
        ));
    }

    Ok(())
}

fn prepare_private_root(root: &Path) -> Result<(), DurableServiceKeyError> {
    fs::create_dir_all(root)?;

    let metadata = fs::symlink_metadata(root)?;

    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DurableServiceKeyError::InvalidState(
            "service key root must be a regular directory",
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
    }

    Ok(())
}

fn cleanup_unpublished_temp_files(root: &Path) -> Result<(), DurableServiceKeyError> {
    let mut stale = Vec::<PathBuf>::new();

    for entry in fs::read_dir(root)? {
        let entry = entry?;

        let name = entry.file_name();

        let Some(name) = name.to_str() else {
            continue;
        };

        if name.starts_with(TEMP_FILE_PREFIX) && name.ends_with(TEMP_FILE_SUFFIX) {
            stale.push(entry.path());

            if stale.len() > MAX_STALE_TEMP_FILES {
                return Err(DurableServiceKeyError::InvalidState(
                    "too many stale service-key temporary files",
                ));
            }
        }
    }

    for path in stale {
        let metadata = fs::symlink_metadata(&path)?;

        if metadata.file_type().is_symlink() || metadata.is_file() {
            fs::remove_file(path)?;
        } else {
            return Err(DurableServiceKeyError::InvalidState(
                "service-key temporary path is not a regular file",
            ));
        }
    }

    Ok(())
}

fn publish_new_record(
    root: &Path,
    temp_path: &Path,
    final_path: &Path,
    encoded: &[u8],
) -> Result<(), DurableServiceKeyError> {
    let mut options = OpenOptions::new();

    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.mode(0o600);
    }

    let mut file = options.open(temp_path)?;

    let write_result = (|| -> std::io::Result<()> {
        file.write_all(encoded)?;
        file.sync_all()?;
        Ok(())
    })();

    drop(file);

    if let Err(error) = write_result {
        let _ = fs::remove_file(temp_path);
        return Err(DurableServiceKeyError::Io(error));
    }

    match fs::hard_link(temp_path, final_path) {
        Ok(()) => {}

        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = fs::remove_file(temp_path);

            /*
             * Another writer published first. Loading the final record below
             * will accept it only if its exact owner and cryptographic binding
             * are valid.
             */
            return Ok(());
        }

        Err(error) => {
            let _ = fs::remove_file(temp_path);
            return Err(DurableServiceKeyError::Io(error));
        }
    }

    fs::remove_file(temp_path)?;

    #[cfg(unix)]
    {
        let directory = File::open(root)?;
        directory.sync_all()?;
    }

    Ok(())
}

#[cfg(unix)]
fn enforce_private_file_permissions(path: &Path) -> Result<(), DurableServiceKeyError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

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
                    "ron-kms-durable-service-key-{label}-{}-{stamp}",
                    std::process::id(),
                )),
            }
        }

        fn path(&self) -> &Path {
            &self.root
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn reopen_preserves_identity_and_verifies_pre_restart_signature() {
        let directory = TestDirectory::new("restart");

        let message = b"cn4-durable-service-key-restart-vector";

        let first =
            DurableEd25519ServiceKey::open_or_create(directory.path(), "crabnode", "svc-passport")
                .expect("create durable key");

        let first_kid = first.key_id().clone();

        let first_public = first.public_key();

        let signature = first
            .sign(&first_kid, message)
            .expect("pre-restart signature");

        assert!(first
            .verify(&first_kid, message, &signature,)
            .expect("pre-restart verify"),);

        drop(first);

        let reopened =
            DurableEd25519ServiceKey::open_or_create(directory.path(), "crabnode", "svc-passport")
                .expect("reopen durable key");

        assert_eq!(reopened.key_id(), &first_kid,);

        assert_eq!(reopened.public_key(), first_public,);

        assert!(reopened
            .verify(&first_kid, message, &signature,)
            .expect("pre-restart signature must verify after reopen",),);

        let second_signature = reopened
            .sign(&first_kid, message)
            .expect("post-restart signature");

        assert_eq!(
            signature, second_signature,
            "Ed25519 signature for identical key/message is deterministic",
        );
    }

    #[test]
    fn corrupt_final_record_fails_closed_without_silent_rekey() {
        let directory = TestDirectory::new("corrupt");

        let first =
            DurableEd25519ServiceKey::open_or_create(directory.path(), "crabnode", "svc-passport")
                .expect("create durable key");

        let original_kid = first.key_id().clone();

        drop(first);

        let final_path = directory.path().join(SERVICE_KEY_FILE_NAME);

        fs::write(&final_path, br#"{"schema":"corrupt"}"#).expect("corrupt fixture");

        let reopened =
            DurableEd25519ServiceKey::open_or_create(directory.path(), "crabnode", "svc-passport");

        assert!(reopened.is_err(), "corrupt durable key must fail closed",);

        let bytes = fs::read(&final_path).expect("corrupt final remains");

        assert_eq!(
            bytes, br#"{"schema":"corrupt"}"#,
            "failure must not silently overwrite durable key state",
        );

        assert!(
            original_kid.version == 1,
            "original fixture remains a version-1 key",
        );
    }

    #[cfg(unix)]
    #[test]
    fn durable_key_directory_and_file_are_private() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new("permissions");

        let key =
            DurableEd25519ServiceKey::open_or_create(directory.path(), "crabnode", "svc-passport")
                .expect("create durable key");

        drop(key);

        let root_mode = fs::metadata(directory.path())
            .expect("root metadata")
            .permissions()
            .mode()
            & 0o777;

        let file_mode = fs::metadata(directory.path().join(SERVICE_KEY_FILE_NAME))
            .expect("key metadata")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(root_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }
}
