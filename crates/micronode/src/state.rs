// crates/micronode/src/state.rs
//! RO:WHAT — Process state container: config, metrics, health, readiness probes, storage.
//! RO:WHY  — Keep shared handles in one place for Axum State.
//! RO:INVARIANTS — No locks across `.await`; clone-friendly handles.

use crate::config::schema::Config;
use crate::observability::ready::ReadyProbes;
use crate::storage::{DynStorage, MemStore, Storage};
use ron_kernel::metrics::health::HealthState;
use ron_kernel::Metrics;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub metrics: Arc<Metrics>,    // exported via /metrics
    pub health: Arc<HealthState>, // liveness
    pub probes: Arc<ReadyProbes>, // readiness (truthful)
    pub storage: DynStorage,      // KV storage
}

impl AppState {
    pub fn new_with_storage(cfg: Config, storage: DynStorage) -> Self {
        // false = exporter not auto-served; we expose /metrics via axum
        let metrics: Arc<Metrics> = Metrics::new(false);
        let health = Arc::new(HealthState::new());
        let probes = Arc::new(ReadyProbes::new());

        // baseline liveness true; readiness remains truthful via probes
        health.set("micronode", true);

        Self { cfg, metrics, health, probes, storage }
    }

    /// Convenience constructor using in-memory store.
    pub fn new(cfg: Config) -> Self {
        let storage = Arc::new(MemStore::new()) as DynStorage;
        Self::new_with_storage(cfg, storage)
    }

    /// Helper to swap storage (for tests / future sled).
    pub fn with_storage<S: Storage>(mut self, s: Arc<S>) -> Self {
        self.storage = s as DynStorage;
        self
    }
}
