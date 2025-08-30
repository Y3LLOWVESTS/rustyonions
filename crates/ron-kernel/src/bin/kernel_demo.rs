// crates/ron-kernel/src/bin/kernel_demo.rs
#![forbid(unsafe_code)]

use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent, Metrics};
use std::net::SocketAddr;
use tracing::{info};
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    println!("Starting kernel_demo…");

    // Bus + metrics
    let bus: Bus<KernelEvent> = Bus::new(1024);
    let metrics = Metrics::new();
    let health = metrics.health().clone();

    // Mark this demo as healthy so /healthz reports OK.
    health.set("kernel_demo", true);

    // Serve admin endpoints (tuple return per Final Blueprint; no unwrap on a Result).
    let admin_addr: SocketAddr = "127.0.0.1:9096".parse().expect("valid socket addr");
    let (_admin_task, bound) = metrics.clone().serve(admin_addr).await;
    println!("Admin endpoints: /metrics /healthz /readyz at http://{bound}/");

    // Print bus events
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            info!(?ev, "bus event");
        }
    });

    // Emit a couple of demo events
    let _ = bus.publish(KernelEvent::Health {
        service: "kernel_demo".to_string(),
        ok: true,
    });
    let _ = bus.publish(KernelEvent::ConfigUpdated { version: 1 });

    println!("Press Ctrl-C to shutdown…");
    // Ignore the Result intentionally; we only need the signal.
    let _ = wait_for_ctrl_c().await;

    // Shutdown signals
    let _ = bus.publish(KernelEvent::Shutdown);
    health.set("kernel_demo", false);

    println!("kernel_demo exiting");
}
