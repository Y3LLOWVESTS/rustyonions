//! Core bus implementation (bounded broadcast, overflow accounting, throttled signals).

#![forbid(unsafe_code)]

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;

use crate::KernelEvent;
use super::metrics::overflow_counter;

/// Cloneable microkernel event bus backed by `tokio::sync::broadcast`.
#[derive(Clone, Debug)]
pub struct Bus {
    pub(crate) tx: broadcast::Sender<KernelEvent>,
    capacity: usize,

    // Local accounting for dropped messages observed (across subscribers).
    dropped_total: Arc<AtomicU64>,

    // Throttling for overflow/crash signaling. Per-bus, cheap, and thread-safe.
    last_overflow_utc: Arc<AtomicU64>,
    pub(crate) overflow_throttle_secs: u64,
}

impl Bus {
    /// Create a new bus with a bounded capacity.
    ///
    /// Capacity is the size of the underlying broadcast ring-buffer. Receivers
    /// that can't keep up will observe `RecvError::Lagged(n)`.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity.max(1));

        // Pre-register the overflow metric so it’s exported as 0 even before any lag.
        let _ = overflow_counter();

        Self {
            tx,
            capacity,
            dropped_total: Arc::new(AtomicU64::new(0)),
            last_overflow_utc: Arc::new(AtomicU64::new(0)),
            overflow_throttle_secs: 5, // sensible default; configurable later
        }
    }

    /// Returns the configured capacity of this bus.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Cumulative dropped messages observed across all subscribers.
    pub fn dropped_total(&self) -> u64 {
        self.dropped_total.load(Ordering::Relaxed)
    }

    /// Subscribe to the bus. Standard broadcast semantics.
    ///
    /// This keeps the public API compatible with existing code that does:
    ///
    /// ```ignore
    /// // In an async context:
    /// // let bus = ron_kernel::bus::Bus::new(8);
    /// // let mut rx = bus.subscribe();
    /// // while let Ok(ev) = rx.recv().await {
    /// //     /* ... */
    /// // }
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<KernelEvent> {
        self.tx.subscribe()
    }

    /// Publish a kernel event to all current subscribers.
    ///
    /// Returns the number of subscribers the message was delivered to, or an
    /// error if there were none at the time of sending.
    pub fn publish(
        &self,
        ev: KernelEvent,
    ) -> Result<usize, broadcast::error::SendError<KernelEvent>> {
        self.tx.send(ev)
    }

    /// Publish but ignore `NoReceivers` (useful for fire-and-forget health pings).
    pub fn publish_lossy(&self, ev: KernelEvent) {
        let _ = self.tx.send(ev);
    }

    /// Internal: record that `n` messages were dropped due to lag and optionally
    /// publish a *throttled* "bus-overflow" crash-style event.
    pub(crate) fn record_overflow(&self, n: u64, reason: String) {
        self.dropped_total.fetch_add(n, Ordering::Relaxed);
        overflow_counter().inc_by(n);
        self.publish_overflow_throttled(reason);
    }

    /// Convenience used by helpers when exact `n` isn't available.
    pub(crate) fn record_minimal_overflow(&self, service_label: &str) {
        self.record_overflow(1, format!("{service} receiver lagged (minimal)", service = service_label));
    }

    /// Internal: publish a throttled ServiceCrashed("bus-overflow") with reason.
    fn publish_overflow_throttled(&self, reason: String) {
        let now = epoch_secs();
        let last = self.last_overflow_utc.load(Ordering::Relaxed);

        if now.saturating_sub(last) >= self.overflow_throttle_secs
            && self
                .last_overflow_utc
                .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            // NOTE: Keeping the `reason` field in your KernelEvent variant as-is.
            let _ = self.publish(KernelEvent::ServiceCrashed {
                service: "bus-overflow".to_string(),
                reason,
            });
        }
    }
}

#[inline]
fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
