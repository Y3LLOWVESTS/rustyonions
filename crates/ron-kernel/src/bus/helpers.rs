//! Lag-aware receiving helpers that integrate with overflow accounting.

#![forbid(unsafe_code)]

use tokio::sync::broadcast;
use tracing::{debug, warn};

use super::core::Bus;
use crate::KernelEvent;

/// Receive with lag awareness and throttled overflow signaling (blocking variant).
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
                let reason = format!(
                    "{service} receiver lagged by {n} events",
                    service = service_label
                );
                warn!(%service_label, lagged = n, "bus receiver lag detected");
                bus.record_overflow(n, reason);
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
            let reason = format!(
                "{service} receiver lagged by {n} events",
                service = service_label
            );
            warn!(%service_label, lagged = n, "bus receiver lag detected (try_recv)");
            bus.record_overflow(n, reason);
            // After acknowledging lag, attempt one more non-blocking read:
            rx.try_recv()
        }
        Err(e) => Err(e),
    }
}
