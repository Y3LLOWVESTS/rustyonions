//! Readiness metrics helpers.
//! RO:WHAT   Export gauges used by `/readyz` truth table.
//! RO:LABELS none (singletons).
//! RO:METRICS
//!   - `gateway_ready_inflight_current` (gauge, `i64`)
//!   - `gateway_ready_error_rate_pct`   (gauge, `i64`)
//!   - `gateway_ready_queue_saturated`  (gauge, `i64`: 0/1)

use once_cell::sync::OnceCell;
use prometheus::{IntGauge, Opts};

fn inflight_gauge() -> &'static IntGauge {
    static G: OnceCell<IntGauge> = OnceCell::new();
    G.get_or_init(|| {
        let g = IntGauge::with_opts(Opts::new(
            "gateway_ready_inflight_current",
            "Current inflight across gateway",
        ))
        .expect("IntGauge");
        prometheus::register(Box::new(g.clone())).expect("register gateway_ready_inflight_current");
        g
    })
}

fn error_pct_gauge() -> &'static IntGauge {
    static G: OnceCell<IntGauge> = OnceCell::new();
    G.get_or_init(|| {
        let g = IntGauge::with_opts(Opts::new(
            "gateway_ready_error_rate_pct",
            "Observed 429/503 % over window",
        ))
        .expect("IntGauge");
        prometheus::register(Box::new(g.clone())).expect("register gateway_ready_error_rate_pct");
        g
    })
}

fn queue_sat_gauge() -> &'static IntGauge {
    static G: OnceCell<IntGauge> = OnceCell::new();
    G.get_or_init(|| {
        let g = IntGauge::with_opts(Opts::new(
            "gateway_ready_queue_saturated",
            "Queue saturated indicator",
        ))
        .expect("IntGauge");
        prometheus::register(Box::new(g.clone())).expect("register gateway_ready_queue_saturated");
        g
    })
}

/// Update inflight (safe cast; saturates at `i64::MAX`).
pub fn set_inflight(v: u64) {
    let as_i64 = i64::try_from(v).unwrap_or(i64::MAX);
    inflight_gauge().set(as_i64);
}

/// Update error percentage (0..=100 expected; safe cast).
pub fn set_error_pct(v: u64) {
    let as_i64 = i64::try_from(v).unwrap_or(i64::MAX);
    error_pct_gauge().set(as_i64);
}

/// Update saturation flag.
pub fn set_queue_saturated(v: bool) {
    queue_sat_gauge().set(i64::from(v));
}
