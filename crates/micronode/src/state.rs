//! RO:WHAT — Process state container: config, metrics, health, readiness probes, storage.
//! RO:WHY  — Keep shared handles in one place for Axum State.
//! RO:INTERACTS — config::schema::Config, observability::ready::ReadyProbes, ron_kernel::Metrics, storage::MemStore.
//! RO:INVARIANTS — No locks across `.await`; handles are clone-friendly; storage is behind a trait.
//! RO:METRICS — Metrics handle exported via /metrics (Prometheus).
//! RO:CONFIG — Config drives server bind/dev routes (storage engine later).
//! RO:SECURITY — No capabilities enforced here (auth/policy lives at handlers).
//! RO:TEST — Covered by HTTP integration tests that exercise AppState via routes.

use crate::config::schema::Config;
use crate::observability::ready::ReadyProbes;
use crate::storage::{DynStorage, MemStore};
use ron_kernel::metrics::health::HealthState;
use ron_kernel::Metrics;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub metrics: Arc<Metrics>,    // exported via /metrics
    pub health: Arc<HealthState>, // liveness
    pub probes: Arc<ReadyProbes>, // readiness (truthful)
    pub storage: DynStorage,      // key/value engine (mem today, pluggable later)
}

impl AppState {
    pub fn new(cfg: Config) -> Self {
        // false = we don't auto-serve exporter here (we expose /metrics via axum)
        let metrics: Arc<Metrics> = Metrics::new(false);
        let health = Arc::new(HealthState::new());
        let probes = Arc::new(ReadyProbes::new());

        // For now Micronode always boots with the in-memory store.
        // Later, Config::storage.engine will select sled vs mem.
        let storage: DynStorage = Arc::new(MemStore::new());

        // Baseline liveness true; readiness remains truthful via probes.
        health.set("micronode", true);

        Self { cfg, metrics, health, probes, storage }
    }
}
