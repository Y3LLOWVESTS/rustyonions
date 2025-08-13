#![forbid(unsafe_code)]
//! accounting: byte counters with a fixed-size, allocation-free ring buffer.
//!
//! - `CountingStream<S>` wraps any `Read+Write` and records bytes in/out.
//! - `Counters` tracks totals and 60 per-minute buckets via a ring buffer.
//!
//! This is intentionally tiny and std-only.

use std::io::{Read, Result as IoResult, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Wrapper that counts bytes read/written through `inner`.
pub struct CountingStream<S> {
    inner: S,
    ctrs: Counters,
}

impl<S> CountingStream<S> {
    pub fn new(inner: S, counters: Counters) -> Self {
        Self { inner, ctrs: counters }
    }

    pub fn counters(&self) -> Counters {
        self.ctrs.clone()
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Read> Read for CountingStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 { self.ctrs.add_in(n as u64); }
        Ok(n)
    }
}

impl<S: Write> Write for CountingStream<S> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let n = self.inner.write(buf)?;
        if n > 0 { self.ctrs.add_out(n as u64); }
        Ok(n)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.inner.flush()
    }
}

/// Snapshot of cumulative and per-minute counters.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub total_in: u64,
    pub total_out: u64,
    /// Oldest→newest, length 60, bytes per minute.
    pub per_min_in: [u64; 60],
    pub per_min_out: [u64; 60],
}

/// Internal, mutex-protected state for counters.
#[derive(Debug)]
struct State {
    total_in: u64,
    total_out: u64,
    ring_in: [u64; 60],
    ring_out: [u64; 60],
    idx: usize,        // points to the current minute bucket
    last_minute: i64,  // epoch minutes of idx
}

impl Default for State {
    fn default() -> Self {
        let now_min = epoch_minutes_now();
        Self {
            total_in: 0,
            total_out: 0,
            ring_in: [0; 60],
            ring_out: [0; 60],
            idx: 0,
            last_minute: now_min,
        }
    }
}

/// A shareable counter set with a fixed-size ring buffer (no allocations on hot path).
#[derive(Clone, Debug)]
pub struct Counters(Arc<Mutex<State>>);

impl Counters {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(State::default())))
    }

    /// Add bytes read in the current minute bucket.
    pub fn add_in(&self, n: u64) {
        let mut s = self.0.lock().unwrap();
        rotate_if_needed(&mut s);
        s.total_in = s.total_in.saturating_add(n);
        let idx = s.idx; // avoid aliasing (mutable + immutable) on `s`
        let newv = s.ring_in[idx].saturating_add(n);
        s.ring_in[idx] = newv;
    }

    /// Add bytes written in the current minute bucket.
    pub fn add_out(&self, n: u64) {
        let mut s = self.0.lock().unwrap();
        rotate_if_needed(&mut s);
        s.total_out = s.total_out.saturating_add(n);
        let idx = s.idx; // avoid aliasing (mutable + immutable) on `s`
        let newv = s.ring_out[idx].saturating_add(n);
        s.ring_out[idx] = newv;
    }

    /// Return a stable snapshot (copies the ring).
    pub fn snapshot(&self) -> Snapshot {
        let mut s = self.0.lock().unwrap();
        rotate_if_needed(&mut s);

        // Reorder so output is oldest→newest.
        let mut out_in = [0u64; 60];
        let mut out_out = [0u64; 60];
        // Oldest bucket is just after current idx.
        for i in 0..60 {
            let src = (s.idx + 1 + i) % 60;
            out_in[i] = s.ring_in[src];
            out_out[i] = s.ring_out[src];
        }

        Snapshot {
            total_in: s.total_in,
            total_out: s.total_out,
            per_min_in: out_in,
            per_min_out: out_out,
        }
    }

    /// Clear all rolling minute buckets (totals are preserved).
    pub fn reset_minutes(&self) {
        let mut s = self.0.lock().unwrap();
        s.ring_in = [0; 60];
        s.ring_out = [0; 60];
        s.idx = 0;
        s.last_minute = epoch_minutes_now();
    }
}

impl Default for Counters {
    fn default() -> Self {
        Self::new()
    }
}

// --- helpers ---

fn epoch_minutes_now() -> i64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0));
    (now.as_secs() / 60) as i64
}

/// Advance the ring buffer to the current minute, zeroing any skipped buckets.
fn rotate_if_needed(s: &mut State) {
    let now_min = epoch_minutes_now();
    if now_min == s.last_minute {
        return;
    }
    if now_min < s.last_minute {
        // System clock went backwards; treat as same minute to avoid mayhem.
        return;
    }
    // Number of minutes passed since last bucket.
    let delta = (now_min - s.last_minute) as usize;
    let steps = delta.min(60); // cap at ring length; more means the whole ring is stale.

    for _ in 0..steps {
        s.idx = (s.idx + 1) % 60;
        s.ring_in[s.idx] = 0;
        s.ring_out[s.idx] = 0;
    }
    s.last_minute = now_min;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_bytes_and_snapshots_rotate() {
        let c = Counters::new();

        // Simulate some I/O
        c.add_in(100);
        c.add_out(40);

        let snap1 = c.snapshot();
        assert_eq!(snap1.total_in, 100);
        assert_eq!(snap1.total_out, 40);

        // Force a rotation by manually tweaking internal state (white-box test)
        {
            let mut s = c.0.lock().unwrap();
            s.last_minute -= 1; // pretend a minute has passed
        }
        c.add_in(7);
        c.add_out(3);

        let snap2 = c.snapshot();
        assert_eq!(snap2.total_in, 107);
        assert_eq!(snap2.total_out, 43);

        // Oldest→newest ordering; last bucket should reflect the latest adds
        assert_eq!(snap2.per_min_in[59], 7);
        assert_eq!(snap2.per_min_out[59], 3);
    }

    #[test]
    fn reset_minutes_clears_ring_only() {
        let c = Counters::new();
        c.add_in(10);
        c.add_out(5);
        let before = c.snapshot();

        c.reset_minutes();
        let after = c.snapshot();

        assert_eq!(after.total_in, before.total_in);   // totals preserved
        assert_eq!(after.total_out, before.total_out); // totals preserved
        assert!(after.per_min_in.iter().all(|&v| v == 0));
        assert!(after.per_min_out.iter().all(|&v| v == 0));
    }
}
