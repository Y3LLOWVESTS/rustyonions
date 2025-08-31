#![forbid(unsafe_code)]

use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let bus: Bus = Bus::new(1024);

    let delivered = bus
        .publish(KernelEvent::Health {
            service: "bus_demo".to_string(),
            ok: true,
        })
        .unwrap_or(0);
    println!("Published initial Health event to {delivered} subscriber(s).");

    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            info!(?ev, "bus_demo received event");
        }
    });

    wait_for_ctrl_c().await?;
    Ok(())
}
