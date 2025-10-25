//! RO:WHAT — Thread-local metric buffers for hot-path counters (feature: metrics_buf).
//! RO:WHY  — PERF: remove atomics from publish path; flush deltas on a timer or threshold.
//! RO:INTERACTS — metrics::exporter::Metrics (Prometheus registry)
//! RO:INVARIANTS — no locks across .await on hot path; best-effort flush on drop
//! RO:METRICS — bus_metrics_tls_flush_total (+ existing counters)
//! RO:CONFIG — flush interval (ms), flush threshold (events)
//! RO:TEST — unit: tls_no_loss_on_drop(); fuzz: interleaved_flush_ordering()

#![cfg(feature = "metrics_buf")]

use prometheus::IntCounter;
use std::{cell::Cell, sync::Arc};
use tokio::sync::Mutex;

thread_local! {
    static PUBLISHED_BUF: Cell<u64> = const { Cell::new(0) };
    static NOTIFY_BUF:    Cell<u64> = const { Cell::new(0) };
}

// Shared sinks owned by the exporter; hot path writes to TLS cells and we flush into these.
#[derive(Clone)]
pub struct BufferedSinks {
    pub published: IntCounter,
    pub notify: IntCounter,
    pub tls_flush_total: IntCounter,
    // threshold for flushing TLS buffers
    flush_threshold: Arc<usize>,
}

impl BufferedSinks {
    pub fn new(
        published: IntCounter,
        notify: IntCounter,
        tls_flush_total: IntCounter,
        flush_threshold: usize,
    ) -> Self {
        // Guardrail: enforce a minimum of 64 to avoid per-message flush in prod.
        Self {
            published,
            notify,
            tls_flush_total,
            flush_threshold: Arc::new(flush_threshold.max(64)),
        }
    }

    #[inline]
    pub fn add_published(&self, n: u64) {
        if n == 0 {
            return;
        }
        PUBLISHED_BUF.with(|c| c.set(c.get().saturating_add(n)));
        self.maybe_flush();
    }

    #[inline]
    pub fn add_notify(&self, n: u64) {
        if n == 0 {
            return;
        }
        NOTIFY_BUF.with(|c| c.set(c.get().saturating_add(n)));
        self.maybe_flush();
    }

    #[inline]
    fn maybe_flush(&self) {
        let thr = *self.flush_threshold as u64;
        let mut do_flush = false;
        PUBLISHED_BUF.with(|c| {
            if c.get() >= thr {
                do_flush = true;
            }
        });
        NOTIFY_BUF.with(|c| {
            if c.get() >= thr {
                do_flush = true;
            }
        });
        if do_flush {
            self.flush();
        }
    }

    pub fn flush(&self) {
        let mut p = 0u64;
        let mut n = 0u64;
        PUBLISHED_BUF.with(|c| {
            p = c.get();
            c.set(0);
        });
        NOTIFY_BUF.with(|c| {
            n = c.get();
            c.set(0);
        });
        if p != 0 {
            self.published.inc_by(p);
        }
        if n != 0 {
            self.notify.inc_by(n);
        }
        if p != 0 || n != 0 {
            self.tls_flush_total.inc();
        }
    }
}

// Background pump handle (periodically flush TLS buffers into shared counters).
#[derive(Clone)]
pub struct FlushPump {
    sinks: BufferedSinks,
    // keep a stop latch if you want (not strictly required in the kernel's long-lived proc)
    stop: Arc<Mutex<bool>>,
}

impl FlushPump {
    pub fn new(sinks: BufferedSinks) -> Self {
        Self {
            sinks,
            stop: Arc::new(Mutex::new(false)),
        }
    }

    /// Convenience for exporter: build a pump from the hot counters facade.
    pub fn new_from_hot(hot: Arc<HotCounters>) -> Self {
        // Same module, can access the inner to clone sinks.
        Self::new(hot.0.clone())
    }

    pub async fn run(self, interval_ms: u64) {
        let mut ticker =
            tokio::time::interval(std::time::Duration::from_millis(interval_ms.max(1)));
        loop {
            ticker.tick().await;
            if *self.stop.lock().await {
                break;
            }
            self.sinks.flush();
        }
    }
}

// Public facade used by the bus (hot path).
#[derive(Clone)]
pub struct HotCounters(pub(super) BufferedSinks);

impl HotCounters {
    pub fn new(sinks: BufferedSinks) -> Self {
        Self(sinks)
    }
    #[inline]
    pub fn inc_published(&self) {
        self.0.add_published(1);
    }
    #[inline]
    pub fn add_published(&self, n: u64) {
        self.0.add_published(n);
    }
    #[inline]
    pub fn inc_notify(&self) {
        self.0.add_notify(1);
    }
}

/// Best-effort drop flush to avoid counter loss on thread teardown.
impl Drop for HotCounters {
    fn drop(&mut self) {
        self.0.flush();
    }
}
