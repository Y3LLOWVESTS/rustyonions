// crates/ron-kernel/src/bin/transport_demo.rs
#![forbid(unsafe_code)]

use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent, Metrics};
use ron_kernel::transport::{spawn_transport, TransportConfig};
use std::{net::SocketAddr, time::Duration};
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    println!("Starting transport_demo…");

    let bus: Bus<KernelEvent> = Bus::new(1024);
    let metrics = Metrics::new();
    let health = metrics.health().clone();

    // Bind transport on a demo port.
    let addr: SocketAddr = "127.0.0.1:54088".parse().unwrap();
    let cfg = TransportConfig {
        addr,
        name: "transport_demo",
        max_conns: 128,
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(30),
    };

    // Spawn transport without TLS.
    let (_task, bound) =
        spawn_transport(cfg, metrics.clone(), health.clone(), bus.clone(), None)
            .await
            .expect("spawn transport");
    info!(%bound, "transport_demo listening (echo)");

    println!("Press Ctrl-C to stop…");
    // Explicitly ignore the Result; we only care about the signal.
    let _ = wait_for_ctrl_c().await;

    let _ = bus.publish(KernelEvent::Shutdown);
    println!("transport_demo exiting");
}
