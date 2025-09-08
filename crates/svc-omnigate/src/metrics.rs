#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct Metrics {
    pub requests_total: AtomicU64,
    pub bytes_in_total: AtomicU64,
    pub bytes_out_total: AtomicU64,
    pub rejected_overload_total: AtomicU64,
    pub rejected_not_found_total: AtomicU64,
    pub rejected_too_large_total: AtomicU64,
    pub inflight_current: AtomicU64, // gauge
}

impl Metrics {
    #[inline]
    pub fn inc_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    // Not all call sites wire this yet; keep it available but silence Clippy.
    #[allow(dead_code)]
    #[inline]
    pub fn add_bytes_in(&self, n: u64) {
        self.bytes_in_total.fetch_add(n, Ordering::Relaxed);
    }

    #[inline]
    pub fn add_bytes_out(&self, n: u64) {
        self.bytes_out_total.fetch_add(n, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_overload(&self) {
        self.rejected_overload_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_not_found(&self) {
        self.rejected_not_found_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_too_large(&self) {
        self.rejected_too_large_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inflight_inc(&self) -> u64 {
        self.inflight_current.fetch_add(1, Ordering::Relaxed) + 1
    }

    #[inline]
    pub fn inflight_dec(&self) {
        self.inflight_current.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn to_prom(&self) -> String {
        format!(
            concat!(
                "requests_total {}\n",
                "bytes_in_total {}\n",
                "bytes_out_total {}\n",
                "rejected_overload_total {}\n",
                "rejected_not_found_total {}\n",
                "rejected_too_large_total {}\n",
                "inflight_current {}\n",
            ),
            self.requests_total.load(Ordering::Relaxed),
            self.bytes_in_total.load(Ordering::Relaxed),
            self.bytes_out_total.load(Ordering::Relaxed),
            self.rejected_overload_total.load(Ordering::Relaxed),
            self.rejected_not_found_total.load(Ordering::Relaxed),
            self.rejected_too_large_total.load(Ordering::Relaxed),
            self.inflight_current.load(Ordering::Relaxed),
        )
    }
}


