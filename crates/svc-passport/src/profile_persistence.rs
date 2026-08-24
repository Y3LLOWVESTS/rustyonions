//! RO:WHAT — Durable immutable-generation snapshot backing for svc-passport username/profile claims.
//! RO:WHY — CrabNode CN-4 requires restart-safe identity state without changing UsernameClaimStore authority.
//! RO:INTERACTS — profile::UsernameClaimRecord and UsernameClaimStore::open_durable.
//! RO:INVARIANTS — snapshots are immutable, generation-monotonic, strict/versioned, bounded, and never overwrite an existing generation.
//! RO:METRICS — none; persistence failures surface through profile claim errors.
//! RO:CONFIG — caller supplies one service-owned state directory.
//! RO:SECURITY — final snapshots reject symlinks/unknown files; temp files are non-authoritative; concurrent generation publication cannot clobber.
//! RO:TEST — crabnode_cn4_durable_profile_store.rs.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::profile::UsernameClaimRecord;

const STORE_SCHEMA: &str = "svc-passport.username-claim-store.v1";

const STORE_VERSION: u32 = 1;

const SNAPSHOT_PREFIX: &str = "claims-v1-";

const SNAPSHOT_SUFFIX: &str = ".json";

const TEMP_PREFIX: &str = ".claims-v1-";

const TEMP_SUFFIX: &str = ".tmp";

const GENERATION_DIGITS: usize = 20;

const MAX_SNAPSHOT_BYTES: u64 = 32 * 1024 * 1024;

const MAX_CLAIMS: usize = 50_000;

const MAX_SNAPSHOTS: usize = 32;

static TEMP_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(crate) struct ProfileSnapshotPersistence {
    root: PathBuf,
}

#[derive(Debug)]
pub(crate) struct LoadedProfileSnapshot {
    pub(crate) generation: u64,
    pub(crate) claims: Vec<UsernameClaimRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileSnapshotError {
    Unavailable(&'static str),
    Corrupt(&'static str),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotFileV1 {
    schema: String,
    version: u32,
    generation: u64,
    claims: Vec<UsernameClaimRecord>,
}

impl ProfileSnapshotPersistence {
    pub(crate) fn open(
        root: impl AsRef<Path>,
    ) -> Result<(Self, LoadedProfileSnapshot), ProfileSnapshotError> {
        let root = root.as_ref().to_path_buf();

        ensure_store_directory(&root)?;

        let persistence = Self { root };

        let snapshots = persistence.scan_snapshot_paths(true)?;

        if snapshots.len() > MAX_SNAPSHOTS {
            return Err(ProfileSnapshotError::Corrupt(
                "too many durable profile snapshots",
            ));
        }

        let mut loaded = LoadedProfileSnapshot {
            generation: 0,
            claims: Vec::new(),
        };

        for (generation, path) in snapshots {
            let snapshot = read_snapshot(&path, generation)?;

            loaded = LoadedProfileSnapshot {
                generation: snapshot.generation,
                claims: snapshot.claims,
            };
        }

        Ok((persistence, loaded))
    }

    pub(crate) fn persist(
        &self,
        expected_generation: u64,
        expected_claims: &[UsernameClaimRecord],
        next_generation: u64,
        next_claims: &[UsernameClaimRecord],
    ) -> Result<(), ProfileSnapshotError> {
        if next_generation
            != expected_generation
                .checked_add(1)
                .ok_or(ProfileSnapshotError::Unavailable(
                    "profile snapshot generation overflow",
                ))?
        {
            return Err(ProfileSnapshotError::Corrupt(
                "non-sequential profile snapshot generation",
            ));
        }

        if next_claims.len() > MAX_CLAIMS {
            return Err(ProfileSnapshotError::Unavailable(
                "profile snapshot claim bound exceeded",
            ));
        }

        let mut snapshots = self.scan_snapshot_paths(true)?;

        let disk_generation = snapshots
            .last()
            .map(|(generation, _)| *generation)
            .unwrap_or(0);

        if disk_generation != expected_generation {
            return Err(ProfileSnapshotError::Unavailable(
                "profile snapshot generation changed concurrently",
            ));
        }

        if expected_generation > 0 {
            let (_, latest_path) = snapshots.last().ok_or(ProfileSnapshotError::Corrupt(
                "profile snapshot generation missing",
            ))?;

            let latest = read_snapshot(latest_path, expected_generation)?;

            if latest.claims != expected_claims {
                return Err(ProfileSnapshotError::Corrupt(
                    "durable profile snapshot disagrees with memory",
                ));
            }
        } else if !expected_claims.is_empty() {
            return Err(ProfileSnapshotError::Corrupt(
                "memory contains claims without a durable generation",
            ));
        }

        while snapshots.len() >= MAX_SNAPSHOTS {
            let (_, oldest) = snapshots.remove(0);

            fs::remove_file(&oldest)
                .map_err(|_| ProfileSnapshotError::Unavailable("prune old profile snapshot"))?;
        }

        let envelope = SnapshotFileV1 {
            schema: STORE_SCHEMA.to_owned(),
            version: STORE_VERSION,
            generation: next_generation,
            claims: next_claims.to_vec(),
        };

        let bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|_| ProfileSnapshotError::Unavailable("serialize profile snapshot"))?;

        if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
            return Err(ProfileSnapshotError::Unavailable(
                "profile snapshot byte bound exceeded",
            ));
        }

        let nonce = TEMP_NONCE.fetch_add(1, Ordering::Relaxed);

        let temp_path = self.root.join(format!(
            "{TEMP_PREFIX}{next_generation:020}-{}-{nonce}{TEMP_SUFFIX}",
            std::process::id(),
        ));

        let final_path = self.root.join(snapshot_filename(next_generation));

        let mut options = OpenOptions::new();

        options.write(true).create_new(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;

            options.mode(0o600);
        }

        let mut file = options
            .open(&temp_path)
            .map_err(|_| ProfileSnapshotError::Unavailable("create temporary profile snapshot"))?;

        if file.write_all(&bytes).is_err() {
            let _ = fs::remove_file(&temp_path);

            return Err(ProfileSnapshotError::Unavailable(
                "write temporary profile snapshot",
            ));
        }

        if file.sync_all().is_err() {
            let _ = fs::remove_file(&temp_path);

            return Err(ProfileSnapshotError::Unavailable(
                "sync temporary profile snapshot",
            ));
        }

        drop(file);

        if final_path.exists() {
            let _ = fs::remove_file(&temp_path);

            return Err(ProfileSnapshotError::Unavailable(
                "profile snapshot generation already exists",
            ));
        }

        if fs::hard_link(&temp_path, &final_path).is_err() {
            let _ = fs::remove_file(&temp_path);

            return Err(ProfileSnapshotError::Unavailable(
                "publish immutable profile snapshot",
            ));
        }

        let _ = fs::remove_file(&temp_path);

        sync_directory_best_effort(&self.root);

        Ok(())
    }

    fn scan_snapshot_paths(
        &self,
        clean_temps: bool,
    ) -> Result<Vec<(u64, PathBuf)>, ProfileSnapshotError> {
        let metadata = fs::symlink_metadata(&self.root)
            .map_err(|_| ProfileSnapshotError::Unavailable("inspect profile store directory"))?;

        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ProfileSnapshotError::Corrupt(
                "profile store root is not a real directory",
            ));
        }

        let entries = fs::read_dir(&self.root)
            .map_err(|_| ProfileSnapshotError::Unavailable("read profile store directory"))?;

        let mut snapshots = Vec::new();

        for entry in entries {
            let entry =
                entry.map_err(|_| ProfileSnapshotError::Unavailable("read profile store entry"))?;

            let file_type = entry
                .file_type()
                .map_err(|_| ProfileSnapshotError::Unavailable("inspect profile store entry"))?;

            let name = entry.file_name();

            let name = name.to_str().ok_or(ProfileSnapshotError::Corrupt(
                "profile store contains non-UTF8 entry",
            ))?;

            if name.starts_with(TEMP_PREFIX) && name.ends_with(TEMP_SUFFIX) {
                if file_type.is_symlink() || !file_type.is_file() {
                    return Err(ProfileSnapshotError::Corrupt(
                        "profile snapshot temp entry has invalid type",
                    ));
                }

                if clean_temps {
                    fs::remove_file(entry.path()).map_err(|_| {
                        ProfileSnapshotError::Unavailable("remove stale profile snapshot temp file")
                    })?;
                }

                continue;
            }

            let generation = parse_snapshot_filename(name).ok_or(ProfileSnapshotError::Corrupt(
                "unexpected entry in profile store directory",
            ))?;

            if file_type.is_symlink() || !file_type.is_file() {
                return Err(ProfileSnapshotError::Corrupt(
                    "profile snapshot is not a regular file",
                ));
            }

            snapshots.push((generation, entry.path()));
        }

        snapshots.sort_by_key(|(generation, _)| *generation);

        for pair in snapshots.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(ProfileSnapshotError::Corrupt(
                    "duplicate profile snapshot generation",
                ));
            }
        }

        Ok(snapshots)
    }
}

fn ensure_store_directory(root: &Path) -> Result<(), ProfileSnapshotError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(ProfileSnapshotError::Corrupt(
                    "profile store root is not a real directory",
                ));
            }
        }

        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(root)
                .map_err(|_| ProfileSnapshotError::Unavailable("create profile store directory"))?;
        }

        Err(_) => {
            return Err(ProfileSnapshotError::Unavailable(
                "inspect profile store directory",
            ));
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let permissions = fs::Permissions::from_mode(0o700);

        fs::set_permissions(root, permissions)
            .map_err(|_| ProfileSnapshotError::Unavailable("secure profile store directory"))?;
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
) -> Result<SnapshotFileV1, ProfileSnapshotError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| ProfileSnapshotError::Unavailable("inspect profile snapshot"))?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ProfileSnapshotError::Corrupt(
            "profile snapshot is not a regular file",
        ));
    }

    if metadata.len() == 0 || metadata.len() > MAX_SNAPSHOT_BYTES {
        return Err(ProfileSnapshotError::Corrupt(
            "profile snapshot size is invalid",
        ));
    }

    let mut file =
        File::open(path).map_err(|_| ProfileSnapshotError::Unavailable("open profile snapshot"))?;

    let mut bytes = Vec::with_capacity(metadata.len() as usize);

    file.read_to_end(&mut bytes)
        .map_err(|_| ProfileSnapshotError::Unavailable("read profile snapshot"))?;

    if bytes.len() as u64 != metadata.len() {
        return Err(ProfileSnapshotError::Corrupt(
            "profile snapshot changed while reading",
        ));
    }

    let snapshot = serde_json::from_slice::<SnapshotFileV1>(&bytes)
        .map_err(|_| ProfileSnapshotError::Corrupt("profile snapshot JSON is invalid"))?;

    if snapshot.schema != STORE_SCHEMA {
        return Err(ProfileSnapshotError::Corrupt(
            "profile snapshot schema is unsupported",
        ));
    }

    if snapshot.version != STORE_VERSION {
        return Err(ProfileSnapshotError::Corrupt(
            "profile snapshot version is unsupported",
        ));
    }

    if snapshot.generation != expected_generation {
        return Err(ProfileSnapshotError::Corrupt(
            "profile snapshot generation does not match filename",
        ));
    }

    if snapshot.claims.len() > MAX_CLAIMS {
        return Err(ProfileSnapshotError::Corrupt(
            "profile snapshot claim bound exceeded",
        ));
    }

    Ok(snapshot)
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
