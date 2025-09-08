#![forbid(unsafe_code)]
#![allow(clippy::expect_used)] // metric registration failures are programmer/config errors

use std::sync::OnceLock;
use prometheus::{IntCounter, IntCounterVec, register_int_counter, register_int_counter_vec};

static REJECT_TIMEOUT: OnceLock<IntCounter> = OnceLock::new();
static REJECT_FRAMES:  OnceLock<IntCounter> = OnceLock::new();
static REJECT_BYTES:   OnceLock<IntCounter> = OnceLock::new();

static DATA_BYTES: OnceLock<IntCounterVec> = OnceLock::new();
static STREAMS:    OnceLock<IntCounterVec> = OnceLock::new();

/// Initialize all OAP metrics exactly once.
pub fn init_oap_metrics() {
    // Counters
    REJECT_TIMEOUT.get_or_init(|| {
        register_int_counter!("oap_reject_timeout_total", "Rejected streams due to timeout")
            .expect("register oap_reject_timeout_total")
    });
    REJECT_FRAMES.get_or_init(|| {
        register_int_counter!("oap_reject_too_many_frames_total", "Rejected streams due to too many frames")
            .expect("register oap_reject_too_many_frames_total")
    });
    REJECT_BYTES.get_or_init(|| {
        register_int_counter!("oap_reject_too_many_bytes_total", "Rejected streams due to too many bytes")
            .expect("register oap_reject_too_many_bytes_total")
    });

    // Labeled counters
    DATA_BYTES.get_or_init(|| {
        register_int_counter_vec!(
            "oap_data_bytes_total",
            "Total data bytes observed per topic",
            &["topic"]
        )
        .expect("register oap_data_bytes_total")
    });

    STREAMS.get_or_init(|| {
        register_int_counter_vec!(
            "oap_streams_total",
            "Total streams started per topic",
            &["topic"]
        )
        .expect("register oap_streams_total")
    });
}

// ---- public helpers (no unwrap on Option) ----

#[inline] pub fn inc_reject_timeout()         { REJECT_TIMEOUT.get().expect("init first").inc(); }
#[inline] pub fn inc_reject_too_many_frames() { REJECT_FRAMES .get().expect("init first").inc(); }
#[inline] pub fn inc_reject_too_many_bytes()  { REJECT_BYTES  .get().expect("init first").inc(); }

#[inline]
pub fn add_data_bytes(topic: &str, n: u64) {
    DATA_BYTES.get().expect("init first").with_label_values(&[topic]).inc_by(n);
}

#[inline]
pub fn inc_streams(topic: &str) {
    STREAMS.get().expect("init first").with_label_values(&[topic]).inc();
}
