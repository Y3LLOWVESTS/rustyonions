use prometheus::{Histogram, HistogramOpts, IntCounter, IntCounterVec, Opts};

pub struct SandboxMetrics {
    pub token_trip_total: IntCounter,
    pub rejected_total: IntCounterVec, // labels: reason
    pub tarpit_ms_hist: Histogram,
}

impl SandboxMetrics {
    pub fn new() -> Self {
        let token_trip_total = IntCounter::new("honeytoken_trips_total", "Total honeytoken/decoy asset hits").unwrap();
        let rejected_total = IntCounterVec::new(
            Opts::new("sandbox_rejected_total", "Rejected bad requests"),
            &["reason"],
        ).unwrap();
        let tarpit_ms_hist = Histogram::with_opts(HistogramOpts::new("tarpit_ms_histogram", "Injected tarpit delays (ms)")).unwrap();

        prometheus::register(Box::new(token_trip_total.clone())).ok();
        prometheus::register(Box::new(rejected_total.clone())).ok();
        prometheus::register(Box::new(tarpit_ms_hist.clone())).ok();

        Self {
            token_trip_total,
            rejected_total,
            tarpit_ms_hist,
        }
    }
}
