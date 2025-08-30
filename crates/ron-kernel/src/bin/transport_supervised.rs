// crates/ron-kernel/src/bin/transport_supervised.rs
#![forbid(unsafe_code)]

use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent, Metrics};
use ron_kernel::transport::{spawn_transport, TransportConfig};
use std::{net::SocketAddr, time::Duration};
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    println!("Starting transport_supervised…");

    let bus: Bus<KernelEvent> = Bus::new(1024);
    let metrics = Metrics::new();
    let health = metrics.health().clone();

    let addr: SocketAddr = "127.0.0.1:54087".parse().unwrap();
    let cfg = TransportConfig {
        addr,
        name: "transport",
        max_conns: 256,
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(30),
    };

    // Simple supervisor loop: spawn transport; listen for Ctrl-C; exit.
    loop {
        info!("spawning transport…");
        let (_handle, bound) = match spawn_transport(
            cfg.clone(),
            metrics.clone(),
            health.clone(),
            bus.clone(),
            None, // tls_override
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                warn!("spawn_transport failed: {e}");
                tokio::time::sleep(Duration::from_millis(300)).await;
                continue;
            }
        };
        info!(%bound, "transport listening");

        // Wait for Ctrl-C to simulate a shutdown/crash and break the loop.
        let _ = wait_for_ctrl_c().await;
        let _ = bus.publish(KernelEvent::ServiceCrashed {
            service: "transport".into(),
            reason: "ctrl-c".into(),
        });
        break;
    }

    println!("transport_supervised exiting");
}
