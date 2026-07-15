// crates/micronode/src/state.rs
//! RO:WHAT — Shared application state for micronode.
//! RO:WHY  — Centralize config, metrics, health, readiness probes, storage, start time.

use crate::config::schema::{Config, StorageEngine};
use crate::observability::{metrics as obs_metrics, ready::ReadyProbes};
use crate::storage::{DynStorage, MemStore};
use crate::verification::ObjectVerificationQueue;
use ron_kernel::metrics::health::HealthState;
use ron_kernel::Metrics;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub metrics: Arc<Metrics>,
    pub health: Arc<HealthState>,
    pub probes: Arc<ReadyProbes>,
    pub storage: DynStorage,
    pub verification: Arc<ObjectVerificationQueue>,
    verification_paused: Arc<AtomicBool>,

    /// Process start time for truthful uptime_seconds (svc-admin displays this).
    pub started_at: Instant,
}

impl AppState {
    pub fn new(cfg: Config) -> Self {
        let started_at = Instant::now();

        let amnesia_mode = matches!(cfg.storage.engine, StorageEngine::Mem);

        // ron-kernel metrics (prometheus registry + exporter).
        // Seed the shared amnesia_mode gauge from the actual micronode storage posture.
        let metrics: Arc<Metrics> = Metrics::new(amnesia_mode);

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

        let verification =
            Arc::new(ObjectVerificationQueue::new(cfg.user_node.pending_evidence_limit as usize));

        let verification_paused = Arc::new(AtomicBool::new(false));

        Self {
            cfg,
            metrics,
            health,
            probes,
            storage,
            verification,
            verification_paused,
            started_at,
        }
    }

    #[must_use]
    pub fn verification_paused(&self) -> bool {
        self.verification_paused.load(Ordering::Acquire)
    }

    pub fn set_verification_paused(&self, paused: bool) -> bool {
        let previous = self.verification_paused.swap(paused, Ordering::AcqRel);

        previous != paused
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}
