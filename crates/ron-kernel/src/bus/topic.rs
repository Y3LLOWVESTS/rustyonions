//! RO:WHAT — Topic-scoped buses (internal utility).
//! RO:WHY  — Allows internal modules/tests to use name-scoped buses without changing public API.
//! RO:INTERACTS — bus::bounded::Bus, metrics (optional).
//! RO:INVARIANTS — No cross-topic delivery; lazy create; no external re-export.
//! RO:METRICS/LOGS — bus_topics_total (gauge).
//! RO:CONFIG — N/A.
//! RO:SECURITY — N/A.

#![allow(dead_code)]

use std::{collections::HashMap, sync::Arc};
use parking_lot::RwLock;

use super::bounded::Bus;
use crate::metrics::exporter::Metrics;

pub type Topic = &'static str;

#[derive(Default)]
pub struct TopicBus<T: Clone + Send + 'static> {
    inner: RwLock<HashMap<Topic, Bus<T>>>,
    metrics: Option<Arc<Metrics>>,
}

impl<T: Clone + Send + 'static> TopicBus<T> {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            metrics: None,
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub fn topic(&self, name: Topic) -> Bus<T> {
        let mut guard = self.inner.write();
        if let Some(bus) = guard.get(name) {
            return bus.clone();
        }
        let bus = match &self.metrics {
            Some(m) => Bus::new().with_metrics(m.clone()),
            None => Bus::new(),
        };
        guard.insert(name, bus.clone());
        if let Some(m) = &self.metrics {
            // FIX: use the correct gauge field name from Metrics
            m.bus_topics_total.set(guard.len() as i64);
        }
        bus
    }
}
