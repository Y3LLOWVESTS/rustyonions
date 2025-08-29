//! Simple typed broadcast bus for kernel/service events.
//! Lossy by design (telemetry/health): subscribers must keep up.
//! Backpressure via bounded broadcast capacity.

#![forbid(unsafe_code)]

use tokio::sync::broadcast;

/// System-wide kernel/service events.
/// Extend as needed; keep variants lightweight.
#[derive(Debug, Clone)]
pub enum KernelEvent {
    /// Initiate graceful shutdown of the node.
    Shutdown,
    /// Configuration changed; carries a monotonically increasing version.
    ConfigUpdated { version: u64 },
    /// A service crashed and was (or will be) restarted by the supervisor.
    ServiceCrashed { service: String, reason: String },
    /// Health/heartbeat update for a service.
    Health { service: String, ok: bool },
}

/// Bounded broadcast bus. New subscribers receive only events emitted after subscription.
/// Choose capacity to balance memory vs. drop risk.
#[derive(Clone)]
pub struct Bus<E: Clone + Send + 'static> {
    tx: broadcast::Sender<E>,
}

impl<E: Clone + Send + 'static> Bus<E> {
    /// Create a new bus with the given ring capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Subscribe to events emitted after this call.
    pub fn subscribe(&self) -> broadcast::Receiver<E> {
        self.tx.subscribe()
    }

    /// Publish an event to all current subscribers.
    /// Returns number of subscribers that received it.
    pub fn publish(&self, event: E) -> Result<usize, broadcast::error::SendError<E>> {
        self.tx.send(event)
    }

    /// Current subscriber count (approximate).
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bus_broadcasts() {
        let bus: Bus<KernelEvent> = Bus::new(16);
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();

        let sent = bus
            .publish(KernelEvent::ConfigUpdated { version: 1 })
            .unwrap();
        assert_eq!(sent, 2);

        let _ = a.recv().await.unwrap();
        let _ = b.recv().await.unwrap();

        let _ = bus.publish(KernelEvent::Shutdown).unwrap();
        let _ = a.recv().await.unwrap();
        let _ = b.recv().await.unwrap();
    }
}
