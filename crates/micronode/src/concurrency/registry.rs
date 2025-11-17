//! RO:WHAT — Registry for named concurrency pools.
//! RO:WHY  — Let HTTP routes and worker pools share semaphores by logical name,
//!           rather than each constructing ad-hoc caps.
//! RO:INTERACTS — `ConcurrencyConfig`, `layers::concurrency::ConcurrencyLayer`.
//! RO:INVARIANTS —
//!   * Each known budget name maps to a bounded `Semaphore`.
//!   * Registry is immutable after construction (no runtime mutation).
//!   * Unknown names fall back to a safe, modest default (no panics).

use std::{collections::HashMap, sync::Arc};

use tokio::sync::Semaphore;

use super::{ConcurrencyConfig, ConcurrencyLimit};

/// Immutable registry of concurrency pools keyed by logical budget name.
#[derive(Debug)]
pub struct ConcurrencyRegistry {
    inner: HashMap<&'static str, Arc<Semaphore>>,
}

impl ConcurrencyRegistry {
    /// Build a registry from the static concurrency configuration.
    ///
    /// Each `ConcurrencyLimit` becomes a distinct semaphore keyed by `limit.name`.
    pub fn from_config(cfg: &ConcurrencyConfig) -> Self {
        let mut inner = HashMap::new();

        for limit in [cfg.http_admin, cfg.http_dev_echo, cfg.http_kv, cfg.http_facets] {
            Self::insert_limit(&mut inner, limit);
        }

        Self { inner }
    }

    fn insert_limit(inner: &mut HashMap<&'static str, Arc<Semaphore>>, limit: ConcurrencyLimit) {
        // If a name is duplicated in the config, last-one-wins. That should not
        // happen in practice, but this keeps behavior deterministic.
        inner.insert(limit.name, Arc::new(Semaphore::new(limit.max_inflight)));
    }

    /// Fetch a semaphore for the given logical budget name.
    ///
    /// If the name is unknown (e.g. future budgets added without updating the
    /// registry construction), we fall back to a modest default (256). This
    /// ensures callers never panic on a missing entry.
    pub fn get(&self, name: &'static str) -> Arc<Semaphore> {
        self.inner.get(name).cloned().unwrap_or_else(|| Arc::new(Semaphore::new(256)))
    }
}
