//! Bounded runtime loading for exact-b3 moderation policy.
//!
//! The loader accepts strict `ron-policy` JSON and returns an immutable,
//! shareable policy snapshot. It does not watch files, delete objects, publish
//! provider records, or mutate wallet, ledger, or reward state.

#![forbid(unsafe_code)]

use std::{fs, path::Path, sync::Arc};

use ron_policy::ModerationPolicy;
use thiserror::Error;

/// Maximum accepted moderation policy file size: 256 KiB.
pub const MAX_MODERATION_POLICY_BYTES: usize = 256 * 1024;

/// Low-cardinality counts for operator status and tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModerationPolicyCounts {
    pub global_deny: u64,
    pub local_block: u64,
    pub local_allow: u64,
    pub owner_tombstone: u64,
    pub quarantine: u64,
}

impl ModerationPolicyCounts {
    /// Derive counts from one immutable policy snapshot.
    #[must_use]
    pub fn from_policy(policy: &ModerationPolicy) -> Self {
        Self {
            global_deny: usize_to_u64(policy.global_deny_count()),
            local_block: usize_to_u64(policy.local_block_count()),
            local_allow: usize_to_u64(policy.local_allow_count()),
            owner_tombstone: usize_to_u64(policy.owner_tombstone_count()),
            quarantine: usize_to_u64(policy.quarantine_count()),
        }
    }

    /// Total number of configured exact-b3 entries.
    #[must_use]
    pub fn total(self) -> u64 {
        self.global_deny
            .saturating_add(self.local_block)
            .saturating_add(self.local_allow)
            .saturating_add(self.owner_tombstone)
            .saturating_add(self.quarantine)
    }
}

/// Successfully loaded immutable moderation snapshot.
#[derive(Debug, Clone)]
pub struct LoadedModerationPolicy {
    pub policy: Arc<ModerationPolicy>,
    pub counts: ModerationPolicyCounts,
}

/// Moderation policy load failure.
#[derive(Debug, Error)]
pub enum ModerationPolicyLoadError {
    #[error("moderation policy file is empty")]
    Empty,

    #[error("moderation policy file exceeds limit: {len} > {max} bytes")]
    TooLarge { len: u64, max: u64 },

    #[error("failed to inspect moderation policy file: {0}")]
    Metadata(#[source] std::io::Error),

    #[error("failed to read moderation policy file: {0}")]
    Read(#[source] std::io::Error),

    #[error("invalid moderation policy JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// Load one strict, bounded moderation policy file.
///
/// The file is checked before and after reading so replacement or growth
/// between metadata inspection and read cannot bypass the size limit.
pub fn load_moderation_policy_file(
    path: &Path,
) -> Result<LoadedModerationPolicy, ModerationPolicyLoadError> {
    let max = usize_to_u64(MAX_MODERATION_POLICY_BYTES);

    let metadata = fs::metadata(path).map_err(ModerationPolicyLoadError::Metadata)?;

    if metadata.len() > max {
        return Err(ModerationPolicyLoadError::TooLarge {
            len: metadata.len(),
            max,
        });
    }

    let bytes = fs::read(path).map_err(ModerationPolicyLoadError::Read)?;

    let actual_len = usize_to_u64(bytes.len());

    if actual_len > max {
        return Err(ModerationPolicyLoadError::TooLarge {
            len: actual_len,
            max,
        });
    }

    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Err(ModerationPolicyLoadError::Empty);
    }

    let policy: ModerationPolicy = serde_json::from_slice(&bytes)?;

    let counts = ModerationPolicyCounts::from_policy(&policy);

    Ok(LoadedModerationPolicy {
        policy: Arc::new(policy),
        counts,
    })
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
