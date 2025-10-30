//! RO:WHAT — AppState: health, metrics, cfg, cache, store, dht client.
//! RO:WHY  — Centralized handles; ready/health truth.
//! RO:INVARIANTS — set ready last; clone handles; register metrics once.

pub mod metrics;
pub mod readiness;
pub mod shutdown;

use crate::{cache, config::Config, dht::client::DhtClient, store::Store};

pub struct AppState {
    pub cfg: Config,
    pub health: readiness::HealthState,
    pub metrics: metrics::Metrics,
    pub cache: cache::IndexCache,
    pub store: Store,
    pub dht: DhtClient,
}

impl AppState {
    pub async fn new(cfg: Config) -> anyhow::Result<Self> {
        let metrics = metrics::Metrics::new()?;
        let health = readiness::HealthState::new();
        let cache = cache::IndexCache::new(cfg.cache_ttl_secs);
        let store = Store::new(cfg.enable_sled)?;
        let dht = DhtClient::new();

        Ok(Self {
            cfg,
            health,
            metrics,
            cache,
            store,
            dht,
        })
    }
}
