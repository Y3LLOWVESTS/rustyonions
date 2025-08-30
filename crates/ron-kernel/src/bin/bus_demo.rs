// crates/ron-kernel/src/bin/bus_demo.rs
#![forbid(unsafe_code)]

use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent};
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    println!("Starting bus_demo…");

    // Create a broadcast bus for KernelEvent
    let bus: Bus<KernelEvent> = Bus::new(1024);

    // Subscribe and print received events in a background task
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            info!(?ev, "bus_demo: received event");
        }
    });

    // Publish an initial health event and report how many receivers saw it.
    let delivered = bus.publish(KernelEvent::Health {
        service: "bus_demo".to_string(),
        ok: true,
    });
    println!("Published initial Health event to {delivered} subscriber(s).");

    // Publish a config-bump demo event.
    let _ = bus.publish(KernelEvent::ConfigUpdated { version: 1 });

    println!("Press Ctrl-C to send Shutdown and exit…");
    // Intentionally ignore the Result; we only care about the signal.
    let _ = wait_for_ctrl_c().await;

    // Signal shutdown to any listeners and exit.
    let _ = bus.publish(KernelEvent::Shutdown);
    println!("bus_demo exiting");
}
