//! Bounded, non-blocking in-process broadcast bus.
//!
//! MOG (features):
//! - `bus_edge_notify`: coalesced per-subscriber wake via `pending` bit + disciplined drain.
//! - `bus_batch`: batch publishing API with single notify sweep (A2).
//! - `metrics_buf`: thread-local buffering for hot-path counters (publish/notify).

use std::sync::Arc;

use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

#[cfg(feature = "bus_edge_notify")]
use {
    std::sync::{Mutex, Weak},
    tokio::sync::Notify,
};

use crate::Metrics;

#[cfg(feature = "bus_edge_notify")]
use crate::bus::mog_edge_notify::{prom_metrics::PromMetrics, EdgeNotify};

/// Kernel bus wrapper around `tokio::broadcast`.
#[derive(Clone)]
pub struct Bus<T: Clone + Send + 'static> {
    tx: broadcast::Sender<T>,
    metrics: Option<Arc<Metrics>>,

    #[cfg(feature = "bus_edge_notify")]
    edge: Arc<EdgeRegistry>,
}

impl<T: Clone + Send + 'static> Bus<T> {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel::<T>(1024);
        Self {
            tx,
            metrics: None,
            #[cfg(feature = "bus_edge_notify")]
            edge: Arc::new(EdgeRegistry::default()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel::<T>(capacity);
        Self {
            tx,
            metrics: None,
            #[cfg(feature = "bus_edge_notify")]
            edge: Arc::new(EdgeRegistry::default()),
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    #[inline]
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Single publish (existing path), with optional TLS metrics buffering.
    pub fn publish(&self, msg: T) -> usize {
        let receivers = self.tx.receiver_count();

        if receivers == 0 {
            if let Some(m) = &self.metrics {
                // Count as "published attempt" + explicit "no receivers".
                #[cfg(feature = "metrics_buf")]
                {
                    if let Some(hot) = m.hot() {
                        hot.inc_published();
                    }
                }
                #[cfg(not(feature = "metrics_buf"))]
                {
                    m.bus_published_total.inc();
                }
                m.bus_no_receivers_total.inc();
            }
            return 0;
        }

        match self.tx.send(msg) {
            Ok(_) => {
                // Account publish on the hot path.
                if let Some(m) = &self.metrics {
                    #[cfg(feature = "metrics_buf")]
                    {
                        if let Some(hot) = m.hot() {
                            hot.inc_published();
                        }
                    }
                    #[cfg(not(feature = "metrics_buf"))]
                    {
                        m.bus_published_total.inc();
                    }
                }

                // Coalesced edge-notify sweep if enabled.
                #[cfg(feature = "bus_edge_notify")]
                self.edge_sweep();

                receivers
            }
            Err(_e) => {
                if let Some(m) = &self.metrics {
                    m.bus_dropped_total.inc();
                }
                0
            }
        }
    }

    /// A2: Batch publish with one notify sweep at the end (feature-gated).
    #[cfg(feature = "bus_batch")]
    pub fn publish_many(&self, batch: &[T]) -> usize {
        if batch.is_empty() {
            return 0;
        }

        let receivers = self.tx.receiver_count();
        if receivers == 0 {
            if let Some(m) = &self.metrics {
                // Visibility: attempted to publish N items with no listeners.
                #[cfg(feature = "metrics_buf")]
                {
                    if let Some(hot) = m.hot() {
                        hot.add_published(batch.len() as u64);
                    }
                }
                #[cfg(not(feature = "metrics_buf"))]
                {
                    m.bus_published_total.inc_by(batch.len() as u64);
                }
                m.bus_no_receivers_total.inc_by(batch.len() as u64);
                m.bus_batch_publish_total.inc();
                m.bus_batch_len_histogram.observe(batch.len() as f64);
            }
            return 0;
        }

        // Send all elements; if any send fails (closed), account drop and stop.
        let mut sent = 0usize;
        for item in batch {
            match self.tx.send(item.clone()) {
                Ok(_) => sent += 1,
                Err(_e) => {
                    if let Some(m) = &self.metrics {
                        m.bus_dropped_total.inc();
                    }
                    break;
                }
            }
        }

        // One coalesced edge-notify sweep (A2).
        #[cfg(feature = "bus_edge_notify")]
        self.edge_sweep();

        if let Some(m) = &self.metrics {
            if sent > 0 {
                #[cfg(feature = "metrics_buf")]
                {
                    if let Some(hot) = m.hot() {
                        hot.add_published(sent as u64);
                    }
                }
                #[cfg(not(feature = "metrics_buf"))]
                {
                    m.bus_published_total.inc_by(sent as u64);
                }
            }
            m.bus_batch_publish_total.inc();
            m.bus_batch_len_histogram.observe(batch.len() as f64);
        }

        receivers
    }

    /// Subscribe and get a classical broadcast `Receiver<T>` (feature-agnostic).
    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.tx.subscribe()
    }

    pub fn handle_recv(
        res: Result<T, broadcast::error::RecvError>,
        metrics: Option<&Metrics>,
    ) -> Option<T> {
        match res {
            Ok(v) => Some(v),
            Err(broadcast::error::RecvError::Lagged(n)) => {
                if let Some(m) = metrics {
                    m.bus_receiver_lag_total.inc_by(n as u64);
                }
                None
            }
            Err(broadcast::error::RecvError::Closed) => None,
        }
    }

    // === Internal: one sweep across live subscribers to deliver a single notify per sub ======

    #[cfg(feature = "bus_edge_notify")]
    fn edge_sweep(&self) {
        // Prometheus metrics for A1/A5 (counters + per-sub pending gauge internally updated
        // by the receiver drain loops). Small, stateless helper.
        let prom = PromMetrics::default();

        let mut sent = 0u64;
        let mut suppressed = 0u64;

        self.edge.with_signals(|signals| {
            signals.retain(|w| w.upgrade().is_some());
            for w in signals.iter() {
                if let Some(sig) = w.upgrade() {
                    // RELAXED is sufficient: visibility is handled by ring fences.
                    if EdgeNotify::maybe_mark_pending_and_should_wake_metrics(&sig.pending, &prom) {
                        sig.notify.notify_one();
                        sent += 1;
                    } else {
                        suppressed += 1;
                    }
                }
            }
        });

        if let Some(m) = &self.metrics {
            // Route 'sends' through TLS buffer when enabled; 'suppressed' stays direct.
            #[cfg(feature = "metrics_buf")]
            {
                if sent > 0 {
                    if let Some(hot) = m.hot() {
                        // Count one notify per sweep (keep it simple on hot path).
                        hot.inc_notify();
                    }
                }
            }
            #[cfg(not(feature = "metrics_buf"))]
            {
                if sent > 0 {
                    m.bus_notify_sends_total.inc_by(sent);
                }
            }

            if suppressed > 0 {
                m.bus_notify_suppressed_total.inc_by(suppressed);
            }
        }
    }

    // === MOG subscriber helpers (feature-gated) ============================================

    /// Subscribe with an internal edge signal (pending bit + Notify) used to coalesce wakes.
    #[cfg(feature = "bus_edge_notify")]
    pub fn subscribe_edge(&self) -> EdgeReceiver<T> {
        let rx = self.tx.subscribe();
        let signal = Arc::new(EdgeSignal::default());
        self.edge.register_signal(&signal);
        EdgeReceiver {
            signal,
            rx,
            metrics: self.metrics.clone(),
        }
    }
}

/// Receiver wrapper (unchanged shape).
pub struct Receiver<T: Clone + Send + 'static> {
    inner: broadcast::Receiver<T>,
    metrics: Option<Arc<Metrics>>,
}

impl<T: Clone + Send + 'static> Receiver<T> {
    pub fn new(inner: broadcast::Receiver<T>, metrics: Option<Arc<Metrics>>) -> Self {
        Self { inner, metrics }
    }

    pub async fn recv(&mut self) -> Option<T> {
        loop {
            match self.inner.recv().await {
                Ok(v) => return Some(v),
                Err(RecvError::Lagged(n)) => {
                    if let Some(m) = &self.metrics {
                        m.bus_receiver_lag_total.inc_by(n as u64);
                    }
                    continue;
                }
                Err(RecvError::Closed) => return None,
            }
        }
    }
}

/* ======== MOG helpers and types (feature-gated; minimal, generic-safe) ==================== */

#[cfg(feature = "bus_edge_notify")]
#[derive(Default)]
struct EdgeRegistry {
    signals: Mutex<Vec<Weak<EdgeSignal>>>,
}

#[cfg(feature = "bus_edge_notify")]
impl EdgeRegistry {
    fn register_signal(&self, sig: &Arc<EdgeSignal>) {
        self.signals.lock().unwrap().push(Arc::downgrade(sig));
    }
    fn with_signals<F: FnOnce(&mut Vec<Weak<EdgeSignal>>) -> ()>(&self, f: F) {
        let mut guard = self.signals.lock().unwrap();
        f(&mut guard);
    }
}

#[cfg(feature = "bus_edge_notify")]
#[derive(Default)]
struct EdgeSignal {
    pending: std::sync::atomic::AtomicBool,
    notify: Notify,
}

#[cfg(feature = "bus_edge_notify")]
pub struct EdgeReceiver<T: Clone + Send + 'static> {
    signal: Arc<EdgeSignal>,
    rx: broadcast::Receiver<T>,
    metrics: Option<Arc<Metrics>>,
}

#[cfg(feature = "bus_edge_notify")]
impl<T: Clone + Send + 'static> EdgeReceiver<T> {
    #[inline]
    pub fn try_recv_now_or_never(&mut self) -> usize {
        use tokio::sync::broadcast::error::TryRecvError::*;
        let mut drained = 0usize;
        loop {
            match self.rx.try_recv() {
                Ok(_msg) => drained += 1,
                Err(Empty) => break,
                Err(Lagged(n)) => {
                    if let Some(m) = &self.metrics {
                        m.bus_receiver_lag_total.inc_by(n as u64);
                    }
                }
                Err(Closed) => break,
            }
        }
        drained
    }

    #[inline]
    pub async fn await_notify(&self) {
        self.signal.notify.notified().await;
    }

    #[inline]
    pub fn pending(&self) -> &std::sync::atomic::AtomicBool {
        &self.signal.pending
    }
    #[inline]
    pub fn notify(&self) -> &Notify {
        &self.signal.notify
    }

    /// Disciplined drain loop (A5) with race check and per-sub pending gauge.
    pub async fn run_drain_loop(&mut self, sub_index: usize) {
        // Prom metrics handle `bus_sub_pending{sub}` updates.
        let prom = PromMetrics::default();

        loop {
            // Drain in bounded bursts to amortize wakes while keeping latency tight.
            let mut drained = 0usize;
            loop {
                let n = self.try_recv_now_or_never();
                if n == 0 {
                    break;
                }
                drained += n;
                if drained >= 1024 {
                    break;
                }
            }

            // Race check — if a publish raced our clear, keep draining (skip await).
            if EdgeNotify::after_drain_race_check(self.pending(), &prom, sub_index) {
                continue;
            }

            // Await next wake to avoid ping-pong.
            self.await_notify().await;
        }
    }
}
