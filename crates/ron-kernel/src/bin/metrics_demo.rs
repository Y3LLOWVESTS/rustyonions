#![forbid(unsafe_code)]

use ron_kernel::Metrics;
use std::net::SocketAddr;
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    println!("Starting metrics_demo …");

    let m = Metrics::new();
    let addr: SocketAddr = "127.0.0.1:9095".parse().unwrap();
    let (_handle, bound) = m.clone().serve(addr).await;
    println!("Metrics at http://{}/metrics", bound);

    // Drive a couple of counters/gauges for demonstration
    for n in 0..10 {
        m.bytes_in.inc_by(1024);
        m.bytes_out.inc_by(2048);
        m.req_latency.observe(0.005);
        m.conns_gauge.set((n % 5) as i64); // <- removed extra parens
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    println!("metrics_demo exiting");
}
