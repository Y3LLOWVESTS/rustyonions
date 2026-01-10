// crates/micronode/src/state.rs
//! RO:WHAT — Shared application state for micronode.
//! RO:WHY  — Centralize config, metrics, health, readiness probes, storage, start time.

use crate::config::schema::{Config, StorageEngine};
use crate::observability::{metrics as obs_metrics, ready::ReadyProbes};
use crate::storage::{DynStorage, MemStore};
use ron_kernel::metrics::health::HealthState;
use ron_kernel::Metrics;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub metrics: Arc<Metrics>,
    pub health: Arc<HealthState>,
    pub probes: Arc<ReadyProbes>,
    pub storage: DynStorage,

    /// Process start time for truthful uptime_seconds (svc-admin displays this).
    pub started_at: Instant,
}

impl AppState {
    pub fn new(cfg: Config) -> Self {
        let started_at = Instant::now();

        // ron-kernel metrics (prometheus registry + exporter).
        // NOTE: In this repo, Metrics::new(false) already returns Arc<Metrics> (per current usage).
        let metrics: Arc<Metrics> = Metrics::new(false);

        // Register micronode-specific metrics into the same registry used by /metrics.
        obs_metrics::init(&metrics.registry);

        let health = Arc::new(HealthState::new());
        // Default: this node is "up" once constructed.
        health.set("micronode", true);

        let probes = Arc::new(ReadyProbes::new());

        // Truthful readiness gating:
        // - Config is loaded if we’re constructing AppState from a validated Config.
        // - Listeners bound is NOT true yet (set when server actually binds).
        // - Metrics bound can be flipped when the exporter is ready (optional).
        probes.set_cfg_loaded(true);

        // deps_ok defaults to true in ReadyProbes::new() (truthful for MemStore).
        // probes.set_deps_ok(true);

        let storage: DynStorage = match cfg.storage.engine {
            StorageEngine::Mem => Arc::new(MemStore::new()),
            // If/when sled-store becomes the default, wire it here. For now, stay amnesia-first.
            _ => Arc::new(MemStore::new()),
        };

        Self {
            cfg,
            metrics,
            health,
            probes,
            storage,
            started_at,
        }
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}
