use parking_lot::Mutex;
use std::io::{Read, Result as IoResult, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
pub struct Counters {
    inner: Arc<Mutex<Inner>>,
}
#[derive(Default)]
struct Inner {
    tx: u64,
    rx: u64,
    // simple rolling window: last N buckets of 1 minute each
    buckets: Vec<Bucket>,
    window: Duration,
    started: Instant,
}
#[derive(Clone, Copy, Default)]
struct Bucket { start: Instant, tx: u64, rx: u64 }

impl Counters {
    pub fn new(window: Duration) -> Self {
        let mut inner = Inner::default();
        inner.window = window;
        inner.started = Instant::now();
        Self { inner: Arc::new(Mutex::new(inner)) }
    }

    fn rotate(&self) {
        let mut g = self.inner.lock();
        let now = Instant::now();
        // keep ~window/60 buckets
        let keep = (g.window.as_secs().max(60) / 60) as usize;
        if g.buckets.is_empty() {
            g.buckets.push(Bucket { start: now, tx: 0, rx: 0 });
            return;
        }
        if g.buckets.len() < keep {
            g.buckets.push(Bucket { start: now, tx: 0, rx: 0 });
            return;
        }
        if now.duration_since(g.buckets.last().unwrap().start) >= Duration::from_secs(60) {
            g.buckets.remove(0);
            g.buckets.push(Bucket { start: now, tx: 0, rx: 0 });
        }
    }

    pub fn add_tx(&self, n: usize) {
        self.rotate();
        let mut g = self.inner.lock();
        g.tx += n as u64;
        if let Some(last) = g.buckets.last_mut() {
            last.tx += n as u64;
        }
    }
    pub fn add_rx(&self, n: usize) {
        self.rotate();
        let mut g = self.inner.lock();
        g.rx += n as u64;
        if let Some(last) = g.buckets.last_mut() {
            last.rx += n as u64;
        }
    }

    pub fn totals(&self) -> (u64, u64) {
        let g = self.inner.lock();
        (g.tx, g.rx)
    }

    pub fn window_totals(&self) -> (u64, u64) {
        let g = self.inner.lock();
        let (mut tx, mut rx) = (0, 0);
        for b in &g.buckets { tx += b.tx; rx += b.rx; }
        (tx, rx)
    }
}

pub struct CountingStream<S> {
    inner: S,
    counters: Counters,
}
impl<S> CountingStream<S> {
    pub fn new(inner: S, counters: Counters) -> Self { Self { inner, counters } }
}
impl<S: Read> Read for CountingStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let n = self.inner.read(buf)?;
        self.counters.add_rx(n);
        Ok(n)
    }
}
impl<S: Write> Write for CountingStream<S> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let n = self.inner.write(buf)?;
        self.counters.add_tx(n);
        Ok(n)
    }
    fn flush(&mut self) -> IoResult<()> { self.inner.flush() }
}
