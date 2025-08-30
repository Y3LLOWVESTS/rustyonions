// crates/ron-kernel/src/bus.rs

use tokio::sync::broadcast;

/// Minimal, cloneable event bus backed by a tokio broadcast channel.
#[derive(Clone)]
pub struct Bus<T: Clone + Send + 'static> {
    tx: broadcast::Sender<T>,
}

impl<T: Clone + Send + 'static> Bus<T> {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Subscribe to the bus.
    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.tx.subscribe()
    }

    /// Publish an event to all subscribers. Returns number of receivers.
    pub fn publish(&self, ev: T) -> usize {
        match self.tx.send(ev) {
            Ok(n) => n,
            Err(_e) => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum KernelEvent {
    /// Health/liveness signal per service.
    Health { service: String, ok: bool },
    /// Configuration was updated (bump version/count for hot-reload awareness).
    ConfigUpdated { version: u64 },
    /// A supervised service crashed (used by transport_supervised).
    ServiceCrashed { service: String, reason: String },
    /// Kernel-wide shutdown signal (placeholder for demos).
    Shutdown,
}
