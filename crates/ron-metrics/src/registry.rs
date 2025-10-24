//! RO:WHAT — Thin wrapper over prometheus::Registry preventing duplicate names.

use prometheus::{Error as PromError, Registry, Result as PromResult};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SafeRegistry {
    inner: Arc<Registry>,
    names: Arc<Mutex<HashSet<String>>>,
}

impl SafeRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Registry::new()),
            names: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn register<F>(&self, family_name: &str, register_fn: F) -> PromResult<()>
    where
        F: FnOnce(&Registry) -> PromResult<()>,
    {
        let mut g = self.names.lock().unwrap();
        if !g.insert(family_name.to_string()) {
            // Return a real prometheus::Error so callers can map it.
            return Err(PromError::Msg(format!(
                "duplicate family: {family_name}"
            )));
        }
        drop(g);
        register_fn(&self.inner)
    }

    pub fn gather(&self) -> Vec<prometheus::proto::MetricFamily> {
        self.inner.gather()
    }

    pub fn raw(&self) -> &Registry {
        &self.inner
    }
}
