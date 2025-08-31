//! Broadcast event bus for the microkernel.
//!
//! Design goals (per Final Blueprint):
//! - Cloneable, bounded, non-blocking bus built on tokio::broadcast.
//! - Never block the kernel: on pressure, receivers may lag; we *observe* that,
//!   increment metrics later (next PR), and publish a *throttled* crash-style event.
//! - Keep the public surface compatible with existing subscribers that call
//!   `bus.subscribe()` and `rx.recv().await`.
//!
//! This file introduces *optional* lag-aware receiving via the helper
//! [`recv_lag_aware`] that you can adopt incrementally in services.

#![forbid(unsafe_code)]

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::KernelEvent;

/// Cloneable microkernel event bus backed by `tokio::sync::broadcast`.
#[derive(Clone, Debug)]
pub struct Bus {
    tx: broadcast::Sender<KernelEvent>,
    capacity: usize,

    // Throttling for overflow/crash signaling. Per-bus, cheap, and thread-safe.
    last_overflow_utc: Arc<AtomicU64>,
    overflow_throttle_secs: u64,
}

impl Bus {
    /// Create a new bus with a bounded capacity.
    ///
    /// Capacity is the size of the underlying broadcast ring-buffer. Receivers
    /// that can't keep up will observe `RecvError::Lagged(n)`.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self {
            tx,
            capacity,
            last_overflow_utc: Arc::new(AtomicU64::new(0)),
            overflow_throttle_secs: 5, // sensible default; configurable later
        }
    }

    /// Returns the configured capacity of this bus.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Subscribe to the bus. Standard broadcast semantics.
    ///
    /// This keeps the public API compatible with existing code that does:
    /// ```
    /// let mut rx = bus.subscribe();
    /// while let Ok(ev) = rx.recv().await { /* ... */ }
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

    /// Internal: publish a *throttled* "bus-overflow" crash-style event.
    ///
    /// We signal via `KernelEvent::ServiceCrashed { service: "bus-overflow", reason }`
    /// at most once per `overflow_throttle_secs`. This keeps noisy receivers from
    /// spamming the bus when they're perpetually lagging.
    fn publish_overflow_throttled(&self, reason: String) {
        let now = epoch_secs();
        let last = self.last_overflow_utc.load(Ordering::Relaxed);
        if now.saturating_sub(last) >= self.overflow_throttle_secs {
            if self
                .last_overflow_utc
                .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                let _ = self.publish(KernelEvent::ServiceCrashed {
                    service: "bus-overflow".to_string(),
                    reason,
                });
            }
        }
    }
}

/// Receive with lag awareness and throttled overflow signaling.
///
/// - If a receiver is too slow, `tokio::broadcast` yields `RecvError::Lagged(n)`.
/// - We *log* the condition and publish one **throttled** crash-style event via the bus,
///   embedding the service label and lag count in the reason.
/// - Then we immediately retry until we either obtain an event or hit a different error.
///
/// This helper is **opt-in** and does not break existing code that uses `rx.recv().await`.
pub async fn recv_lag_aware(
    rx: &mut broadcast::Receiver<KernelEvent>,
    bus: &Bus,
    service_label: &str,
) -> Result<KernelEvent, broadcast::error::RecvError> {
    loop {
        match rx.recv().await {
            Ok(ev) => return Ok(ev),
            // ✅ Fixed: match arm must use `=>`, not `{` directly.
            Err(broadcast::error::RecvError::Lagged(n)) => {
                let reason = format!(
                    "{service} receiver lagged by {n} events",
                    service = service_label
                );
                warn!(%service_label, lagged = n, "bus receiver lag detected");
                bus.publish_overflow_throttled(reason);
                // Continue the loop to attempt receiving the next message.
                continue;
            }
            Err(other) => {
                // Closed or other terminal errors bubble up to caller.
                debug!(%service_label, error = %other, "bus receiver error");
                return Err(other);
            }
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
