//! RO:WHAT — Core Bus type wrapping a bounded Tokio broadcast channel
//! RO:WHY  — Provide monomorphic, bounded, lossy, observable-by-host semantics
//! RO:INTERACTS — config::BusConfig; internal::channel; event::Event
//! RO:INVARIANTS — bounded channel; capacity fixed; no background tasks; host updates metrics
//! RO:TEST — tests/* cover fanout, lag/overflow, cutover

use crate::{config::BusConfig, errors::BusError, event::Event, internal::channel};
use tokio::sync::broadcast::{Receiver, Sender};

/// Bounded in-process broadcast bus (lossy for lagging receivers).
pub struct Bus {
    tx: Sender<Event>,
    capacity: usize,
}

impl Bus {
    /// Construct a new Bus from a config (or default).
    pub fn new(cfg: impl Into<BusConfig>) -> Result<Self, BusError> {
        let cfg = cfg.into();
        cfg.validate().map_err(BusError::Config)?;
        let capacity = cfg.capacity as usize;
        let (tx, _rx) = channel::bounded::<Event>(capacity);
        // Drop the initial receiver; users will call subscribe(). No background tasks here.
        Ok(Self { tx, capacity })
    }

    /// Cloneable sender handle for publishers.
    pub fn sender(&self) -> Sender<Event> {
        self.tx.clone()
    }

    /// Unique receiver for a single subscriber task.
    ///
    /// Pattern: **one receiver per task** to avoid unintended sharing/races.
    pub fn subscribe(&self) -> Receiver<Event> {
        self.tx.subscribe()
    }

    /// The bounded queue capacity (messages).
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}
