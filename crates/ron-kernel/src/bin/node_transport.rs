// crates/ron-kernel/src/bin/node_transport.rs
#![forbid(unsafe_code)]

use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent, Metrics};
use ron_kernel::transport::{spawn_transport, TransportConfig};
use ron_kernel::config::spawn_config_watcher;
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
    println!("Starting node_transport (transport listener demo)…");

    // --- Bus + Metrics -------------------------------------------------------
    let bus: Bus<KernelEvent> = Bus::new(1024);
    let metrics = Metrics::new();
    let health = metrics.health().clone();

    // Optional: fire a background no-op watcher to match the kernel stub.
    // (It returns a JoinHandle<()>; we keep it to avoid unused warnings.)
    let _cfg_task = spawn_config_watcher().await;

    // --- Transport configuration ---------------------------------------------
    let addr: SocketAddr = "127.0.0.1:54087".parse().expect("valid socket addr");
    let tcfg = TransportConfig {
        addr,
        name: "transport",
        max_conns: 256,
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(30),
    };

    // Subscribe to bus just to print interesting kernel events.
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            info!(?ev, "bus event");
        }
    });

    // --- Spawn the transport --------------------------------------------------
    // No TLS override for this demo (pass None). Handle spawn errors explicitly.
    let (handle, bound) = match spawn_transport(
        tcfg,
        metrics.clone(),
        health.clone(),
        bus.clone(),
        None, // tls_override: Option<Arc<tokio_rustls::rustls::ServerConfig>>
    )
    .await
    {
        Ok((h, addr)) => (h, addr),
        Err(e) => {
            warn!("spawn_transport failed: {e}");
            return;
        }
    };

    info!(%bound, "transport listening");
    println!("Transport listening on http://{bound}/ (echo demo). Press Ctrl-C to stop.");

    let _ = wait_for_ctrl_c().await;
    // Best-effort shutdown signal for anyone listening.
    let _ = bus.publish(KernelEvent::Shutdown);

    // Let the task wind down; we don't join it here to keep the demo simple.
    let _ = handle;
    println!("node_transport exiting");
}
