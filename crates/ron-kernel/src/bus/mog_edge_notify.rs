// crates/ron-kernel/src/bus/mog_edge_notify.rs

/*!
MOG A1 + A5 — Edge-Triggered Notify + Disciplined Drain
Feature: `bus_edge_notify`

Internal helpers:
- Coalesce wakeups per subscriber using a `pending` bit (A1).
- Disciplined drain loop that clears `pending` safely and avoids ping-pong wakes (A5).

Public API: **None** (internal only). Wire these helpers into the bus internals.
Zero behavior change unless the bus calls into this module under the `bus_edge_notify` feature.
*/

#![cfg(feature = "bus_edge_notify")]
#![deny(unsafe_code)]

use core::sync::atomic::{AtomicBool, Ordering};

/// Minimal metrics hook used by this module.
/// Keep it decoupled; provide a Prometheus impl below (optional).
pub trait EdgeMetrics: Send + Sync + 'static {
    fn inc_notify_sent(&self) {}
    fn inc_notify_suppressed(&self) {}
    fn set_sub_pending(&self, _sub_idx: usize, _val: bool) {}
}
impl EdgeMetrics for () {}

/// Per-subscriber edge notifier (stateless; state lives in passed-in atomics).
#[derive(Default, Debug)]
pub struct EdgeNotify;

impl EdgeNotify {
    /// Publisher-side: set `pending` to true. If we transitioned 0→1, caller should send a wake.
    /// (Legacy signature — no metrics.)
    #[inline(always)]
    pub fn maybe_mark_pending_and_should_wake(pending: &AtomicBool) -> bool {
        // Relaxed is sufficient: message visibility comes from the ring’s Release/Acquire.
        let prev = pending.swap(true, Ordering::Relaxed);
        !prev
    }

    /// Publisher-side (metrics-aware): increments sends/suppressed counters.
    #[inline(always)]
    pub fn maybe_mark_pending_and_should_wake_metrics<M: EdgeMetrics>(
        pending: &AtomicBool,
        metrics: &M,
    ) -> bool {
        let prev = pending.swap(true, Ordering::Relaxed);
        if !prev {
            metrics.inc_notify_sent();
            true
        } else {
            metrics.inc_notify_suppressed();
            false
        }
    }

    /// Subscriber-side (legacy): clear `pending=false` after draining and perform a race check.
    /// Returns `true` if new work arrived during/after the clear.
    ///
    /// NOTE: Kept for compatibility, but prefer `after_drain_race_check()` below,
    /// which uses a stronger detection pattern.
    #[inline(always)]
    pub fn clear_pending_and_race_check(pending: &AtomicBool) -> bool {
        let _was_set = pending.swap(false, Ordering::Relaxed);
        pending.load(Ordering::Relaxed)
    }

    /// Subscriber-side: disciplined race-check after draining.
    ///
    /// Pattern:
    ///   drain_all();
    ///   if EdgeNotify::after_drain_race_check(pending, metrics, sub_idx) { continue; }
    ///   await_notify();
    ///
    /// Returns `true` if a publish raced after we cleared pending — the caller should
    /// skip awaiting and re-enter the drain loop immediately.
    #[inline(always)]
    pub fn after_drain_race_check<M: EdgeMetrics>(
        pending: &AtomicBool,
        metrics: &M,
        sub_idx: usize,
    ) -> bool {
        // 1) Clear pending (we are about to await).
        pending.store(false, Ordering::Relaxed);
        metrics.set_sub_pending(sub_idx, false);

        // 2) Race detect: if a publisher set it after our clear, swap(false) returns true.
        let raced = pending.swap(false, Ordering::Relaxed);
        if raced {
            // Re-arm to true so subsequent publishers get suppression (coalescing continues).
            pending.store(true, Ordering::Relaxed);
            metrics.set_sub_pending(sub_idx, true);
        }
        raced
    }

    /// Subscriber-side: disciplined drain loop.
    ///
    /// - `try_recv_now` must drain ALL available items and return the number drained.
    /// - `await_notify` waits for a single notification (e.g., `notify.notified().await`).
    pub async fn drain_loop<TryNow, AwaitFut, M>(
        &self,
        sub_idx: usize,
        pending: &AtomicBool,
        mut try_recv_now: TryNow,
        mut await_notify: impl FnMut() -> AwaitFut,
        metrics: &M,
    ) where
        TryNow: FnMut() -> usize,
        AwaitFut: core::future::Future<Output = ()>,
        M: EdgeMetrics,
    {
        metrics.set_sub_pending(sub_idx, true);
        loop {
            // 1) Drain everything currently available.
            loop {
                let n = try_recv_now();
                if n == 0 {
                    break;
                }
            }

            // 2) Clear pending + race check. If raced, keep draining (skip await).
            if Self::after_drain_race_check(pending, metrics, sub_idx) {
                continue;
            }

            // 3) Await next wake to avoid ping-pong.
            await_notify().await;
            metrics.set_sub_pending(sub_idx, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::AtomicBool;

    #[test]
    fn publisher_edges_are_coalesced() {
        let pending = AtomicBool::new(false);

        // First mark triggers a wake
        assert!(EdgeNotify::maybe_mark_pending_and_should_wake(&pending));
        // Further marks coalesce (no additional wakes)
        assert!(!EdgeNotify::maybe_mark_pending_and_should_wake(&pending));
        assert!(!EdgeNotify::maybe_mark_pending_and_should_wake(&pending));

        // After a normal after-drain race check with no race, we should be clear
        let raced = EdgeNotify::after_drain_race_check(&pending, &(), 0);
        assert!(!raced);

        // Next publish becomes a new edge again
        assert!(EdgeNotify::maybe_mark_pending_and_should_wake(&pending));
    }

    #[test]
    fn after_drain_no_race_returns_false() {
        let pending = AtomicBool::new(true);
        // Simulate: we drained; now we do the after-drain check; no concurrent publisher
        let raced = EdgeNotify::after_drain_race_check(&pending, &(), 0);
        assert!(!raced, "no publisher raced; should return false");
        assert!(!pending.load(Ordering::Relaxed), "pending remains false");
    }
}

/// Optional: Prometheus-backed metrics for A1/A5 (counters + per-sub gauge).
/// Enable by constructing `PromMetrics` and passing &PromMetrics to drain/publish paths.
#[cfg(feature = "bus_edge_notify")]
pub mod prom_metrics {
    use super::EdgeMetrics;
    use once_cell::sync::Lazy;
    use prometheus::{
        opts, register_int_counter, register_int_gauge_vec, IntCounter, IntGaugeVec,
    };

    static NOTIFY_SENDS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
        register_int_counter!(
            opts!("bus_notify_sends_total", "Notify calls performed on 0→1 edges")
        )
        .expect("register bus_notify_sends_total")
    });

    static NOTIFY_SUPPRESSED_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
        register_int_counter!(
            opts!(
                "bus_notify_suppressed_total",
                "Notify attempts suppressed because subscriber was already pending"
            )
        )
        .expect("register bus_notify_suppressed_total")
    });

    static SUB_PENDING: Lazy<IntGaugeVec> = Lazy::new(|| {
        register_int_gauge_vec!(
            "bus_sub_pending",
            "Pending bit per subscriber (0|1)",
            &["sub"]
        )
        .expect("register bus_sub_pending")
    });

    /// Prometheus-backed EdgeMetrics. Label is `sub` with the numeric index.
    #[derive(Clone, Default)]
    pub struct PromMetrics;

    impl EdgeMetrics for PromMetrics {
        #[inline(always)]
        fn inc_notify_sent(&self) {
            NOTIFY_SENDS_TOTAL.inc();
        }
        #[inline(always)]
        fn inc_notify_suppressed(&self) {
            NOTIFY_SUPPRESSED_TOTAL.inc();
        }
        #[inline(always)]
        fn set_sub_pending(&self, sub_idx: usize, val: bool) {
            SUB_PENDING
                .with_label_values(&[&format!("{sub_idx}")])
                .set(if val { 1 } else { 0 });
        }
    }
}
