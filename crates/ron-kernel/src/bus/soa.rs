#![cfg(feature = "bus_soa")]

//! SoA (Structure-of-Arrays) ring backend for the in-process bus.

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

use crate::Metrics;
use tokio::sync::broadcast::error::RecvError;

#[cfg(feature = "bus_edge_notify")]
mod edge_helper {
    use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Weak};
    use tokio::sync::Notify;

    #[derive(Default)]
    pub struct EdgeSignal {
        pub notify: Notify,
        pub pending: AtomicBool,
    }

    pub struct EdgeNotify;

    impl EdgeNotify {
        #[inline]
        pub fn set_pending_and_notify(sig: &Arc<EdgeSignal>) -> bool {
            let prev = sig.pending.swap(true, Ordering::AcqRel);
            if !prev {
                sig.notify.notify_one();
            }
            !prev
        }
        #[inline]
        pub fn clear_pending_and_race_check(pending: &AtomicBool) -> bool {
            let _was_set = pending.swap(false, Ordering::AcqRel);
            pending.load(Ordering::Relaxed)
        }
    }

    /// Reader-friendly registry with occasional GC and O(1) "is there anyone?" fast-path.
    #[derive(Default)]
    pub struct EdgeRegistry {
        pub signals: parking_lot::RwLock<Vec<Weak<EdgeSignal>>>,
        pub gc_tick: AtomicUsize,
        pub active: AtomicUsize, // count of live edge subscribers
    }

    impl EdgeRegistry {
        pub fn register(&self, sig: &Arc<EdgeSignal>) {
            let mut w = self.signals.write();
            w.push(Arc::downgrade(sig));
            self.active.fetch_add(1, Ordering::Relaxed);
        }

        pub fn deregister(&self) {
            self.active.fetch_sub(1, Ordering::Relaxed);
        }

        /// Iterate with a read-lock. Return `true` if any dead Weak were seen.
        pub fn for_each_with_read<F: FnMut(&Arc<EdgeSignal>)>(&self, mut f: F) -> bool {
            let r = self.signals.read();
            let mut saw_dead = false;
            for w in r.iter() {
                if let Some(sig) = w.upgrade() {
                    f(&sig);
                } else {
                    saw_dead = true;
                }
            }
            saw_dead
        }

        /// Occasionally compact dead entries. Called rarely to avoid write-lock churn.
        pub fn maybe_gc(&self, saw_dead: bool) {
            if !saw_dead {
                return;
            }
            let tick = self.gc_tick.fetch_add(1, Ordering::Relaxed);
            if (tick & 63) != 0 {
                return;
            }
            let mut w = self.signals.write();
            w.retain(|weak| weak.strong_count() > 0);
        }
    }
}
#[cfg(feature = "bus_edge_notify")]
use edge_helper::{EdgeNotify, EdgeRegistry, EdgeSignal};

// ===== Slot ==================================================================

// Keep it lean: no per-message Arc; clone T under a Mutex like the bounded backend.
pub(crate) struct Slot<T> {
    seq: AtomicU64,
    msg: parking_lot::Mutex<Option<T>>,
    ready_mask: AtomicU64,
}
impl<T> Slot<T> {
    fn new() -> Self {
        Self {
            seq: AtomicU64::new(0),
            msg: parking_lot::Mutex::new(None),
            ready_mask: AtomicU64::new(0),
        }
    }
}

// ===== Bus ===================================================================

pub struct Bus<T: Clone + Send + 'static> {
    cap: usize,
    seq: Arc<AtomicU64>,
    slots: Arc<Vec<Slot<T>>>,

    subs_in_use: Arc<parking_lot::Mutex<[bool; 64]>>,
    sub_count: Arc<parking_lot::Mutex<usize>>,

    global_notify: Arc<Notify>,

    publishers: Arc<AtomicUsize>,
    closed: Arc<AtomicBool>,

    metrics: Option<Arc<Metrics>>,

    #[cfg(feature = "bus_edge_notify")]
    edge: Arc<EdgeRegistry>,
}

impl<T: Clone + Send + 'static> Bus<T> {
    pub fn new() -> Self { Self::with_capacity(1024) }

    pub fn with_capacity(cap: usize) -> Self {
        let mut v = Vec::with_capacity(cap);
        for _ in 0..cap { v.push(Slot::new()); }
        Self {
            cap,
            seq: Arc::new(AtomicU64::new(0)),
            slots: Arc::new(v),
            subs_in_use: Arc::new(parking_lot::Mutex::new([false; 64])),
            sub_count: Arc::new(parking_lot::Mutex::new(0)),
            global_notify: Arc::new(Notify::new()),
            publishers: Arc::new(AtomicUsize::new(1)),
            closed: Arc::new(AtomicBool::new(false)),
            metrics: None,
            #[cfg(feature = "bus_edge_notify")]
            edge: Arc::new(EdgeRegistry::default()),
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self { self.metrics = Some(metrics); self }

    #[inline] pub fn receiver_count(&self) -> usize { *self.sub_count.lock() }

    #[inline]
    fn current_mask(&self) -> u64 {
        let subs = self.subs_in_use.lock();
        let mut mask: u64 = 0;
        for bit in 0..64 { if subs[bit] { mask |= 1u64 << bit; } }
        mask
    }

    /// Writer order:
    /// 1) payload -> msg (under lock)
    /// 2) ready_mask.store(mask, Release)
    /// 3) seq.store(next, Release)
    #[inline]
    fn publish_inner(&self, val: T, mask: u64, do_wake: bool) -> usize {
        let rc = self.receiver_count();
        if rc == 0 {
            if let Some(m) = &self.metrics { m.bus_no_receivers_total.inc(); }
            return 0;
        }

        let next = self.seq.fetch_add(1, Ordering::AcqRel) + 1;
        let idx = (next as usize) % self.cap;

        {
            let mut guard = self.slots[idx].msg.lock();
            *guard = Some(val);
        }
        self.slots[idx].ready_mask.store(mask, Ordering::Release);
        self.slots[idx].seq.store(next, Ordering::Release);

        if let Some(m) = &self.metrics { m.bus_published_total.inc(); }

        if do_wake {
            #[cfg(feature = "bus_edge_notify")]
            {
                // O(1) fast-path: if no edge subscribers, skip the whole sweep.
                if self.edge.active.load(Ordering::Relaxed) != 0 {
                    self.edge_sweep();
                }
            }
            #[cfg(not(feature = "bus_edge_notify"))]
            self.global_notify.notify_waiters();
        }

        rc
    }

    pub fn publish(&self, msg: T) -> usize {
        let mask = self.current_mask();
        self.publish_inner(msg, mask, true)
    }

    #[cfg(feature = "bus_batch")]
    pub fn publish_many(&self, batch: &[T]) -> usize {
        if batch.is_empty() { return 0; }
        let rc = self.receiver_count();
        if rc == 0 {
            if let Some(m) = &self.metrics {
                m.bus_no_receivers_total.inc_by(batch.len() as u64);
                m.bus_batch_publish_total.inc();
                m.bus_batch_len_histogram.observe(batch.len() as f64);
            }
            return 0;
        }
        let mask = self.current_mask();
        for item in batch { let _ = self.publish_inner(item.clone(), mask, false); }

        #[cfg(feature = "bus_edge_notify")]
        {
            if self.edge.active.load(Ordering::Relaxed) != 0 {
                self.edge_sweep();
            }
        }
        #[cfg(not(feature = "bus_edge_notify"))]
        self.global_notify.notify_waiters();

        if let Some(m) = &self.metrics {
            m.bus_batch_publish_total.inc();
            m.bus_batch_len_histogram.observe(batch.len() as f64);
        }
        rc
    }

    pub fn subscribe(&self) -> Receiver<T> {
        let (id, _count) = {
            let mut used = self.subs_in_use.lock();
            let mut idx: Option<usize> = None;
            for i in 0..64 {
                if !used[i] { used[i] = true; idx = Some(i); break; }
            }
            let mut sc = self.sub_count.lock();
            if idx.is_some() { *sc += 1; }
            (idx.expect("up to 64 subscribers"), *sc)
        };

        let tail = self.seq.load(Ordering::Acquire);
        Receiver {
            bus_cap: self.cap,
            ring: self.slots.clone(),
            tail,
            id: id as u8,
            global_notify: self.global_notify.clone(),
            _metrics: self.metrics.clone(),
            subs_in_use: Arc::clone(&self.subs_in_use),
            sub_count: Arc::clone(&self.sub_count),
            closed: Arc::clone(&self.closed),
            seq: Arc::clone(&self.seq),
        }
    }

    #[cfg(feature = "bus_edge_notify")]
    pub fn subscribe_edge(&self) -> EdgeReceiver<T> {
        let inner = self.subscribe();
        let signal = Arc::new(EdgeSignal::default());
        self.edge.register(&signal);
        EdgeReceiver { inner, signal, registry: Arc::clone(&self.edge) }
    }

    #[inline]
    pub fn handle_recv(res: Result<T, RecvError>, metrics: Option<&Metrics>) -> Option<T> {
        match res {
            Ok(v) => Some(v),
            Err(RecvError::Lagged(_)) => { if let Some(m) = metrics { m.bus_receiver_lag_total.inc_by(1); } None }
            Err(RecvError::Closed) => None,
        }
    }

    #[cfg(feature = "bus_edge_notify")]
    fn edge_sweep(&self) {
        let mut sent = 0u64;
        let mut suppressed = 0u64;

        let saw_dead = self.edge.for_each_with_read(|sig| {
            if EdgeNotify::set_pending_and_notify(sig) { sent += 1; } else { suppressed += 1; }
        });
        self.edge.maybe_gc(saw_dead);

        if let Some(m) = &self.metrics {
            if sent > 0 { m.bus_notify_sends_total.inc_by(sent); }
            if suppressed > 0 { m.bus_notify_suppressed_total.inc_by(suppressed); }
        }
    }
}

impl<T: Clone + Send + 'static> Clone for Bus<T> {
    fn clone(&self) -> Self {
        self.publishers.fetch_add(1, Ordering::AcqRel);
        Self {
            cap: self.cap,
            seq: Arc::clone(&self.seq),
            slots: Arc::clone(&self.slots),
            subs_in_use: Arc::clone(&self.subs_in_use),
            sub_count: Arc::clone(&self.sub_count),
            global_notify: Arc::clone(&self.global_notify),
            publishers: Arc::clone(&self.publishers),
            closed: Arc::clone(&self.closed),
            metrics: self.metrics.clone(),
            #[cfg(feature = "bus_edge_notify")]
            edge: Arc::clone(&self.edge),
        }
    }
}

impl<T: Clone + Send + 'static> Drop for Bus<T> {
    fn drop(&mut self) {
        if self.publishers.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.closed.store(true, Ordering::Release);
            self.global_notify.notify_waiters();
        }
    }
}

// ===== Receiver ==============================================================

pub struct Receiver<T: Clone + Send + 'static> {
    pub(crate) bus_cap: usize,
    pub(crate) ring: Arc<Vec<Slot<T>>>,
    pub(crate) tail: u64,
    pub(crate) id: u8,
    pub(crate) global_notify: Arc<Notify>,
    pub(crate) _metrics: Option<Arc<Metrics>>,
    pub(crate) subs_in_use: Arc<parking_lot::Mutex<[bool; 64]>>,
    pub(crate) sub_count: Arc<parking_lot::Mutex<usize>>,
    pub(crate) closed: Arc<AtomicBool>,
    pub(crate) seq: Arc<AtomicU64>,
}

impl<T: Clone + Send + 'static> Receiver<T> {
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        loop {
            let next = self.tail + 1;
            let idx = (next as usize) % self.bus_cap;
            let slot = &self.ring[idx];

            let slot_seq = slot.seq.load(Ordering::Acquire);

            if slot_seq > next {
                let delta = slot_seq - next;
                self.tail = slot_seq - 1;
                return Err(RecvError::Lagged(delta));
            }

            if slot_seq == 0 || slot_seq < next {
                if self.closed.load(Ordering::Acquire) {
                    let cur = self.seq.load(Ordering::Acquire);
                    if cur < next { return Err(RecvError::Closed); }
                }
                self.global_notify.notified().await;
                continue;
            }

            let bit = 1u64 << (self.id as u64);
            let prev_mask = slot.ready_mask.fetch_and(!bit, Ordering::AcqRel);
            if (prev_mask & bit) == 0 {
                self.tail = slot_seq;
                return Err(RecvError::Lagged(1));
            }

            let cloned = {
                let guard = slot.msg.lock();
                let v = guard.as_ref().expect("payload must exist if bit was set");
                v.clone()
            };

            self.tail = slot_seq;
            return Ok(cloned);
        }
    }
}

impl<T: Clone + Send + 'static> Drop for Receiver<T> {
    fn drop(&mut self) {
        let mut used = self.subs_in_use.lock();
        used[self.id as usize] = false;
        let mut c = self.sub_count.lock();
        *c = c.saturating_sub(1);
    }
}

// ===== Edge Receiver =========================================================

#[cfg(feature = "bus_edge_notify")]
pub struct EdgeReceiver<T: Clone + Send + 'static> {
    pub(crate) inner: Receiver<T>,
    pub(crate) signal: Arc<EdgeSignal>,
    pub(crate) registry: Arc<EdgeRegistry>,
}

#[cfg(feature = "bus_edge_notify")]
impl<T: Clone + Send + 'static> EdgeReceiver<T> {
    #[inline]
    pub fn try_recv_now_or_never(&mut self) -> usize {
        let mut drained = 0usize;
        loop {
            let next = self.inner.tail + 1;
            let idx = (next as usize) % self.inner.bus_cap;
            let slot = &self.inner.ring[idx];

            let slot_seq = slot.seq.load(Ordering::Acquire);
            if slot_seq == 0 || slot_seq < next { break; }
            if slot_seq > next { self.inner.tail = slot_seq - 1; continue; }

            let bit = 1u64 << (self.inner.id as u64);
            let prev_mask = slot.ready_mask.fetch_and(!bit, Ordering::AcqRel);
            if (prev_mask & bit) == 0 { self.inner.tail = slot_seq; continue; }

            {
                let guard = slot.msg.lock();
                let _ = guard.as_ref().expect("payload must exist if bit was set");
            }

            self.inner.tail = slot_seq;
            drained += 1;
        }
        drained
    }

    pub async fn run_drain_loop(&mut self, _sub_index: usize) {
        loop {
            let mut total = 0usize;
            loop {
                let n = self.try_recv_now_or_never();
                if n == 0 { break; }
                total += n;
                if total >= 1024 { break; }
            }
            let raced = edge_helper::EdgeNotify::clear_pending_and_race_check(&self.signal.pending);
            if raced { continue; }
            if total == 0 {
                self.signal.notify.notified().await;
                continue;
            }
            self.signal.notify.notified().await;
        }
    }

    pub async fn drain(&mut self, max: usize) -> usize {
        if max == 0 { return 0; }
        let mut drained = 0usize;
        while drained < max {
            match self.inner.recv().await {
                Ok(_v) => drained += 1,
                Err(RecvError::Lagged(_)) => {},
                Err(RecvError::Closed) => break,
            }
        }
        drained
    }

    #[inline] pub async fn await_notify(&self) { self.signal.notify.notified().await; }
    pub fn pending(&self) -> &core::sync::atomic::AtomicBool { &self.signal.pending }
}

#[cfg(feature = "bus_edge_notify")]
impl<T: Clone + Send + 'static> Drop for EdgeReceiver<T> {
    fn drop(&mut self) {
        // Mark one fewer active edge subscriber; Weak will GC later.
        self.registry.deregister();
    }
}
