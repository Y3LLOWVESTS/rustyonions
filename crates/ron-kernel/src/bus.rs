use std::net::SocketAddr;
use tokio::sync::broadcast;

/// Kernel-wide telemetry/notifications and lightweight signals.
#[derive(Debug, Clone)]
pub enum Event {
    // Kernel lifecycle
    KernelStarting,
    KernelShuttingDown,

    // Supervision
    Restart { service: &'static str, reason: String },

    // Transport telemetry
    ConnOpened { peer: SocketAddr },
    ConnClosed { peer: SocketAddr },
    BytesIn { n: u64 },
    BytesOut { n: u64 },
}

#[derive(Clone)]
pub struct Bus {
    tx: broadcast::Sender<Event>,
}

impl Bus {
    /// Capacity is the channel’s ring buffer size; older messages are dropped when full.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(16));
        Self { tx }
    }

    pub fn publish(&self, ev: Event) {
        let _ = self.tx.send(ev);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}
