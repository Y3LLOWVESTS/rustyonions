//! Subscriber-side helpers (topic-style filters, timeouts, non-blocking polling).

#![forbid(unsafe_code)]

use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time;

use super::core::Bus;
use crate::KernelEvent;

/// Receive with timeout. If the receiver lagged, update bus counters and continue.
/// Returns `Some(event)` on success, or `None` on timeout/closed.
pub async fn recv_with_timeout(
    bus: &Bus,
    rx: &mut broadcast::Receiver<KernelEvent>,
    timeout_dur: Duration,
) -> Option<KernelEvent> {
    match time::timeout(timeout_dur, rx.recv()).await {
        Ok(Ok(ev)) => Some(ev),
        Ok(Err(broadcast::error::RecvError::Lagged(skipped))) => {
            bus.record_overflow(skipped as u64, format!("topic recv lagged by {skipped}"));
            // Try again immediately to pull the next available message.
            match rx.try_recv() {
                Ok(ev) => Some(ev),
                Err(broadcast::error::TryRecvError::Lagged(_)) => {
                    // Still lagging; we already counted, let caller loop/yield.
                    None
                }
                Err(_) => None,
            }
        }
        _ => None,
    }
}

/// Non-blocking poll. Returns `Some(event)` or `None` if empty/closed.
pub fn try_recv_now(
    bus: &Bus,
    rx: &mut broadcast::Receiver<KernelEvent>,
    service_label: &str,
) -> Option<KernelEvent> {
    match rx.try_recv() {
        Ok(ev) => Some(ev),
        Err(broadcast::error::TryRecvError::Lagged(_)) => {
            bus.record_minimal_overflow(service_label);
            None
        }
        _ => None,
    }
}

/// Topic-style filter: wait up to `timeout` for an event matching `pred`.
pub async fn recv_matching<F>(
    bus: &Bus,
    rx: &mut broadcast::Receiver<KernelEvent>,
    timeout: Duration,
    mut pred: F,
) -> Option<KernelEvent>
where
    F: FnMut(&KernelEvent) -> bool,
{
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            return None;
        }
        let rem = deadline - now;
        if let Some(ev) = recv_with_timeout(bus, rx, rem).await {
            if pred(&ev) {
                return Some(ev);
            }
        } else {
            // timed out or closed
            return None;
        }
    }
}
