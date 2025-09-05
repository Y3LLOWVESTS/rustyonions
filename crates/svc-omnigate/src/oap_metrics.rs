// Prometheus metrics for OAP/1 (topic counters + reject totals).
// Standalone to avoid touching existing metrics.rs.

#![forbid(unsafe_code)]

use once_cell::sync::OnceCell;
use prometheus::{
    IntCounter, IntCounterVec, Opts, Registry, default_registry,
    register_int_counter, register_int_counter_vec,
};

static REJECT_TIMEOUT: OnceCell<IntCounter> = OnceCell::new();
static REJECT_FRAMES:  OnceCell<IntCounter> = OnceCell::new();
static REJECT_BYTES:   OnceCell<IntCounter> = OnceCell::new();
static DATA_BYTES:     OnceCell<IntCounterVec> = OnceCell::new();
static STREAMS:        OnceCell<IntCounterVec> = OnceCell::new();
static ACK_GRANTS:     OnceCell<IntCounter> = OnceCell::new();

pub fn init_oap_metrics() {
    // Idempotent; safe to call multiple times.
    let _ = REJECT_TIMEOUT.get_or_try_init(|| {
        register_int_counter!(Opts::new("oap_rejected_total", "Total rejected OAP streams")
            .const_label("reason", "timeout"))
    });
    let _ = REJECT_FRAMES.get_or_try_init(|| {
        register_int_counter!(Opts::new("oap_rejected_total", "Total rejected OAP streams")
            .const_label("reason", "too_many_frames"))
    });
    let _ = REJECT_BYTES.get_or_try_init(|| {
        register_int_counter!(Opts::new("oap_rejected_total", "Total rejected OAP streams")
            .const_label("reason", "too_many_bytes"))
    });
    let _ = ACK_GRANTS.get_or_try_init(|| {
        register_int_counter!(Opts::new("oap_ack_grants_total", "ACK grants issued"))
    });
    let _ = DATA_BYTES.get_or_try_init(|| {
        register_int_counter_vec!(
            Opts::new("oap_data_bytes_total", "OAP data bytes by topic"),
            &["topic"]
        )
    });
    let _ = STREAMS.get_or_try_init(|| {
        register_int_counter_vec!(
            Opts::new("oap_streams_total", "OAP stream count by topic"),
            &["topic"]
        )
    });
}

#[inline] pub fn inc_reject_timeout()           { REJECT_TIMEOUT.get().unwrap().inc(); }
#[inline] pub fn inc_reject_too_many_frames()   { REJECT_FRAMES.get().unwrap().inc(); }
#[inline] pub fn inc_reject_too_many_bytes()    { REJECT_BYTES.get().unwrap().inc(); }
#[inline] pub fn inc_ack_grants()               { ACK_GRANTS.get().unwrap().inc(); }
#[inline] pub fn add_data_bytes(topic: &str, n: u64) { DATA_BYTES.get().unwrap().with_label_values(&[topic]).inc_by(n); }
#[inline] pub fn inc_streams(topic: &str)            { STREAMS.get().unwrap().with_label_values(&[topic]).inc(); }
