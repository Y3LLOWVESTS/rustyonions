//! RO:WHAT — Coordinate exact local object pruning across storage, DHT,
//! and the svc-index provider-response cache.
//!
//! RO:WHY — A service-node prune must remove local bytes, withdraw this
//! node's exact provider record, and invalidate stale local provider lookup
//! results without collapsing the independent outcomes into fake success.
//!
//! RO:INTERACTS — svc-storage Storage::prune, svc-dht ProviderStore,
//! svc-index IndexCache, RuntimeStatus, and the moderation admin endpoint.
//!
//! RO:INVARIANTS —
//!   - The exact registered runtime stores and cache are used.
//!   - Each local mutation outcome remains independently visible.
//!   - Repeated prune reports already_absent instead of fake mutation.
//!   - Provider withdrawal targets only the configured local node identity.
//!   - Index invalidation touches provider responses only.
//!   - Resolve entries and manifest pointers are never removed here.
//!   - No network-wide propagation, wallet, ledger, or reward authority.
//!
//! RO:TEST — services::prune::tests.

#![forbid(unsafe_code)]

use std::{
    fmt,
    sync::{Arc, Mutex},
};

use ron_policy::B3Id;
use serde::Serialize;
use svc_dht::{provider::ProviderWithdrawalOutcome, types::CrabNodeId, ProviderStore};
use svc_index::cache::{IndexCache, ProviderCacheInvalidationOutcome};
use svc_storage::storage::{DynStorage, PruneOutcome};

const LOCAL_SCOPE: &str = "local_storage_provider_and_index_cache";

#[derive(Clone)]
struct DhtPruneHandle {
    providers: Arc<ProviderStore>,
    node_id: CrabNodeId,
}

/// Shared handles for the authoritative embedded runtime stores and cache.
#[derive(Default)]
pub struct PruneCoordinator {
    storage: Mutex<Option<DynStorage>>,
    dht: Mutex<Option<DhtPruneHandle>>,
    index: Mutex<Option<IndexCache>>,
}

impl fmt::Debug for PruneCoordinator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let storage_registered = self
            .storage
            .lock()
            .expect("prune storage mutex poisoned")
            .is_some();

        let dht_registered = self.dht.lock().expect("prune DHT mutex poisoned").is_some();

        let index_registered = self
            .index
            .lock()
            .expect("prune index mutex poisoned")
            .is_some();

        f.debug_struct("PruneCoordinator")
            .field("storage_registered", &storage_registered)
            .field("dht_registered", &dht_registered)
            .field("index_registered", &index_registered)
            .finish()
    }
}

impl PruneCoordinator {
    pub fn register_storage(&self, storage: DynStorage) {
        *self.storage.lock().expect("prune storage mutex poisoned") = Some(storage);
    }

    pub fn clear_storage(&self) {
        *self.storage.lock().expect("prune storage mutex poisoned") = None;
    }

    pub fn register_provider_store(&self, providers: Arc<ProviderStore>, node_id: CrabNodeId) {
        *self.dht.lock().expect("prune DHT mutex poisoned") =
            Some(DhtPruneHandle { providers, node_id });
    }

    pub fn clear_provider_store(&self) {
        *self.dht.lock().expect("prune DHT mutex poisoned") = None;
    }

    pub fn register_index_cache(&self, cache: IndexCache) {
        *self.index.lock().expect("prune index mutex poisoned") = Some(cache);
    }

    pub fn clear_index_cache(&self) {
        *self.index.lock().expect("prune index mutex poisoned") = None;
    }

    /// Attempt all local pruning steps independently.
    ///
    /// Handles are cloned before awaiting storage so no coordinator lock is
    /// held across `.await`.
    pub async fn prune(&self, object: &B3Id) -> PruneReport {
        let storage = self
            .storage
            .lock()
            .expect("prune storage mutex poisoned")
            .clone();

        let dht = self.dht.lock().expect("prune DHT mutex poisoned").clone();

        let index = self
            .index
            .lock()
            .expect("prune index mutex poisoned")
            .clone();

        let local_bytes = match storage {
            Some(storage) => match storage.prune(object).await {
                Ok(PruneOutcome::Removed { bytes }) => LocalBytesPruneStep::Removed { bytes },
                Ok(PruneOutcome::NotFound) => LocalBytesPruneStep::NotFound,
                Err(err) => LocalBytesPruneStep::Failed {
                    error: err.to_string(),
                },
            },
            None => LocalBytesPruneStep::Unavailable,
        };

        let provider = match dht {
            Some(dht) => match dht.providers.withdraw_id(object.as_str(), dht.node_id) {
                ProviderWithdrawalOutcome::Withdrawn => ProviderWithdrawalStep::Withdrawn,
                ProviderWithdrawalOutcome::NotFound => ProviderWithdrawalStep::NotFound,
            },
            None => ProviderWithdrawalStep::Unavailable,
        };

        let index_cache_invalidation = match index {
            Some(cache) => match cache.invalidate_providers(object.as_str()) {
                ProviderCacheInvalidationOutcome::Invalidated => {
                    IndexCacheInvalidationStep::Invalidated
                }
                ProviderCacheInvalidationOutcome::NotFound => IndexCacheInvalidationStep::NotFound,
            },
            None => IndexCacheInvalidationStep::Unavailable,
        };

        let failures = usize::from(local_bytes.failed())
            + usize::from(provider.failed())
            + usize::from(index_cache_invalidation.failed());

        let changed =
            local_bytes.changed() || provider.changed() || index_cache_invalidation.changed();

        let status = match (failures, changed) {
            (0, true) => PruneStatus::Pruned,
            (0, false) => PruneStatus::AlreadyAbsent,
            (1 | 2, _) => PruneStatus::Partial,
            _ => PruneStatus::Failed,
        };

        PruneReport {
            version: 1,
            object: object.as_str().to_string(),
            scope: LOCAL_SCOPE,
            status,
            complete: failures == 0,
            changed,
            local_bytes,
            provider,
            index_cache_invalidation,
            network_propagation: false,
            resolve_cache_invalidation: false,
            manifest_pointer_removal: false,
            wallet_mutation: false,
            ledger_mutation: false,
            reward_finality: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PruneStatus {
    Pruned,
    AlreadyAbsent,
    Partial,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LocalBytesPruneStep {
    Removed { bytes: u64 },
    NotFound,
    Unavailable,
    Failed { error: String },
}

impl LocalBytesPruneStep {
    fn changed(&self) -> bool {
        matches!(self, Self::Removed { .. })
    }

    fn failed(&self) -> bool {
        matches!(self, Self::Unavailable | Self::Failed { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProviderWithdrawalStep {
    Withdrawn,
    NotFound,
    Unavailable,
}

impl ProviderWithdrawalStep {
    fn changed(self) -> bool {
        matches!(self, Self::Withdrawn)
    }

    fn failed(self) -> bool {
        matches!(self, Self::Unavailable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexCacheInvalidationStep {
    Invalidated,
    NotFound,
    Unavailable,
}

impl IndexCacheInvalidationStep {
    fn changed(self) -> bool {
        matches!(self, Self::Invalidated)
    }

    fn failed(self) -> bool {
        matches!(self, Self::Unavailable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PruneReport {
    pub version: u8,
    pub object: String,
    pub scope: &'static str,
    pub status: PruneStatus,
    pub complete: bool,
    pub changed: bool,
    pub local_bytes: LocalBytesPruneStep,
    pub provider: ProviderWithdrawalStep,
    pub index_cache_invalidation: IndexCacheInvalidationStep,

    /// Network-wide propagation is not implemented in this phase.
    pub network_propagation: bool,

    /// Resolve-cache entries are deliberately outside exact provider-cache
    /// invalidation because no safe reverse-link ownership exists.
    pub resolve_cache_invalidation: bool,

    /// Mutable manifest pointers are not deleted by content pruning.
    pub manifest_pointer_removal: bool,

    pub wallet_mutation: bool,
    pub ledger_mutation: bool,
    pub reward_finality: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Bytes;
    use svc_index::types::ProvidersResponse;
    use svc_storage::{errors::StorageError, storage::MemoryStorage};

    const CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

    const NEIGHBOR_CID: &str =
        "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    const NODE_A_URI: &str =
        "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";

    const NODE_B_URI: &str =
        "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";

    fn object() -> B3Id {
        CID.parse().expect("test object must be canonical")
    }

    fn node(uri: &str) -> CrabNodeId {
        CrabNodeId::from_uri(uri).expect("test node must be canonical")
    }

    fn providers_response(cid: &str) -> ProvidersResponse {
        ProvidersResponse {
            cid: cid.to_owned(),
            providers: Vec::new(),
            truncated: false,
            etag: None,
        }
    }

    #[tokio::test]
    async fn prune_removes_all_exact_local_surfaces() {
        let coordinator = PruneCoordinator::default();

        let storage: DynStorage = Arc::new(MemoryStorage::default());

        storage
            .put(CID, Bytes::from_static(b"abc"))
            .await
            .expect("test object put");

        let providers = Arc::new(ProviderStore::new(std::time::Duration::from_secs(60)));

        providers
            .add(
                CID.to_owned(),
                NODE_A_URI.to_owned(),
                Some(std::time::Duration::from_secs(60)),
            )
            .expect("configured provider");

        providers
            .add(
                CID.to_owned(),
                NODE_B_URI.to_owned(),
                Some(std::time::Duration::from_secs(60)),
            )
            .expect("neighbor provider");

        let index = IndexCache::new(60);
        index.put_providers(CID.to_owned(), providers_response(CID));
        index.put_providers(NEIGHBOR_CID.to_owned(), providers_response(NEIGHBOR_CID));

        coordinator.register_storage(storage.clone());
        coordinator.register_provider_store(providers.clone(), node(NODE_A_URI));

        // This clone must reference the same cache used by the caller.
        coordinator.register_index_cache(index.clone());

        let report = coordinator.prune(&object()).await;

        assert_eq!(report.status, PruneStatus::Pruned);
        assert!(report.complete);
        assert!(report.changed);
        assert_eq!(
            report.local_bytes,
            LocalBytesPruneStep::Removed { bytes: 3 }
        );
        assert_eq!(report.provider, ProviderWithdrawalStep::Withdrawn);
        assert_eq!(
            report.index_cache_invalidation,
            IndexCacheInvalidationStep::Invalidated
        );
        assert_eq!(report.scope, "local_storage_provider_and_index_cache");
        assert!(!report.network_propagation);
        assert!(!report.resolve_cache_invalidation);
        assert!(!report.manifest_pointer_removal);
        assert!(!report.wallet_mutation);
        assert!(!report.ledger_mutation);
        assert!(!report.reward_finality);

        assert!(matches!(
            storage.head(CID).await,
            Err(StorageError::NotFound)
        ));

        assert_eq!(
            providers.get_live(CID),
            vec![NODE_B_URI.to_owned()],
            "neighbor provider must remain"
        );

        assert!(
            index.get_providers(CID).is_none(),
            "registered clone must invalidate the authoritative cache"
        );

        assert_eq!(
            index
                .get_providers(NEIGHBOR_CID)
                .expect("neighbor cache entry must survive")
                .cid,
            NEIGHBOR_CID
        );
    }

    #[tokio::test]
    async fn repeated_prune_reports_already_absent() {
        let coordinator = PruneCoordinator::default();

        let storage: DynStorage = Arc::new(MemoryStorage::default());

        let providers = Arc::new(ProviderStore::new(std::time::Duration::from_secs(60)));

        coordinator.register_storage(storage);
        coordinator.register_provider_store(providers, node(NODE_A_URI));
        coordinator.register_index_cache(IndexCache::new(60));

        let report = coordinator.prune(&object()).await;

        assert_eq!(report.status, PruneStatus::AlreadyAbsent);
        assert!(report.complete);
        assert!(!report.changed);
        assert_eq!(report.local_bytes, LocalBytesPruneStep::NotFound);
        assert_eq!(report.provider, ProviderWithdrawalStep::NotFound);
        assert_eq!(
            report.index_cache_invalidation,
            IndexCacheInvalidationStep::NotFound
        );
    }

    #[tokio::test]
    async fn missing_storage_reports_partial_but_runs_other_steps() {
        let coordinator = PruneCoordinator::default();

        let providers = Arc::new(ProviderStore::new(std::time::Duration::from_secs(60)));

        providers
            .add(
                CID.to_owned(),
                NODE_A_URI.to_owned(),
                Some(std::time::Duration::from_secs(60)),
            )
            .expect("configured provider");

        let index = IndexCache::new(60);
        index.put_providers(CID.to_owned(), providers_response(CID));

        coordinator.register_provider_store(providers.clone(), node(NODE_A_URI));
        coordinator.register_index_cache(index.clone());

        let report = coordinator.prune(&object()).await;

        assert_eq!(report.status, PruneStatus::Partial);
        assert!(!report.complete);
        assert!(report.changed);
        assert_eq!(report.local_bytes, LocalBytesPruneStep::Unavailable);
        assert_eq!(report.provider, ProviderWithdrawalStep::Withdrawn);
        assert_eq!(
            report.index_cache_invalidation,
            IndexCacheInvalidationStep::Invalidated
        );
        assert!(providers.get_live(CID).is_empty());
        assert!(index.get_providers(CID).is_none());
    }

    #[tokio::test]
    async fn missing_index_cache_reports_partial_without_fake_completion() {
        let coordinator = PruneCoordinator::default();

        let storage: DynStorage = Arc::new(MemoryStorage::default());

        storage
            .put(CID, Bytes::from_static(b"abc"))
            .await
            .expect("test object put");

        let providers = Arc::new(ProviderStore::new(std::time::Duration::from_secs(60)));

        providers
            .add(
                CID.to_owned(),
                NODE_A_URI.to_owned(),
                Some(std::time::Duration::from_secs(60)),
            )
            .expect("configured provider");

        coordinator.register_storage(storage);
        coordinator.register_provider_store(providers, node(NODE_A_URI));

        let report = coordinator.prune(&object()).await;

        assert_eq!(report.status, PruneStatus::Partial);
        assert!(!report.complete);
        assert!(report.changed);
        assert_eq!(
            report.index_cache_invalidation,
            IndexCacheInvalidationStep::Unavailable
        );
    }

    #[tokio::test]
    async fn missing_runtime_handles_fail_without_fake_success() {
        let coordinator = PruneCoordinator::default();

        let report = coordinator.prune(&object()).await;

        assert_eq!(report.status, PruneStatus::Failed);
        assert!(!report.complete);
        assert!(!report.changed);
        assert_eq!(report.local_bytes, LocalBytesPruneStep::Unavailable);
        assert_eq!(report.provider, ProviderWithdrawalStep::Unavailable);
        assert_eq!(
            report.index_cache_invalidation,
            IndexCacheInvalidationStep::Unavailable
        );
    }
}
