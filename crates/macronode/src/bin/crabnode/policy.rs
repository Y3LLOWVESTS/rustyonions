//! RO:WHAT — Canonical exact-b3 policy-file controls for the crabnode operator CLI.
//! RO:WHY — BUILD_PLAN_Z Phase 10 needs CLI-first local block, allow, and quarantine control.
//! RO:INTERACTS — crabnode.rs, ron-policy, svc-storage bounded policy loader.
//! RO:INVARIANTS — strict b3 IDs; atomic JSON replacement; startup snapshot activation only.
//! RO:SECURITY — refuses symlinks; no storage delete, provider withdrawal, wallet, ledger, or reward authority.
//! RO:TEST — integration: crates/macronode/tests/crabnode_policy_cli.rs.

#![forbid(unsafe_code)]

use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use ron_policy::{B3Id, ModerationPolicy};
use svc_storage::moderation_runtime::{
    load_moderation_policy_file, ModerationPolicyCounts, MAX_MODERATION_POLICY_BYTES,
};

use super::CrabnodeError;

const CRABNODE_POLICY_FILE_ENV: &str = "CRABNODE_POLICY_FILE";
const RUNTIME_POLICY_FILE_ENV: &str = "RON_SERVICE_NODE_MODERATION_POLICY_PATH";

#[derive(Debug)]
pub(super) enum ModerationAction {
    Block(String),
    Unblock(String),
    Allow(String),
    RemoveAllow(String),
    Quarantine(String),
    ReleaseQuarantine(String),
}

impl ModerationAction {
    fn action_name(&self) -> &'static str {
        match self {
            Self::Block(_) => "block",
            Self::Unblock(_) => "unblock",
            Self::Allow(_) => "allow",
            Self::RemoveAllow(_) => "remove_allow",
            Self::Quarantine(_) => "quarantine",
            Self::ReleaseQuarantine(_) => "release_quarantine",
        }
    }

    fn object_text(&self) -> &str {
        match self {
            Self::Block(object)
            | Self::Unblock(object)
            | Self::Allow(object)
            | Self::RemoveAllow(object)
            | Self::Quarantine(object)
            | Self::ReleaseQuarantine(object) => object,
        }
    }

    fn apply(&self, policy: &mut ModerationPolicy, object: &B3Id) -> bool {
        match self {
            Self::Block(_) => policy.insert_local_block(object.clone()),
            Self::Unblock(_) => policy.remove_local_block(object),
            Self::Allow(_) => policy.insert_local_allow(object.clone()),
            Self::RemoveAllow(_) => policy.remove_local_allow(object),
            Self::Quarantine(_) => policy.insert_quarantine(object.clone()),
            Self::ReleaseQuarantine(_) => policy.remove_quarantine(object),
        }
    }
}

pub(super) fn status(explicit_path: Option<&Path>) -> Result<(), CrabnodeError> {
    let path = configured_policy_path(explicit_path)?;
    let (policy, existed) = load_policy_for_cli(&path)?;
    let counts = ModerationPolicyCounts::from_policy(&policy);

    let body = serde_json::json!({
        "state": if existed { "loaded" } else { "not_created" },
        "entries": counts_json(counts),
        "activation": "startup_snapshot",
        "hot_reload": false,
        "storage_delete": false,
        "provider_withdrawal": false,
        "wallet_mutation": false,
        "ledger_mutation": false,
        "reward_finality": false,
    });

    println!("{body}");
    Ok(())
}

pub(super) fn mutate(
    explicit_path: Option<&Path>,
    dry_run: bool,
    action: &ModerationAction,
) -> Result<(), CrabnodeError> {
    let path = configured_policy_path(explicit_path)?;
    let object = action
        .object_text()
        .parse::<B3Id>()
        .map_err(|err| CrabnodeError::usage(format!("invalid moderation object: {err}")))?;

    if dry_run {
        println!(
            "crabnode {} {}: would update the configured exact-b3 \
             policy; no storage deletion, provider withdrawal, wallet \
             mutation, ledger mutation, or reward finality; restart \
             required for runtime activation",
            action.action_name(),
            object
        );

        return Ok(());
    }

    let (mut policy, existed) = load_policy_for_cli(&path)?;
    let changed = action.apply(&mut policy, &object);

    if changed {
        write_policy_atomically(&path, &policy)?;
    }

    let decision = policy.evaluate(&object);
    let counts = ModerationPolicyCounts::from_policy(&policy);

    let body = serde_json::json!({
        "action": action.action_name(),
        "object": object.as_str(),
        "changed": changed,
        "created_policy_file": changed && !existed,
        "restart_required": changed,
        "runtime_hot_reload": false,
        "effective": {
            "permits_serve": decision.permits_serve(),
            "reason": decision.reason.as_str(),
        },
        "entries": counts_json(counts),
        "storage_delete": false,
        "provider_withdrawal": false,
        "wallet_mutation": false,
        "ledger_mutation": false,
        "reward_finality": false,
    });

    println!("{body}");
    Ok(())
}

fn configured_policy_path(explicit_path: Option<&Path>) -> Result<PathBuf, CrabnodeError> {
    if let Some(path) = explicit_path {
        return Ok(path.to_path_buf());
    }

    if let Some(path) = optional_policy_path_env(CRABNODE_POLICY_FILE_ENV)? {
        return Ok(path);
    }

    if let Some(path) = optional_policy_path_env(RUNTIME_POLICY_FILE_ENV)? {
        return Ok(path);
    }

    Err(CrabnodeError::config(format!(
        "moderation command requires --policy-file PATH, \
         {CRABNODE_POLICY_FILE_ENV}, or {RUNTIME_POLICY_FILE_ENV}"
    )))
}

fn optional_policy_path_env(key: &'static str) -> Result<Option<PathBuf>, CrabnodeError> {
    match env::var(key) {
        Ok(raw) => {
            let trimmed = raw.trim();

            if trimmed.is_empty() {
                return Err(CrabnodeError::config(format!("{key} cannot be empty")));
            }

            Ok(Some(PathBuf::from(trimmed)))
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(CrabnodeError::config(format!(
            "{key} must contain valid UTF-8"
        ))),
    }
}

fn load_policy_for_cli(path: &Path) -> Result<(ModerationPolicy, bool), CrabnodeError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(CrabnodeError::config(
                    "refusing to read or replace a moderation policy symlink",
                ));
            }

            if !metadata.is_file() {
                return Err(CrabnodeError::config(
                    "moderation policy path must be a regular file",
                ));
            }

            let loaded = load_moderation_policy_file(path).map_err(|err| {
                CrabnodeError::config(format!("configured moderation policy was rejected: {err}"))
            })?;

            Ok((loaded.policy.as_ref().clone(), true))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok((ModerationPolicy::default(), false))
        }
        Err(err) => Err(CrabnodeError::io(format!(
            "failed to inspect moderation policy file: {err}"
        ))),
    }
}

fn write_policy_atomically(path: &Path, policy: &ModerationPolicy) -> Result<(), CrabnodeError> {
    refuse_symlink(path)?;

    let mut bytes = serde_json::to_vec_pretty(policy)
        .map_err(|err| CrabnodeError::config(format!("failed to encode policy JSON: {err}")))?;

    bytes.push(b'\n');

    if bytes.len() > MAX_MODERATION_POLICY_BYTES {
        return Err(CrabnodeError::config(format!(
            "moderation policy would exceed limit: {} > {} bytes",
            bytes.len(),
            MAX_MODERATION_POLICY_BYTES
        )));
    }

    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let parent_metadata = fs::metadata(parent).map_err(|err| {
        CrabnodeError::io(format!("failed to inspect policy parent directory: {err}"))
    })?;

    if !parent_metadata.is_dir() {
        return Err(CrabnodeError::config(
            "moderation policy parent must be a directory",
        ));
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| CrabnodeError::config("moderation policy filename must be valid UTF-8"))?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| CrabnodeError::io(format!("system clock error: {err}")))?
        .as_nanos();

    let temp_path = parent.join(format!(
        ".{file_name}.crabnode-{}-{nonce}.tmp",
        std::process::id()
    ));

    let result = write_and_replace(path, &temp_path, &bytes);

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

fn write_and_replace(path: &Path, temp_path: &Path, bytes: &[u8]) -> Result<(), CrabnodeError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(temp_path).map_err(|err| {
        CrabnodeError::io(format!("failed to create temporary policy file: {err}"))
    })?;

    file.write_all(bytes).map_err(|err| {
        CrabnodeError::io(format!("failed to write temporary policy file: {err}"))
    })?;

    file.sync_all()
        .map_err(|err| CrabnodeError::io(format!("failed to sync temporary policy file: {err}")))?;

    drop(file);

    refuse_symlink(path)?;

    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path).map_err(|err| {
            CrabnodeError::io(format!("failed to replace moderation policy: {err}"))
        })?;
    }

    fs::rename(temp_path, path).map_err(|err| {
        CrabnodeError::io(format!("failed to activate moderation policy file: {err}"))
    })
}

fn refuse_symlink(path: &Path) -> Result<(), CrabnodeError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(CrabnodeError::config(
            "refusing to read or replace a moderation policy symlink",
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(CrabnodeError::io(format!(
            "failed to inspect moderation policy path: {err}"
        ))),
    }
}

fn counts_json(counts: ModerationPolicyCounts) -> serde_json::Value {
    serde_json::json!({
        "global_deny": counts.global_deny,
        "local_block": counts.local_block,
        "local_allow": counts.local_allow,
        "owner_tombstone": counts.owner_tombstone,
        "quarantine": counts.quarantine,
        "total": counts.total(),
    })
}
