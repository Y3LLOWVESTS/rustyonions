//! RO:WHAT — Simple per-conn inflight limiter (MVP).
use crate::limits::MAX_INFLIGHT_FRAMES;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct Inflight {
    n: AtomicUsize,
}
impl Inflight {
    pub fn new() -> Self {
        Self {
            n: AtomicUsize::new(0),
        }
    }
    pub fn try_inc(&self) -> bool {
        let cur = self.n.load(Ordering::Relaxed);
        if cur >= MAX_INFLIGHT_FRAMES {
            return false;
        }
        self.n.fetch_add(1, Ordering::Relaxed);
        true
    }
    pub fn dec(&self) {
        self.n.fetch_sub(1, Ordering::Relaxed);
    }
}
