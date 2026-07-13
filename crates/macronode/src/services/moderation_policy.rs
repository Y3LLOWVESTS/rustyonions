//! RO:WHAT — Loads and activates unsigned local and signed global moderation snapshots.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 10 requires authenticated global policy before storage readiness.
//!
//! RO:INTERACTS — `ron-policy`, `svc-storage` bounded loading, and the embedded storage worker.
//!
//! RO:INVARIANTS — Signed global state and operator-local state remain separated and compose canonically.
//!
//! RO:SECURITY — Signature, expiry, epoch, rollback state, and atomic persistence fail closed.
//!
//! RO:TEST — `tests/service_node_signed_moderation.rs`.

#![forbid(unsafe_code)]

use std::{
    env,
    error::Error as StdError,
    fmt,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use fs2::FileExt;
use ron_policy::{
    verify_signed_moderation_policy, ModerationPolicy, SignedModerationPolicyV1,
    TrustedModerationSigner,
};
use serde::{Deserialize, Serialize};
use svc_storage::moderation_runtime::{
    load_moderation_policy_file, ModerationPolicyCounts, MAX_MODERATION_POLICY_BYTES,
};

const LOCAL_POLICY_PATH_ENV: &str = "RON_SERVICE_NODE_MODERATION_POLICY_PATH";
const SIGNED_POLICY_PATH_ENV: &str = "RON_SERVICE_NODE_SIGNED_MODERATION_POLICY_PATH";
const TRUSTED_SIGNER_ID_ENV: &str = "RON_SERVICE_NODE_MODERATION_TRUSTED_SIGNER_ID";
const TRUSTED_PUBLIC_KEY_HEX_ENV: &str = "RON_SERVICE_NODE_MODERATION_TRUSTED_PUBLIC_KEY_HEX";
const ACCEPTED_STATE_PATH_ENV: &str = "RON_SERVICE_NODE_MODERATION_ACCEPTED_STATE_PATH";

const ACCEPTED_STATE_VERSION: u16 = 1;
const MAX_ACCEPTED_STATE_BYTES: usize = 4 * 1024;
const ED25519_PUBLIC_KEY_HEX_LEN: usize = 64;
const ED25519_SIGNATURE_HEX_LEN: usize = 128;

/// Runtime activation metadata for one validated moderation snapshot.
#[derive(Debug, Clone, Copy)]
pub enum ModerationActivation {
    /// Existing strict local snapshot behavior.
    UnsignedLocal,
    /// Authenticated global snapshot, optionally composed with local state.
    SignedGlobal {
        /// Verified global epoch.
        epoch: u64,
        /// Verified expiration boundary.
        expires_at_unix_s: u64,
        /// Whether an operator-local overlay was composed.
        local_overlay_active: bool,
    },
}

impl ModerationActivation {
    /// Stable source label for privacy-safe runtime status.
    #[must_use]
    pub const fn status_source(self) -> &'static str {
        match self {
            Self::UnsignedLocal => "unsigned_local",
            Self::SignedGlobal {
                local_overlay_active: false,
                ..
            } => "signed_global",
            Self::SignedGlobal {
                local_overlay_active: true,
                ..
            } => "signed_global_plus_local",
        }
    }
}

/// Fully validated moderation snapshot ready for immutable router injection.
#[derive(Debug)]
pub struct ActivatedModerationPolicy {
    /// Shared canonical policy consumed by `svc-storage`.
    pub policy: Arc<ModerationPolicy>,
    /// Aggregate privacy-safe counts.
    pub counts: ModerationPolicyCounts,
    /// Source and signed metadata.
    pub activation: ModerationActivation,
}

/// Optional startup moderation configuration.
#[derive(Debug)]
pub enum ConfiguredModerationPolicy {
    /// No local or signed source was requested.
    NotConfigured,
    /// One fully validated immutable policy is ready.
    Active(ActivatedModerationPolicy),
}

/// Fail-closed startup configuration error with a bounded status source.
#[derive(Debug)]
pub struct ModerationConfigurationError {
    status_source: &'static str,
    message: String,
}

impl ModerationConfigurationError {
    fn new(status_source: &'static str, message: impl Into<String>) -> Self {
        Self {
            status_source,
            message: message.into(),
        }
    }

    /// Return a low-cardinality source label safe for runtime status.
    #[must_use]
    pub const fn status_source(&self) -> &'static str {
        self.status_source
    }
}

impl fmt::Display for ModerationConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for ModerationConfigurationError {}

#[derive(Debug)]
struct SignedPolicyConfig {
    snapshot_path: PathBuf,
    trusted_signer_id: String,
    trusted_public_key: [u8; 32],
    accepted_state_path: PathBuf,
}

impl SignedPolicyConfig {
    fn from_env() -> Result<Option<Self>, ModerationConfigurationError> {
        let snapshot_path = optional_env_value(SIGNED_POLICY_PATH_ENV)?;
        let trusted_signer_id = optional_env_value(TRUSTED_SIGNER_ID_ENV)?;
        let trusted_public_key_hex = optional_env_value(TRUSTED_PUBLIC_KEY_HEX_ENV)?;
        let accepted_state_path = optional_env_value(ACCEPTED_STATE_PATH_ENV)?;

        let any_present = snapshot_path.is_some()
            || trusted_signer_id.is_some()
            || trusted_public_key_hex.is_some()
            || accepted_state_path.is_some();

        if !any_present {
            return Ok(None);
        }

        let snapshot_path = snapshot_path.ok_or_else(|| {
            signed_config_error(format!(
                "{SIGNED_POLICY_PATH_ENV} is required when signed moderation is configured"
            ))
        })?;

        let trusted_signer_id = trusted_signer_id.ok_or_else(|| {
            signed_config_error(format!(
                "{TRUSTED_SIGNER_ID_ENV} is required when signed moderation is configured"
            ))
        })?;

        let trusted_public_key_hex = trusted_public_key_hex.ok_or_else(|| {
            signed_config_error(format!(
                "{TRUSTED_PUBLIC_KEY_HEX_ENV} is required when signed moderation is configured"
            ))
        })?;

        let accepted_state_path = accepted_state_path.ok_or_else(|| {
            signed_config_error(format!(
                "{ACCEPTED_STATE_PATH_ENV} is required when signed moderation is configured"
            ))
        })?;

        let trusted_public_key = decode_public_key_hex(&trusted_public_key_hex)?;

        Ok(Some(Self {
            snapshot_path: PathBuf::from(snapshot_path),
            trusted_signer_id,
            trusted_public_key,
            accepted_state_path: PathBuf::from(accepted_state_path),
        }))
    }
}

#[derive(Debug)]
struct VerifiedSignedGlobal {
    policy: ModerationPolicy,
    epoch: u64,
    expires_at_unix_s: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptedSignedModerationStateV1 {
    version: u16,
    signer_id: String,
    epoch: u64,
    signature_hex: String,
}

/// Load every configured moderation source and produce one canonical snapshot.
///
/// # Errors
///
/// Returns [`ModerationConfigurationError`] when local policy loading, signed
/// trust configuration, cryptographic verification, rollback checking, source
/// composition, or accepted-state persistence fails.
pub fn load_configured_moderation_policy(
) -> Result<ConfiguredModerationPolicy, ModerationConfigurationError> {
    let signed_config = SignedPolicyConfig::from_env()?;
    let local_path = optional_env_value(LOCAL_POLICY_PATH_ENV)
        .map_err(|err| unsigned_config_error(err.to_string()))?
        .map(PathBuf::from);

    let source = match (signed_config.is_some(), local_path.is_some()) {
        (true, true) => "signed_global_plus_local",
        (true, false) => "signed_global",
        (false, true) => "unsigned_local",
        (false, false) => return Ok(ConfiguredModerationPolicy::NotConfigured),
    };

    let local = match local_path {
        Some(path) => Some(load_moderation_policy_file(&path).map_err(|err| {
            ModerationConfigurationError::new(
                source,
                format!("configured local moderation policy was rejected: {err}"),
            )
        })?),
        None => None,
    };

    let Some(signed_config) = signed_config else {
        let local = local.expect("local policy exists for unsigned activation");

        return Ok(ConfiguredModerationPolicy::Active(
            ActivatedModerationPolicy {
                policy: local.policy,
                counts: local.counts,
                activation: ModerationActivation::UnsignedLocal,
            },
        ));
    };

    let local_overlay_active = local.is_some();
    let local_policy = local
        .as_ref()
        .map_or_else(ModerationPolicy::default, |loaded| {
            loaded.policy.as_ref().clone()
        });

    ModerationPolicy::compose_signed_global_with_local(&ModerationPolicy::default(), &local_policy)
        .map_err(|err| {
            ModerationConfigurationError::new(
                source,
                format!("moderation policy source separation failed: {err}"),
            )
        })?;

    let verified = verify_signed_global_policy(&signed_config)
        .map_err(|err| ModerationConfigurationError::new(source, err.to_string()))?;

    let composed =
        ModerationPolicy::compose_signed_global_with_local(&verified.policy, &local_policy)
            .expect("validated signed-global and local source roles must compose");

    let counts = ModerationPolicyCounts::from_policy(&composed);

    Ok(ConfiguredModerationPolicy::Active(
        ActivatedModerationPolicy {
            policy: Arc::new(composed),
            counts,
            activation: ModerationActivation::SignedGlobal {
                epoch: verified.epoch,
                expires_at_unix_s: verified.expires_at_unix_s,
                local_overlay_active,
            },
        },
    ))
}

fn verify_signed_global_policy(
    config: &SignedPolicyConfig,
) -> Result<VerifiedSignedGlobal, ModerationConfigurationError> {
    let bytes = read_bounded_regular_file(
        &config.snapshot_path,
        MAX_MODERATION_POLICY_BYTES,
        "signed moderation policy",
    )
    .map_err(signed_config_error)?;

    let snapshot = serde_json::from_slice::<SignedModerationPolicyV1>(&bytes).map_err(|err| {
        signed_config_error(format!("invalid signed moderation policy JSON: {err}"))
    })?;

    let trusted_signer = TrustedModerationSigner::new(
        config.trusted_signer_id.clone(),
        config.trusted_public_key,
    )
    .map_err(|err| signed_config_error(format!("invalid trusted moderation signer: {err}")))?;

    let lock_file = open_epoch_lock(&config.accepted_state_path)?;
    lock_file.lock_exclusive().map_err(|err| {
        signed_config_error(format!("failed to lock accepted moderation state: {err}"))
    })?;

    let verification = verify_signed_global_policy_under_lock(
        &snapshot,
        &trusted_signer,
        &config.accepted_state_path,
    );

    let unlock = FileExt::unlock(&lock_file).map_err(|err| {
        signed_config_error(format!("failed to unlock accepted moderation state: {err}"))
    });

    match (verification, unlock) {
        (Ok(verified), Ok(())) => Ok(verified),
        (Err(err), _) => Err(err),
        (Ok(_), Err(err)) => Err(err),
    }
}

fn verify_signed_global_policy_under_lock(
    snapshot: &SignedModerationPolicyV1,
    trusted_signer: &TrustedModerationSigner,
    accepted_state_path: &Path,
) -> Result<VerifiedSignedGlobal, ModerationConfigurationError> {
    let accepted = load_accepted_state(accepted_state_path)?;

    if let Some(state) = accepted.as_ref() {
        if state.signer_id != snapshot.signer_id {
            return Err(signed_config_error(
                "accepted moderation state signer does not match the configured signed snapshot",
            ));
        }
    }

    let last_accepted_epoch = match accepted.as_ref() {
        None => None,
        Some(state) if snapshot.epoch > state.epoch => Some(state.epoch),
        Some(state)
            if snapshot.epoch == state.epoch && snapshot.signature_hex == state.signature_hex =>
        {
            None
        }
        Some(state) if snapshot.epoch == state.epoch => {
            return Err(signed_config_error(
                "signed moderation policy reused an accepted epoch with different signed bytes",
            ));
        }
        Some(state) => Some(state.epoch),
    };

    let verified = verify_signed_moderation_policy(
        snapshot,
        trusted_signer,
        now_unix_s(),
        last_accepted_epoch,
    )
    .map_err(|err| {
        signed_config_error(format!(
            "signed moderation policy verification failed: {err}"
        ))
    })?;

    let global_counts = ModerationPolicyCounts::from_policy(&verified.policy);

    if global_counts.local_block != 0
        || global_counts.local_allow != 0
        || global_counts.quarantine != 0
    {
        return Err(signed_config_error(
            "signed global moderation policy contains operator-local state",
        ));
    }

    let requires_state_update = match accepted.as_ref() {
        None => true,
        Some(state) => verified.epoch > state.epoch,
    };

    if requires_state_update {
        let state = AcceptedSignedModerationStateV1 {
            version: ACCEPTED_STATE_VERSION,
            signer_id: verified.signer_id.clone(),
            epoch: verified.epoch,
            signature_hex: snapshot.signature_hex.clone(),
        };

        write_accepted_state_atomically(accepted_state_path, &state)?;
    }

    Ok(VerifiedSignedGlobal {
        policy: verified.policy,
        epoch: verified.epoch,
        expires_at_unix_s: verified.expires_at_unix_s,
    })
}

fn load_accepted_state(
    path: &Path,
) -> Result<Option<AcceptedSignedModerationStateV1>, ModerationConfigurationError> {
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(signed_config_error(format!(
                "failed to inspect accepted moderation state: {err}"
            )));
        }
    }

    let bytes =
        read_bounded_regular_file(path, MAX_ACCEPTED_STATE_BYTES, "accepted moderation state")
            .map_err(signed_config_error)?;

    let state =
        serde_json::from_slice::<AcceptedSignedModerationStateV1>(&bytes).map_err(|err| {
            signed_config_error(format!("invalid accepted moderation state JSON: {err}"))
        })?;

    if state.version != ACCEPTED_STATE_VERSION
        || state.epoch == 0
        || state.signer_id.is_empty()
        || !is_canonical_lower_hex(&state.signature_hex, ED25519_SIGNATURE_HEX_LEN)
    {
        return Err(signed_config_error(
            "accepted moderation state failed canonical validation",
        ));
    }

    Ok(Some(state))
}

fn open_epoch_lock(path: &Path) -> Result<File, ModerationConfigurationError> {
    let parent = policy_parent(path)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            signed_config_error("accepted moderation state filename must be valid UTF-8")
        })?;

    let lock_path = parent.join(format!(".{file_name}.lock"));
    refuse_symlink(&lock_path, "accepted moderation state lock")?;

    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    options.open(lock_path).map_err(|err| {
        signed_config_error(format!(
            "failed to open accepted moderation state lock: {err}"
        ))
    })
}

fn write_accepted_state_atomically(
    path: &Path,
    state: &AcceptedSignedModerationStateV1,
) -> Result<(), ModerationConfigurationError> {
    refuse_symlink(path, "accepted moderation state")?;

    let mut bytes = serde_json::to_vec_pretty(state).map_err(|err| {
        signed_config_error(format!("failed to encode accepted moderation state: {err}"))
    })?;
    bytes.push(b'\n');

    if bytes.len() > MAX_ACCEPTED_STATE_BYTES {
        return Err(signed_config_error(
            "accepted moderation state exceeded its size limit",
        ));
    }

    let parent = policy_parent(path)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            signed_config_error("accepted moderation state filename must be valid UTF-8")
        })?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos();

    let temp_path = parent.join(format!(
        ".{file_name}.macronode-{}-{nonce}.tmp",
        std::process::id()
    ));

    let result = write_and_replace(path, &temp_path, &bytes, parent);

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

fn write_and_replace(
    path: &Path,
    temp_path: &Path,
    bytes: &[u8],
    parent: &Path,
) -> Result<(), ModerationConfigurationError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(temp_path).map_err(|err| {
        signed_config_error(format!("failed to create temporary accepted state: {err}"))
    })?;

    file.write_all(bytes).map_err(|err| {
        signed_config_error(format!("failed to write temporary accepted state: {err}"))
    })?;
    file.sync_all().map_err(|err| {
        signed_config_error(format!("failed to sync temporary accepted state: {err}"))
    })?;

    drop(file);
    refuse_symlink(path, "accepted moderation state")?;

    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path).map_err(|err| {
            signed_config_error(format!(
                "failed to replace accepted moderation state: {err}"
            ))
        })?;
    }

    fs::rename(temp_path, path).map_err(|err| {
        signed_config_error(format!(
            "failed to activate accepted moderation state: {err}"
        ))
    })?;

    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|err| {
            signed_config_error(format!(
                "failed to sync accepted moderation state directory: {err}"
            ))
        })?;

    Ok(())
}

fn read_bounded_regular_file(
    path: &Path,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|err| format!("failed to inspect {label}: {err}"))?;

    if metadata.file_type().is_symlink() {
        return Err(format!("refusing {label} symlink"));
    }

    if !metadata.is_file() {
        return Err(format!("{label} must be a regular file"));
    }

    let max_u64 = u64::try_from(max_bytes).unwrap_or(u64::MAX);

    if metadata.len() > max_u64 {
        return Err(format!("{label} exceeds its size limit"));
    }

    let bytes = fs::read(path).map_err(|err| format!("failed to read {label}: {err}"))?;

    if bytes.is_empty() {
        return Err(format!("{label} is empty"));
    }

    if bytes.len() > max_bytes {
        return Err(format!("{label} exceeds its size limit"));
    }

    Ok(bytes)
}

fn policy_parent(path: &Path) -> Result<&Path, ModerationConfigurationError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let metadata = fs::metadata(parent).map_err(|err| {
        signed_config_error(format!("failed to inspect moderation state parent: {err}"))
    })?;

    if !metadata.is_dir() {
        return Err(signed_config_error(
            "moderation state parent must be a directory",
        ));
    }

    Ok(parent)
}

fn refuse_symlink(path: &Path, label: &str) -> Result<(), ModerationConfigurationError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(signed_config_error(format!("refusing {label} symlink")))
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(signed_config_error(format!(
            "failed to inspect {label}: {err}"
        ))),
    }
}

fn optional_env_value(key: &'static str) -> Result<Option<String>, ModerationConfigurationError> {
    match env::var(key) {
        Ok(raw) => {
            let trimmed = raw.trim();

            if trimmed.is_empty() {
                return Err(signed_config_error(format!("{key} cannot be empty")));
            }

            Ok(Some(trimmed.to_owned()))
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(signed_config_error(format!(
            "{key} must contain valid UTF-8"
        ))),
    }
}

fn decode_public_key_hex(raw: &str) -> Result<[u8; 32], ModerationConfigurationError> {
    if !is_canonical_lower_hex(raw, ED25519_PUBLIC_KEY_HEX_LEN) {
        return Err(signed_config_error(format!(
            "{TRUSTED_PUBLIC_KEY_HEX_ENV} must be 64 lowercase hexadecimal characters"
        )));
    }

    let mut decoded = [0_u8; 32];

    for (index, pair) in raw.as_bytes().chunks_exact(2).enumerate() {
        let high = decode_lower_hex(pair[0]).expect("validated public key hex");
        let low = decode_lower_hex(pair[1]).expect("validated public key hex");
        decoded[index] = (high << 4) | low;
    }

    Ok(decoded)
}

fn is_canonical_lower_hex(raw: &str, expected_len: usize) -> bool {
    raw.len() == expected_len
        && raw
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

const fn decode_lower_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn signed_config_error(message: impl Into<String>) -> ModerationConfigurationError {
    ModerationConfigurationError::new("signed_global", message)
}

fn unsigned_config_error(message: impl Into<String>) -> ModerationConfigurationError {
    ModerationConfigurationError::new("unsigned_local", message)
}
