//! RO:WHAT — Tiny interior-mutable holder for `Config` with get/set.
//! RO:WHY  — Several subsystems (watcher, HTTP surfaces) need a shared view.
//! RO:INTERACTS — Used by `config::watcher` to apply hot-reloads.
//! RO:INVARIANTS — Reads are lock-free clone; writes replace atomically.

use parking_lot::RwLock;
use std::sync::Arc;

use super::Config;

#[derive(Clone)]
pub struct ConfigCell {
    inner: Arc<RwLock<Config>>,
}

impl ConfigCell {
    pub fn new(init: Config) -> Self {
        Self {
            inner: Arc::new(RwLock::new(init)),
        }
    }

    /// Snapshot the current config (cheap clone).
    #[inline]
    pub fn get(&self) -> Config {
        self.inner.read().clone()
    }

    /// Replace with a new config, returning the old snapshot.
    #[inline]
    pub fn set(&self, new_cfg: Config) -> Config {
        let mut w = self.inner.write();
        let old = w.clone();
        *w = new_cfg;
        old
    }
}
