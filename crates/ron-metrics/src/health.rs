//! RO:WHAT — In-memory readiness map with snapshot semantics.

use parking_lot::RwLock;
use std::{collections::BTreeMap, sync::Arc};

pub type ServiceName = String;

#[derive(Clone)]
pub struct HealthState {
    inner: Arc<RwLock<BTreeMap<ServiceName, bool>>>,
}

impl HealthState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
    pub fn set(&self, service: ServiceName, ok: bool) {
        let mut w = self.inner.write();
        w.insert(service, ok);
    }
    pub fn snapshot(&self) -> BTreeMap<ServiceName, bool> {
        self.inner.read().clone()
    }
    pub fn all_ready(&self) -> bool {
        self.inner.read().values().all(|v| *v)
    }
}
