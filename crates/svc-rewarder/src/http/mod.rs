//! RO:WHAT — HTTP module facade and shared state for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/PERF/ECON. The HTTP shell is the stable service contract.
//! RO:INTERACTS — config, metrics, readiness, concurrency, outputs intents, cache.
//! RO:INVARIANTS — shared state uses bounded/locked structures without holding locks across await.
//! RO:METRICS — Metrics handle shared by all handlers.
//! RO:CONFIG — RewarderState derives from validated Config.
//! RO:SECURITY — handlers enforce caps; state contains no secret values.
//! RO:TEST — integration/http_compute.rs and readiness.rs.

use std::sync::Arc;

use crate::bus::RewarderBus;
use crate::concurrency::ConcurrencyGates;
use crate::config::Config;
use crate::inputs::cache::FifoCache;
use crate::metrics::Metrics;
use crate::outputs::{IntentStore, RewardManifest};
use crate::readiness::HealthState;
use crate::Result;

pub mod dto;
pub mod error;
pub mod handlers;
pub mod routes;

/// Cloneable application state.
#[derive(Clone)]
pub struct RewarderState {
    /// Effective config.
    pub config: Arc<Config>,
    /// Metrics handle.
    pub metrics: Metrics,
    /// Readiness gates.
    pub health: Arc<HealthState>,
    /// Concurrency gates.
    pub gates: ConcurrencyGates,
    /// Event bus.
    pub bus: RewarderBus,
    /// Idempotent egress store.
    pub intents: Arc<IntentStore>,
    /// In-memory manifest cache by epoch id.
    pub manifests: Arc<FifoCache<String, RewardManifest>>,
}

impl RewarderState {
    /// Build state from validated config.
    pub fn new(config: Config) -> Result<Self> {
        let metrics = Metrics::new()?;
        let health = Arc::new(HealthState::new());
        health.set(|s| {
            s.config_loaded = true;
            s.ledger_ok = true;
            s.policy_registry_ok = true;
            s.queue_ok = true;
        });
        Ok(Self {
            gates: ConcurrencyGates::new(&config.concurrency),
            config: Arc::new(config),
            metrics,
            health,
            bus: RewarderBus::new(256),
            intents: Arc::new(IntentStore::default()),
            manifests: Arc::new(FifoCache::new(1024)),
        })
    }
}
