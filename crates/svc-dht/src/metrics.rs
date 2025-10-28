//! RO:WHAT — Prometheus metrics for svc-dht
//! RO:WHY — Observability; Concerns: PERF/GOV
//! RO:INTERACTS — rpc/http /metrics; bootstrap/pipeline update counters
//! RO:INVARIANTS — register once; cheap hot path

use once_cell::sync::Lazy;
use prometheus::{
    register_histogram, register_int_counter, Encoder, Histogram, IntCounter, TextEncoder,
};

pub struct DhtMetrics {
    pub lookups_total: IntCounter,
    pub provides_total: IntCounter,
    pub lookup_latency_seconds: Histogram,
    pub lookup_hops: Histogram,
}

impl DhtMetrics {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            lookups_total: register_int_counter!("dht_lookups_total", "Total DHT lookups")?,
            provides_total: register_int_counter!("dht_provides_total", "Total DHT provides")?,
            lookup_latency_seconds: register_histogram!(
                "dht_lookup_latency_seconds",
                "Lookup latency seconds",
                vec![0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0]
            )?,
            lookup_hops: register_histogram!(
                "dht_lookup_hops",
                "Lookup hop count",
                vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
            )?,
        })
    }

    /// RO:WHAT — Record one lookup completion with latency + hop count.
    pub fn observe_lookup(&self, dur: std::time::Duration, hops: u32) {
        self.lookups_total.inc();
        self.lookup_latency_seconds.observe(dur.as_secs_f64());
        self.lookup_hops.observe(hops as f64);
    }

    pub fn encode() -> anyhow::Result<String> {
        static ENC: Lazy<TextEncoder> = Lazy::new(TextEncoder::new);
        let mf = prometheus::gather();
        let mut buf = Vec::with_capacity(8 * 1024);
        ENC.encode(&mf, &mut buf)?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }
}
