//! RO:WHAT — Object-safe storage trait and bounded ephemeral memory backend.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 11 requires unknown and unreviewed object
//! bytes to remain amnesia-first, bounded, and evictable by default.
//!
//! RO:INTERACTS — HTTP object routes, OAP object serving, paid-write ingest,
//! persistence metadata, and local pruning.
//!
//! RO:INVARIANTS — content-addressed keys; finite object/byte limits; oldest
//! entries evict first; missing objects return `NotFound`; ranges never clamp.
//!
//! RO:SECURITY — the default backend is memory-only and makes no restart
//! durability, provider, moderation, reward, wallet, or ledger claim.
//!
//! RO:TEST — `tests/amnesia_storage.rs` and `tests/storage_prune.rs`.

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use axum::body::Bytes;
use parking_lot::RwLock;
use ron_policy::B3Id;

use crate::errors::StorageError;

/// Default maximum number of objects retained by the memory cache.
pub const DEFAULT_MEMORY_MAX_OBJECTS: usize = 4_096;

/// Default maximum aggregate bytes retained by the memory cache.
pub const DEFAULT_MEMORY_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// Metadata returned for one locally stored object.
#[derive(Debug, Clone)]
pub struct HeadMeta {
    /// Exact byte length.
    pub len: u64,
    /// Strong BLAKE3 entity tag.
    pub etag: String,
}

/// Storage result alias.
pub type Result<T, E = StorageError> = std::result::Result<T, E>;

/// Truthful local residency class for the current backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageResidency {
    /// Process-local memory that disappears with the backend instance.
    EphemeralMemory,
}

impl StorageResidency {
    /// Stable low-cardinality residency label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EphemeralMemory => "ephemeral_memory",
        }
    }
}

/// Finite limits for one memory-storage instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryStorageLimits {
    max_objects: usize,
    max_bytes: u64,
}

impl MemoryStorageLimits {
    /// Build finite non-zero memory limits.
    ///
    /// Returns `None` when either limit is zero.
    #[must_use]
    pub const fn try_new(max_objects: usize, max_bytes: u64) -> Option<Self> {
        if max_objects == 0 || max_bytes == 0 {
            None
        } else {
            Some(Self {
                max_objects,
                max_bytes,
            })
        }
    }

    /// Maximum number of retained objects.
    #[must_use]
    pub const fn max_objects(self) -> usize {
        self.max_objects
    }

    /// Maximum aggregate retained bytes.
    #[must_use]
    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }
}

impl Default for MemoryStorageLimits {
    fn default() -> Self {
        Self {
            max_objects: DEFAULT_MEMORY_MAX_OBJECTS,
            max_bytes: DEFAULT_MEMORY_MAX_BYTES,
        }
    }
}

/// Truthful result of removing one exact local object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PruneOutcome {
    /// The object existed and its local bytes were removed.
    Removed {
        /// Number of bytes removed from the local store.
        bytes: u64,
    },
    /// The exact object was not present locally.
    NotFound,
}

/// Object-safe local storage interface.
#[async_trait::async_trait]
pub trait Storage: Send + Sync + 'static {
    /// Store bytes under their caller-supplied content identifier.
    async fn put(&self, cid: &str, data: Bytes) -> Result<()>;

    /// Return whether the exact local key exists.
    #[allow(dead_code)]
    async fn exists(&self, cid: &str) -> Result<bool>;

    /// Return local object metadata.
    async fn head(&self, cid: &str) -> Result<HeadMeta>;

    /// Remove the bytes for one exact canonical B3 object.
    ///
    /// This operation affects local storage only. It does not withdraw
    /// provider records, mutate moderation policy, or create reward evidence.
    async fn prune(&self, object: &B3Id) -> Result<PruneOutcome>;

    /// Return the complete local object.
    #[allow(dead_code)]
    async fn get_full(&self, cid: &str) -> Result<Bytes>;

    /// Return an inclusive byte range and the complete object length.
    async fn get_range(&self, cid: &str, start: u64, end_inclusive: u64) -> Result<(Bytes, u64)>;
}

#[derive(Debug, Default)]
struct MemoryStorageState {
    objects: HashMap<String, Bytes>,
    insertion_order: VecDeque<String>,
    total_bytes: u64,
}

impl MemoryStorageState {
    fn remove(&mut self, cid: &str) -> Option<Bytes> {
        let removed = self.objects.remove(cid)?;

        self.total_bytes = self
            .total_bytes
            .saturating_sub(u64::try_from(removed.len()).unwrap_or(u64::MAX));

        self.insertion_order.retain(|queued| queued != cid);

        Some(removed)
    }

    fn evict_oldest(&mut self) -> bool {
        while let Some(oldest) = self.insertion_order.pop_front() {
            if let Some(removed) = self.objects.remove(&oldest) {
                self.total_bytes = self
                    .total_bytes
                    .saturating_sub(u64::try_from(removed.len()).unwrap_or(u64::MAX));

                return true;
            }
        }

        false
    }
}

/// Bounded, process-local, evictable storage.
///
/// This backend is intentionally amnesia-first. It performs no filesystem
/// writes and makes no claim that bytes survive a restart or new instance.
pub struct MemoryStorage {
    limits: MemoryStorageLimits,
    inner: RwLock<MemoryStorageState>,
}

impl MemoryStorage {
    /// Build the default bounded memory cache.
    #[must_use]
    pub fn new() -> Self {
        Self::with_limits(MemoryStorageLimits::default())
    }

    /// Build a bounded memory cache with explicit finite limits.
    #[must_use]
    pub fn with_limits(limits: MemoryStorageLimits) -> Self {
        Self {
            limits,
            inner: RwLock::new(MemoryStorageState::default()),
        }
    }

    /// Truthful residency class for this backend.
    #[must_use]
    pub const fn residency(&self) -> StorageResidency {
        StorageResidency::EphemeralMemory
    }

    /// Whether entries may be removed automatically to enforce limits.
    #[must_use]
    pub const fn is_evictable(&self) -> bool {
        true
    }

    /// Configured finite limits.
    #[must_use]
    pub const fn limits(&self) -> MemoryStorageLimits {
        self.limits
    }

    /// Current number of retained objects.
    #[must_use]
    pub fn object_count(&self) -> usize {
        self.inner.read().objects.len()
    }

    /// Current aggregate retained bytes.
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.inner.read().total_bytes
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Storage for MemoryStorage {
    async fn put(&self, cid: &str, data: Bytes) -> Result<()> {
        let data_len = u64::try_from(data.len()).map_err(|_| StorageError::CapacityExceeded)?;

        if data_len > self.limits.max_bytes {
            return Err(StorageError::CapacityExceeded);
        }

        let mut state = self.inner.write();

        // Replacing an existing exact key must not double-count bytes or queue
        // entries. The replacement becomes the newest cache entry.
        state.remove(cid);

        while state.objects.len() >= self.limits.max_objects
            || state.total_bytes.saturating_add(data_len) > self.limits.max_bytes
        {
            if !state.evict_oldest() {
                return Err(StorageError::CapacityExceeded);
            }
        }

        state.total_bytes = state
            .total_bytes
            .checked_add(data_len)
            .ok_or(StorageError::CapacityExceeded)?;

        state.insertion_order.push_back(cid.to_string());
        state.objects.insert(cid.to_string(), data);

        Ok(())
    }

    async fn exists(&self, cid: &str) -> Result<bool> {
        Ok(self.inner.read().objects.contains_key(cid))
    }

    async fn head(&self, cid: &str) -> Result<HeadMeta> {
        let state = self.inner.read();
        let bytes = state.objects.get(cid).ok_or(StorageError::NotFound)?;

        let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let etag = format!("\"{}\"", blake3::hash(bytes).to_hex());

        Ok(HeadMeta { len, etag })
    }

    async fn prune(&self, object: &B3Id) -> Result<PruneOutcome> {
        let mut state = self.inner.write();

        let Some(bytes) = state.remove(object.as_str()) else {
            return Ok(PruneOutcome::NotFound);
        };

        Ok(PruneOutcome::Removed {
            bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        })
    }

    async fn get_full(&self, cid: &str) -> Result<Bytes> {
        self.inner
            .read()
            .objects
            .get(cid)
            .cloned()
            .ok_or(StorageError::NotFound)
    }

    async fn get_range(&self, cid: &str, start: u64, end_inclusive: u64) -> Result<(Bytes, u64)> {
        let state = self.inner.read();
        let bytes = state.objects.get(cid).ok_or(StorageError::NotFound)?;
        let total_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);

        if start > end_inclusive || start >= total_len || end_inclusive >= total_len {
            return Err(StorageError::RangeNotSatisfiable);
        }

        let start = usize::try_from(start).map_err(|_| StorageError::RangeNotSatisfiable)?;
        let end = usize::try_from(end_inclusive).map_err(|_| StorageError::RangeNotSatisfiable)?;

        Ok((bytes.slice(start..=end), total_len))
    }
}

/// Shared object-safe storage handle.
pub type DynStorage = Arc<dyn Storage + Send + Sync + 'static>;
