#![forbid(unsafe_code)]

use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent};
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

    println!("Starting bus_demo …");
    let bus: Bus<KernelEvent> = Bus::new(64);
    let mut sub = bus.subscribe();
    println!(
        "Bus ready. Current subscriber count: {}",
        bus.subscriber_count()
    );

    // Subscriber task
    tokio::spawn(async move {
        while let Ok(ev) = sub.recv().await {
            // Also print to stdout for visibility
            println!("[bus_demo] received event: {:?}", ev);
            tracing::info!(?ev, "bus event");
        }
    });

    // Publish a few events right away
    let _ = bus.publish(KernelEvent::ConfigUpdated { version: 1 });
    let _ = bus.publish(KernelEvent::Health {
        service: "transport".into(),
        ok: true,
    });
    println!("Published: ConfigUpdated, Health");

    println!("Press Ctrl-C to publish Shutdown and exit…");
    wait_for_ctrl_c().await;
    let _ = bus.publish(KernelEvent::Shutdown);
    println!("Published: Shutdown. Exiting bus_demo.");
}
