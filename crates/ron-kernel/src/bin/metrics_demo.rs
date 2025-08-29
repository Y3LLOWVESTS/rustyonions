#![forbid(unsafe_code)]

use ron_kernel::Metrics;
use std::{net::SocketAddr, time::Duration};
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    // Fall back to "info" if RUST_LOG is not set
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();

    // Fixed port so it's obvious where to curl
    let addr: SocketAddr = "127.0.0.1:9095".parse().expect("parse addr");

    let metrics = Metrics::new();
    let (handle, bound) = metrics.clone().serve(addr).await;

    println!(
        "Prometheus metrics listening at http://{}/metrics",
        bound
    );
    println!("Try: curl http://127.0.0.1:9095/metrics");

    // Toy loop to exercise metrics
    let m = metrics.clone();
    let worker = tokio::spawn(async move {
        let mut n = 0u64;
        loop {
            m.bytes_in.inc_by(512);
            m.bytes_out.inc_by(256);
            m.conns_gauge.set(((n % 5) as i64));
            m.restarts.with_label_values(&["overlay"]).inc();
            m.req_latency.observe(0.005 + (n as f64 % 10.0) * 0.001);
            n += 1;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    // Wait for Ctrl-C
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::warn!(%err, "ctrl_c signal error");
    }

    // Clean shutdown (demo)
    worker.abort();
    handle.abort();
    println!("metrics_demo exiting");
}
