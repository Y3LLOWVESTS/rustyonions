//! Broadcast event bus for the microkernel.
//!
//! Design goals (per Final Blueprint):
//! - Cloneable, bounded, non-blocking bus built on tokio::broadcast.
//! - Never block the kernel: on pressure, receivers may lag; we observe that,
//!   increment metrics, and publish a *throttled* crash-style event.
//! - Keep the public surface compatible with existing subscribers that call
//!   `bus.subscribe()` and `rx.recv().await`.
//!
//! This file provides *optional* lag-aware receiving via the helper
//! [`recv_lag_aware`] you can adopt incrementally in services.

#![forbid(unsafe_code)]

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, OnceLock,
};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;
use tracing::{debug, warn};

use prometheus::{IntCounter, register_int_counter};

use crate::KernelEvent;

/// Global Prometheus counter for dropped events due to lag.
fn overflow_counter() -> &'static IntCounter {
    static C: OnceLock<IntCounter> = OnceLock::new();
    C.get_or_init(|| {
        register_int_counter!(
            "bus_overflow_dropped_total",
            "Total KernelEvent messages dropped due to subscriber lag/overflow"
        )
        .expect("register bus_overflow_dropped_total")
    })
}

/// Cloneable microkernel event bus backed by `tokio::sync::broadcast`.
#[derive(Clone, Debug)]
pub struct Bus {
    tx: broadcast::Sender<KernelEvent>,
    capacity: usize,

    // Local accounting for dropped messages observed (across subscribers).
    dropped_total: Arc<AtomicU64>,

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
        let (tx, _rx) = broadcast::channel(capacity.max(1));

        // NEW: pre-register the overflow metric so it’s exported as 0 even before any lag.
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

    /// Internal: record that `n` messages were dropped due to lag and optionally
    /// publish a *throttled* "bus-overflow" crash-style event with `reason`.
    fn record_overflow(&self, n: u64, reason: String) {
        self.dropped_total.fetch_add(n, Ordering::Relaxed);
        overflow_counter().inc_by(n);
        self.publish_overflow_throttled(reason);
    }

    /// Internal: publish a throttled ServiceCrashed("bus-overflow") with reason.
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
/// - We log the condition, increment counters, and publish one **throttled**
///   crash-style event via the bus, embedding the service label and lag count.
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
            Err(broadcast::error::RecvError::Lagged(n)) => {
                let reason = format!("{service} receiver lagged by {n} events", service = service_label);
                warn!(%service_label, lagged = n, "bus receiver lag detected");
                bus.record_overflow(n as u64, reason);
                continue;
            }
            Err(other) => {
                debug!(%service_label, error = %other, "bus receiver error");
                return Err(other);
            }
        }
    }
}

/// Non-blocking variant with the same lag accounting.
pub fn try_recv_lag_aware(
    rx: &mut broadcast::Receiver<KernelEvent>,
    bus: &Bus,
    service_label: &str,
) -> Result<KernelEvent, broadcast::error::TryRecvError> {
    match rx.try_recv() {
        Ok(ev) => Ok(ev),
        Err(broadcast::error::TryRecvError::Lagged(n)) => {
            let reason = format!("{service} receiver lagged by {n} events", service = service_label);
            warn!(%service_label, lagged = n, "bus receiver lag detected (try_recv)");
            bus.record_overflow(n as u64, reason);
            // After acknowledging lag, attempt one more non-blocking read:
            rx.try_recv()
        }
        Err(e) => Err(e),
    }
}

#[inline]
fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn lag_increments_and_throttles() {
        let bus = Bus::new(2);
        let mut rx_fast = bus.subscribe();
        let mut rx_slow = bus.subscribe();

        // Fill slow subscriber to force lag
        let _ = bus.publish(KernelEvent::Health { service: "a".into(), ok: true });
        let _ = bus.publish(KernelEvent::Health { service: "b".into(), ok: true });
        let _ = bus.publish(KernelEvent::Health { service: "c".into(), ok: true });

        // Drain fast normally
        let _ = timeout(Duration::from_millis(50), rx_fast.recv()).await.unwrap().unwrap();
        let _ = timeout(Duration::from_millis(50), rx_fast.recv()).await.unwrap().unwrap();
        let _ = timeout(Duration::from_millis(50), rx_fast.recv()).await.unwrap().unwrap();

        // Slow will detect lag and recover using helper
        let _ = timeout(Duration::from_millis(50), super::recv_lag_aware(&mut rx_slow, &bus, "test")).await.unwrap();

        assert!(bus.dropped_total() > 0);
    }
}
