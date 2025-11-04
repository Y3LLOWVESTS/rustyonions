//! Gateway readiness gauges (prometheus).
//! RO:WHY  Keep names distinct (`gateway_ready_*`) to avoid collisions with any
//!         existing `ready_*` metrics registered elsewhere.

use prometheus::{IntGauge, Opts};
use std::sync::OnceLock;

const G_INFLIGHT: &str = "gateway_ready_inflight_current";
const G_ERROR_PCT: &str = "gateway_ready_error_rate_pct";
const G_QUEUE_SAT: &str = "gateway_ready_queue_saturated";

fn inflight_gauge() -> &'static IntGauge {
    static G: OnceLock<IntGauge> = OnceLock::new();
    G.get_or_init(|| {
        let g = IntGauge::with_opts(Opts::new(G_INFLIGHT, "Current inflight across gateway"))
            .expect("IntGauge");
        prometheus::register(Box::new(g.clone()))
            .expect("register gateway_ready_inflight_current");
        g
    })
}
fn error_pct_gauge() -> &'static IntGauge {
    static G: OnceLock<IntGauge> = OnceLock::new();
    G.get_or_init(|| {
        let g = IntGauge::with_opts(Opts::new(G_ERROR_PCT, "Observed 429/503 % over window"))
            .expect("IntGauge");
        prometheus::register(Box::new(g.clone()))
            .expect("register gateway_ready_error_rate_pct");
        g
    })
}
fn queue_sat_gauge() -> &'static IntGauge {
    static G: OnceLock<IntGauge> = OnceLock::new();
    G.get_or_init(|| {
        let g = IntGauge::with_opts(Opts::new(G_QUEUE_SAT, "Queue saturated indicator"))
            .expect("IntGauge");
        prometheus::register(Box::new(g.clone()))
            .expect("register gateway_ready_queue_saturated");
        g
    })
}

/// Setters used by the readiness sampler.
pub fn set_inflight(v: u64) {
    let _ = inflight_gauge().set(v as i64);
}
pub fn set_error_pct(v: u64) {
    let _ = error_pct_gauge().set(v as i64);
}
pub fn set_queue_saturated(v: bool) {
    let _ = queue_sat_gauge().set(if v { 1 } else { 0 });
}
