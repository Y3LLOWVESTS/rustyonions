//! RO:WHAT — Private immutable redo-intent journal for Native Passport capability issuance transactions.
//! RO:WHY — CN-4 capability issuance spans challenge replay state and capability state; a durable admitted intent is required so restart can finish the same decision without fake success or partial-commit loss.
//! RO:INTERACTS — PassportChallengeV1, exact root-signed DeviceAuthorizationV1 admission snapshot, DeviceSession proof signature evidence, NativePassportDeviceBoundCapabilityV1, future server_capability_runtime, challenge runtime, registry verification, and capability snapshot store.
//! RO:INVARIANTS — only IssueCapability intents are admitted; intent files are immutable and idempotent by challenge ID; the exact admitted DeviceAuthorizationV1 is journaled; capability/challenge/authorization/device/scope/context/root-epoch/time bindings are strict; completion only removes the journal after durable application.
//! RO:METRICS — none.
//! RO:CONFIG — caller supplies a dedicated service-owned capability-issuance redo directory.
//! RO:SECURITY — stores signed public proof/capability metadata only; no private DeviceKey/root key/PIN/recovery material, routes, username mutation, wallet, or ledger authority.
//! RO:TEST — focused unit tests cover restart, idempotent prepare, conflicting prepare, strict binding, corruption, and completion.

#![forbid(unsafe_code)]

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use ron_proto::{
    ChallengeIdV1, DeviceAuthorizationV1, Ed25519SignatureV1,
    NativePassportDeviceBoundCapabilityV1, PassportChallengePurposeV1, PassportChallengeV1,
    CHALLENGE_ID_V1_B3_PREFIX,
};
use serde::{Deserialize, Serialize};

const STORE_SCHEMA: &str = "svc-passport.native-passport-capability-issuance-redo.v1";
const STORE_VERSION: u32 = 1;

const INTENT_PREFIX: &str = "capability-issuance-v1-";
const INTENT_SUFFIX: &str = ".json";
const TEMP_PREFIX: &str = ".capability-issuance-v1-";
const TEMP_SUFFIX: &str = ".tmp";

const MAX_INTENT_BYTES: u64 = 256 * 1024;
const MAX_PENDING_INTENTS: usize = 4096;

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePassportCapabilityIssuanceTxnIntentV1 {
    pub(super) challenge: PassportChallengeV1,
    pub(super) proof_created_at_ms: u64,
    pub(super) proof_signature: Ed25519SignatureV1,

    /// Exact root-signed device authorization used when this issuance
    /// transaction crossed the durable admission boundary.
    ///
    /// Recovery must verify this snapshot against durable root/device records
    /// rather than silently substituting a later authorization.
    pub(super) device_authorization: DeviceAuthorizationV1,

    pub(super) capability: NativePassportDeviceBoundCapabilityV1,
    pub(super) accepted_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativePassportCapabilityIssuanceTxnPrepareDispositionV1 {
    Prepared,
    AlreadyPrepared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativePassportCapabilityIssuanceTxnStoreError {
    Unavailable(&'static str),
    Corrupt(&'static str),
    Conflict,
    Capacity,
}

#[derive(Debug)]
pub(super) struct NativePassportCapabilityIssuanceTxnStore {
    root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntentFileV1 {
    schema: String,
    version: u32,
    intent: NativePassportCapabilityIssuanceTxnIntentV1,
}

fn unavailable(reason: &'static str) -> NativePassportCapabilityIssuanceTxnStoreError {
    NativePassportCapabilityIssuanceTxnStoreError::Unavailable(reason)
}

fn corrupt(reason: &'static str) -> NativePassportCapabilityIssuanceTxnStoreError {
    NativePassportCapabilityIssuanceTxnStoreError::Corrupt(reason)
}

impl NativePassportCapabilityIssuanceTxnStore {
    pub(super) fn open(
        root: impl AsRef<Path>,
    ) -> Result<
        (Self, Vec<NativePassportCapabilityIssuanceTxnIntentV1>),
        NativePassportCapabilityIssuanceTxnStoreError,
    > {
        let root = root.as_ref().to_path_buf();
        ensure_store_directory(&root)?;

        let store = Self { root };
        let pending = store.load_pending()?;

        Ok((store, pending))
    }

    pub(super) fn prepare(
        &self,
        intent: &NativePassportCapabilityIssuanceTxnIntentV1,
    ) -> Result<
        NativePassportCapabilityIssuanceTxnPrepareDispositionV1,
        NativePassportCapabilityIssuanceTxnStoreError,
    > {
        validate_intent(intent)?;

        let pending = self.load_pending()?;

        if pending.len() >= MAX_PENDING_INTENTS
            && !pending
                .iter()
                .any(|existing| existing.challenge.challenge_id == intent.challenge.challenge_id)
        {
            return Err(NativePassportCapabilityIssuanceTxnStoreError::Capacity);
        }

        let final_path = self.intent_path(&intent.challenge.challenge_id)?;

        if final_path.exists() {
            let existing = read_intent(&final_path)?;

            if existing == *intent {
                return Ok(
                    NativePassportCapabilityIssuanceTxnPrepareDispositionV1::AlreadyPrepared,
                );
            }

            return Err(NativePassportCapabilityIssuanceTxnStoreError::Conflict);
        }

        let envelope = IntentFileV1 {
            schema: STORE_SCHEMA.to_owned(),
            version: STORE_VERSION,
            intent: intent.clone(),
        };

        let bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|_| unavailable("serialize capability issuance redo intent"))?;

        if bytes.len() as u64 > MAX_INTENT_BYTES {
            return Err(unavailable(
                "capability issuance redo intent exceeds byte bound",
            ));
        }

        let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);

        let temp_path = self.root.join(format!(
            "{TEMP_PREFIX}{}-{}-{nonce}{TEMP_SUFFIX}",
            challenge_digest(&intent.challenge.challenge_id)?,
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
            .map_err(|_| unavailable("create capability issuance redo temp file"))?;

        if file.write_all(&bytes).is_err() {
            let _ = fs::remove_file(&temp_path);
            return Err(unavailable("write capability issuance redo temp file"));
        }

        if file.sync_all().is_err() {
            let _ = fs::remove_file(&temp_path);
            return Err(unavailable("sync capability issuance redo temp file"));
        }

        drop(file);

        if fs::hard_link(&temp_path, &final_path).is_err() {
            let _ = fs::remove_file(&temp_path);

            if final_path.exists() {
                let existing = read_intent(&final_path)?;

                if existing == *intent {
                    return Ok(
                        NativePassportCapabilityIssuanceTxnPrepareDispositionV1::AlreadyPrepared,
                    );
                }

                return Err(NativePassportCapabilityIssuanceTxnStoreError::Conflict);
            }

            return Err(unavailable("publish capability issuance redo intent"));
        }

        let _ = fs::remove_file(&temp_path);
        sync_directory_best_effort(&self.root);

        Ok(NativePassportCapabilityIssuanceTxnPrepareDispositionV1::Prepared)
    }

    pub(super) fn complete(
        &self,
        intent: &NativePassportCapabilityIssuanceTxnIntentV1,
    ) -> Result<(), NativePassportCapabilityIssuanceTxnStoreError> {
        validate_intent(intent)?;

        let path = self.intent_path(&intent.challenge.challenge_id)?;

        if !path.exists() {
            return Ok(());
        }

        let existing = read_intent(&path)?;

        if existing != *intent {
            return Err(NativePassportCapabilityIssuanceTxnStoreError::Conflict);
        }

        fs::remove_file(&path)
            .map_err(|_| unavailable("remove completed capability issuance redo intent"))?;

        sync_directory_best_effort(&self.root);

        Ok(())
    }

    pub(super) fn load_pending(
        &self,
    ) -> Result<
        Vec<NativePassportCapabilityIssuanceTxnIntentV1>,
        NativePassportCapabilityIssuanceTxnStoreError,
    > {
        let mut pending = Vec::new();

        for entry in fs::read_dir(&self.root)
            .map_err(|_| unavailable("read capability issuance redo directory"))?
        {
            let entry = entry.map_err(|_| unavailable("read capability issuance redo entry"))?;

            let file_type = entry
                .file_type()
                .map_err(|_| unavailable("inspect capability issuance redo entry"))?;

            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| corrupt("non-Unicode capability issuance redo entry"))?;

            if name.starts_with(TEMP_PREFIX) && name.ends_with(TEMP_SUFFIX) {
                if file_type.is_symlink() || !file_type.is_file() {
                    return Err(corrupt(
                        "capability issuance redo temp entry is not a regular file",
                    ));
                }

                fs::remove_file(entry.path())
                    .map_err(|_| unavailable("remove stale capability issuance redo temp file"))?;

                continue;
            }

            if !name.starts_with(INTENT_PREFIX) || !name.ends_with(INTENT_SUFFIX) {
                return Err(corrupt(
                    "unexpected entry in capability issuance redo directory",
                ));
            }

            if file_type.is_symlink() || !file_type.is_file() {
                return Err(corrupt(
                    "capability issuance redo intent is not a regular file",
                ));
            }

            let intent = read_intent(&entry.path())?;

            if self.intent_path(&intent.challenge.challenge_id)? != entry.path() {
                return Err(corrupt(
                    "capability issuance redo filename does not match challenge ID",
                ));
            }

            pending.push(intent);
        }

        if pending.len() > MAX_PENDING_INTENTS {
            return Err(NativePassportCapabilityIssuanceTxnStoreError::Capacity);
        }

        pending.sort_by(|left, right| {
            left.challenge
                .challenge_id
                .as_str()
                .cmp(right.challenge.challenge_id.as_str())
        });

        for pair in pending.windows(2) {
            if pair[0].challenge.challenge_id == pair[1].challenge.challenge_id {
                return Err(corrupt("duplicate capability issuance redo challenge ID"));
            }
        }

        Ok(pending)
    }

    pub(super) fn intent_path(
        &self,
        challenge_id: &ChallengeIdV1,
    ) -> Result<PathBuf, NativePassportCapabilityIssuanceTxnStoreError> {
        Ok(self.root.join(format!(
            "{INTENT_PREFIX}{}{INTENT_SUFFIX}",
            challenge_digest(challenge_id)?,
        )))
    }
}

fn validate_intent(
    intent: &NativePassportCapabilityIssuanceTxnIntentV1,
) -> Result<(), NativePassportCapabilityIssuanceTxnStoreError> {
    intent
        .challenge
        .validate()
        .map_err(|_| corrupt("capability issuance redo challenge is invalid"))?;

    intent
        .capability
        .validate()
        .map_err(|_| corrupt("capability issuance redo capability is invalid"))?;

    intent
        .device_authorization
        .validate()
        .map_err(|_| corrupt("capability issuance redo DeviceAuthorization is invalid"))?;

    if intent.challenge.purpose != PassportChallengePurposeV1::IssueCapability {
        return Err(corrupt(
            "capability issuance redo challenge purpose is invalid",
        ));
    }

    let passport_id = intent
        .challenge
        .passport_id
        .as_ref()
        .ok_or_else(|| corrupt("capability issuance redo is missing Passport binding"))?;

    let device_id = intent
        .challenge
        .device_id
        .as_ref()
        .ok_or_else(|| corrupt("capability issuance redo is missing Device binding"))?;

    if intent.challenge.operation_body_hash.is_none() {
        return Err(corrupt(
            "capability issuance redo is missing operation body binding",
        ));
    }

    if &intent.device_authorization.passport_id != passport_id {
        return Err(corrupt(
            "capability issuance redo DeviceAuthorization Passport binding mismatch",
        ));
    }

    if &intent.device_authorization.device_id != device_id {
        return Err(corrupt(
            "capability issuance redo DeviceAuthorization Device binding mismatch",
        ));
    }

    if intent.device_authorization.network_id != intent.challenge.network_id
        || intent.device_authorization.environment != intent.challenge.environment
    {
        return Err(corrupt(
            "capability issuance redo DeviceAuthorization context mismatch",
        ));
    }

    if intent.capability.root_key_epoch != Some(intent.device_authorization.root_key_epoch) {
        return Err(corrupt(
            "capability issuance redo root epoch binding mismatch",
        ));
    }

    for scope in &intent.capability.scopes {
        if !intent
            .device_authorization
            .authorized_scope_ceiling
            .as_slice()
            .iter()
            .any(|allowed| allowed == scope)
        {
            return Err(corrupt(
                "capability issuance redo scope exceeds admitted DeviceAuthorization",
            ));
        }
    }

    if &intent.capability.passport_id != passport_id {
        return Err(corrupt(
            "capability issuance redo Passport binding mismatch",
        ));
    }

    if &intent.capability.device_id != device_id {
        return Err(corrupt("capability issuance redo Device binding mismatch"));
    }

    if intent.capability.audience != intent.challenge.audience
        || intent.capability.environment != intent.challenge.environment
    {
        return Err(corrupt("capability issuance redo trusted context mismatch"));
    }

    if intent.capability.scopes != intent.challenge.requested_scopes {
        return Err(corrupt("capability issuance redo scope binding mismatch"));
    }

    if intent.accepted_at_ms == 0
        || intent.accepted_at_ms < intent.challenge.issued_at_ms
        || intent.accepted_at_ms > intent.challenge.expires_at_ms
    {
        return Err(corrupt(
            "capability issuance redo acceptance time is invalid",
        ));
    }

    if intent.proof_created_at_ms < intent.challenge.issued_at_ms
        || intent.proof_created_at_ms > intent.challenge.expires_at_ms
    {
        return Err(corrupt("capability issuance redo proof time is invalid"));
    }

    if intent.capability.issued_at_ms != intent.accepted_at_ms {
        return Err(corrupt(
            "capability issuance redo capability issue time mismatch",
        ));
    }

    Ok(())
}

fn challenge_digest(
    challenge_id: &ChallengeIdV1,
) -> Result<&str, NativePassportCapabilityIssuanceTxnStoreError> {
    challenge_id
        .as_str()
        .strip_prefix(CHALLENGE_ID_V1_B3_PREFIX)
        .ok_or_else(|| corrupt("capability issuance redo challenge ID prefix is invalid"))
}

fn ensure_store_directory(
    root: &Path,
) -> Result<(), NativePassportCapabilityIssuanceTxnStoreError> {
    fs::create_dir_all(root)
        .map_err(|_| unavailable("create capability issuance redo directory"))?;

    let metadata = fs::symlink_metadata(root)
        .map_err(|_| unavailable("inspect capability issuance redo directory"))?;

    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(corrupt(
            "capability issuance redo root is not a real directory",
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| unavailable("secure capability issuance redo directory"))?;
    }

    Ok(())
}

fn read_intent(
    path: &Path,
) -> Result<
    NativePassportCapabilityIssuanceTxnIntentV1,
    NativePassportCapabilityIssuanceTxnStoreError,
> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| unavailable("inspect capability issuance redo intent"))?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(corrupt(
            "capability issuance redo intent is not a regular file",
        ));
    }

    if metadata.len() > MAX_INTENT_BYTES {
        return Err(corrupt(
            "capability issuance redo intent exceeds byte bound",
        ));
    }

    let bytes = fs::read(path).map_err(|_| unavailable("read capability issuance redo intent"))?;

    let file: IntentFileV1 = serde_json::from_slice(&bytes)
        .map_err(|_| corrupt("decode capability issuance redo intent"))?;

    if file.schema != STORE_SCHEMA || file.version != STORE_VERSION {
        return Err(corrupt(
            "capability issuance redo intent schema/version is unsupported",
        ));
    }

    validate_intent(&file.intent)?;

    Ok(file.intent)
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
        B3DigestHex, CapabilityIdV1, DeviceAuthorizationNonceV1, DeviceAuthorizationScopeCeilingV1,
        DeviceAuthorizationSigningPayloadV1, DeviceClassV1, DeviceIdV1, Ed25519PublicKeyHex,
        NativePassportContextLabelV1, NativePassportScopeV1, PassportIdV1, ServiceKeyIdV1,
        DEVICE_AUTHORIZATION_V1_VERSION, PASSPORT_CHALLENGE_V1_VERSION,
    };

    use super::*;

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    const ISSUED_AT_MS: u64 = 1_800_000_000_000;
    const ACCEPTED_AT_MS: u64 = ISSUED_AT_MS + 10_000;

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
                    "svc-passport-capability-txn-{label}-{}-{stamp}",
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
        PassportChallengeV1 {
            version: PASSPORT_CHALLENGE_V1_VERSION,
            challenge_id: ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_A}"))
                .expect("challenge ID"),
            network_id: context("rustyonions-devnet"),
            environment: context("private-beta"),
            audience: context("svc-passport"),
            issuing_service_id: context("svc-passport"),
            service_key_id: ServiceKeyIdV1::parse("ed25519/default/v1").expect("service key ID"),
            purpose: PassportChallengePurposeV1::IssueCapability,
            requested_scopes: vec![scope("identity.read"), scope("identity.username.claim")],
            passport_id: Some(
                PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}"))
                    .expect("Passport ID"),
            ),
            device_id: Some(
                DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("Device ID"),
            ),
            operation_body_hash: Some(
                B3DigestHex::parse("operation_body_hash", HEX_D).expect("body hash"),
            ),
            nonce: B3DigestHex::parse("challenge_nonce", HEX_E).expect("nonce"),
            issued_at_ms: ISSUED_AT_MS,
            expires_at_ms: ISSUED_AT_MS + 60_000,
            service_signature: Ed25519SignatureV1::from_bytes([0x55; 64]),
        }
    }

    fn device_authorization() -> DeviceAuthorizationV1 {
        let challenge = challenge();

        let authorized_scope_ceiling = DeviceAuthorizationScopeCeilingV1::new(vec![
            scope("identity.read"),
            scope("identity.username.claim"),
        ])
        .expect("authorization scope ceiling");

        let payload = DeviceAuthorizationSigningPayloadV1 {
            version: DEVICE_AUTHORIZATION_V1_VERSION,
            network_id: challenge.network_id,
            environment: challenge.environment,
            passport_id: challenge.passport_id.expect("Passport binding"),
            root_key_epoch: 0,
            device_id: challenge.device_id.expect("Device binding"),
            device_public_key: Ed25519PublicKeyHex::parse(
                "1111111111111111111111111111111111111111111111111111111111111111",
            )
            .expect("test DeviceKey public key"),
            device_class: DeviceClassV1::RootAdminDesktop,
            authorized_scope_ceiling,
            authorization_nonce: DeviceAuthorizationNonceV1::from_bytes([0x11; 16]),
            issued_at_ms: ISSUED_AT_MS - 1_000,
            expires_at_ms: Some(ACCEPTED_AT_MS + 3_600_000),
        };

        DeviceAuthorizationV1::from_signing_payload(
            payload,
            Ed25519SignatureV1::from_bytes([0x44; 64]),
        )
        .expect("structurally valid test DeviceAuthorization")
    }

    fn capability() -> NativePassportDeviceBoundCapabilityV1 {
        NativePassportDeviceBoundCapabilityV1 {
            version: 1,
            capability_id: CapabilityIdV1::parse(format!("capability:v1:b3:{HEX_D}"))
                .expect("capability ID"),
            passport_id: PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}"))
                .expect("Passport ID"),
            device_id: DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}"))
                .expect("Device ID"),
            audience: context("svc-passport"),
            environment: context("private-beta"),
            scopes: vec![scope("identity.read"), scope("identity.username.claim")],
            issued_at_ms: ACCEPTED_AT_MS,
            expires_at_ms: ACCEPTED_AT_MS + 3_600_000,
            policy_version: 1,
            root_key_epoch: Some(0),
        }
    }

    fn intent() -> NativePassportCapabilityIssuanceTxnIntentV1 {
        NativePassportCapabilityIssuanceTxnIntentV1 {
            challenge: challenge(),
            proof_created_at_ms: ISSUED_AT_MS + 5_000,
            proof_signature: Ed25519SignatureV1::from_bytes([0x66; 64]),
            device_authorization: device_authorization(),
            capability: capability(),
            accepted_at_ms: ACCEPTED_AT_MS,
        }
    }

    #[test]
    fn immutable_intent_survives_restart_and_prepare_is_idempotent() {
        let directory = TestDirectory::new("restart");

        let (store, pending) =
            NativePassportCapabilityIssuanceTxnStore::open(directory.path()).expect("open");

        assert!(pending.is_empty());

        let intent = intent();

        assert_eq!(
            store.prepare(&intent),
            Ok(NativePassportCapabilityIssuanceTxnPrepareDispositionV1::Prepared)
        );

        assert_eq!(
            store.prepare(&intent),
            Ok(NativePassportCapabilityIssuanceTxnPrepareDispositionV1::AlreadyPrepared)
        );

        drop(store);

        let (_reopened, pending) =
            NativePassportCapabilityIssuanceTxnStore::open(directory.path()).expect("reopen");

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
        }
    }

    #[test]
    fn conflicting_prepare_for_same_challenge_fails_closed() {
        let directory = TestDirectory::new("conflict");

        let (store, _) =
            NativePassportCapabilityIssuanceTxnStore::open(directory.path()).expect("open");

        let first = intent();
        store.prepare(&first).expect("prepare first");

        let mut conflicting = first;
        conflicting.proof_signature = Ed25519SignatureV1::from_bytes([0x77; 64]);

        assert_eq!(
            store.prepare(&conflicting),
            Err(NativePassportCapabilityIssuanceTxnStoreError::Conflict),
        );
    }

    #[test]
    fn challenge_capability_binding_fails_closed() {
        let directory = TestDirectory::new("binding");

        let (store, _) =
            NativePassportCapabilityIssuanceTxnStore::open(directory.path()).expect("open");

        let mut wrong_scope = intent();
        wrong_scope.capability.scopes = vec![scope("identity.read")];

        assert!(matches!(
            store.prepare(&wrong_scope),
            Err(NativePassportCapabilityIssuanceTxnStoreError::Corrupt(
                "capability issuance redo scope binding mismatch"
            ))
        ));

        let mut wrong_purpose = intent();
        wrong_purpose.challenge.purpose = PassportChallengePurposeV1::RefreshCapability;

        assert!(matches!(
            store.prepare(&wrong_purpose),
            Err(NativePassportCapabilityIssuanceTxnStoreError::Corrupt(
                "capability issuance redo challenge purpose is invalid"
            ))
        ));
    }

    #[test]
    fn exact_device_authorization_snapshot_is_bound_into_redo_intent() {
        let directory = TestDirectory::new("authority-binding");

        let (store, _) =
            NativePassportCapabilityIssuanceTxnStore::open(directory.path()).expect("open");

        let mut wrong_device = intent();

        wrong_device.device_authorization.device_id = DeviceIdV1::parse(
            "device:v1:ed25519:b3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        )
        .expect("other Device ID");

        assert!(matches!(
            store.prepare(&wrong_device),
            Err(NativePassportCapabilityIssuanceTxnStoreError::Corrupt(
                "capability issuance redo DeviceAuthorization Device binding mismatch"
            ))
        ));

        let mut wrong_epoch = intent();
        wrong_epoch.capability.root_key_epoch = Some(1);

        assert!(matches!(
            store.prepare(&wrong_epoch),
            Err(NativePassportCapabilityIssuanceTxnStoreError::Corrupt(
                "capability issuance redo root epoch binding mismatch"
            ))
        ));

        let mut insufficient_scope = intent();

        insufficient_scope
            .device_authorization
            .authorized_scope_ceiling =
            DeviceAuthorizationScopeCeilingV1::new(vec![scope("identity.read")])
                .expect("narrow scope ceiling");

        assert!(matches!(
            store.prepare(&insufficient_scope),
            Err(NativePassportCapabilityIssuanceTxnStoreError::Corrupt(
                "capability issuance redo scope exceeds admitted DeviceAuthorization"
            ))
        ));

        assert!(
            store
                .load_pending()
                .expect("pending after rejects")
                .is_empty(),
            "rejected authority drift must never create a redo intent",
        );
    }

    #[test]
    fn completion_removes_only_the_exact_intent_and_is_idempotent() {
        let directory = TestDirectory::new("complete");

        let (store, _) =
            NativePassportCapabilityIssuanceTxnStore::open(directory.path()).expect("open");

        let intent = intent();

        store.prepare(&intent).expect("prepare");

        assert_eq!(store.load_pending().expect("pending"), vec![intent.clone()],);

        store.complete(&intent).expect("complete");

        assert!(store.load_pending().expect("pending after").is_empty());

        store.complete(&intent).expect("idempotent complete");
    }

    #[test]
    fn conflicting_completion_never_deletes_durable_intent() {
        let directory = TestDirectory::new("complete-conflict");

        let (store, _) =
            NativePassportCapabilityIssuanceTxnStore::open(directory.path()).expect("open");

        let intent = intent();

        store.prepare(&intent).expect("prepare");

        let mut conflicting = intent.clone();
        conflicting.proof_signature = Ed25519SignatureV1::from_bytes([0x7a; 64]);

        assert_eq!(
            store.complete(&conflicting),
            Err(NativePassportCapabilityIssuanceTxnStoreError::Conflict),
        );

        assert_eq!(
            store.load_pending().expect("pending"),
            vec![intent],
            "conflicting completion must not delete the admitted intent",
        );
    }

    #[test]
    fn corrupted_intent_fails_closed_on_restart() {
        let directory = TestDirectory::new("corrupt");

        let (store, _) =
            NativePassportCapabilityIssuanceTxnStore::open(directory.path()).expect("open");

        let intent = intent();

        store.prepare(&intent).expect("prepare");

        let path = store
            .intent_path(&intent.challenge.challenge_id)
            .expect("intent path");

        fs::write(path, b"{").expect("truncate");

        assert!(matches!(
            NativePassportCapabilityIssuanceTxnStore::open(directory.path()),
            Err(NativePassportCapabilityIssuanceTxnStoreError::Corrupt(
                "decode capability issuance redo intent"
            ))
        ));
    }

    #[test]
    fn transaction_store_has_no_route_secret_username_or_value_authority() {
        let source = include_str!("server_capability_txn_store.rs");
        let implementation = source
            .split("\n#[cfg(test)]")
            .next()
            .expect("implementation");

        for forbidden in [
            "Router::",
            ".route(",
            "consume_durable(",
            "issue_capability(",
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
                "capability transaction store gained forbidden authority pattern {forbidden}"
            );
        }
    }
}
