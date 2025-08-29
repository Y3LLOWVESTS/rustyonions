#![forbid(unsafe_code)]

use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent, Metrics};
use ron_kernel::transport::{spawn_transport, TransportConfig};
use std::{net::SocketAddr, time::Duration};
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    println!("Starting node_transport (metrics + health + TCP transport)…");

    // Metrics + admin HTTP on fixed port
    let metrics = Metrics::new();
    let admin_addr: SocketAddr = "127.0.0.1:9095".parse().expect("parse admin addr");
    let (admin_handle, bound_admin) = metrics.clone().serve(admin_addr).await;
    println!(
        "Admin endpoints: /metrics /healthz /readyz at http://{}/",
        bound_admin
    );

    // Bus for kernel events
    let bus: Bus<KernelEvent> = Bus::new(64);
    let mut sub = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = sub.recv().await {
            println!("[node_transport] bus event: {:?}", ev);
            tracing::info!(?ev, "bus event");
        }
    });

    // Use the SAME health state the metrics server exposes
    let health = metrics.health().clone();

    // Transport configuration (cap + timeouts)
    let listen_addr: SocketAddr = "127.0.0.1:54087".parse().expect("parse listen addr");
    let cfg = TransportConfig {
        addr: listen_addr,
        name: "transport",
        max_conns: 256,                          // cap concurrent connections
        read_timeout: Duration::from_secs(10),   // per-read timeout
        write_timeout: Duration::from_secs(10),  // per-write timeout
        idle_timeout: Duration::from_secs(60),   // idle connection cutoff
    };

    // Spawn real TCP transport on fixed port (echo server)
    let (transport_handle, bound_listen) =
        spawn_transport(cfg, metrics.clone(), health.clone(), bus.clone())
            .await
            .expect("spawn transport");
    println!("Transport listening at {}", bound_listen);
    println!("Try in another terminal:");
    println!("  nc 127.0.0.1 54087");
    println!("  type: hello<enter> (echo)");
    println!("Also: curl http://127.0.0.1:9095/readyz  and  /metrics");

    // Periodic node heartbeat
    let bus_for_heartbeat = bus.clone();
    let heartbeat = tokio::spawn(async move {
        loop {
            let _ = bus_for_heartbeat.publish(KernelEvent::Health {
                service: "node".into(),
                ok: true,
            });
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });

    println!("Press Ctrl-C to shutdown…");
    wait_for_ctrl_c().await;

    let _ = bus.publish(KernelEvent::Shutdown);
    heartbeat.abort();
    transport_handle.abort();
    admin_handle.abort();
    println!("node_transport exiting");
}
