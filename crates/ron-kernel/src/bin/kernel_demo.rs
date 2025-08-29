//! Tiny demo that just starts/stops the kernel primitives.
//! Run with: cargo run -p ron-kernel

use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

use ron_kernel::{
    Bus, Event, Shutdown,
    spawn_supervised, SupervisorOptions, tracing_init,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_init("info,ron_kernel=debug,kernel_demo=debug");
    info!("kernel_demo starting… (Ctrl-C to stop)");

    let bus = Bus::new(1024);
    let shutdown = Shutdown::new();

    // Example subscriber
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            match ev {
                Event::Restart { service, reason } => {
                    warn!(service, %reason, "Bus: restart observed");
                }
                ev => info!(?ev, "Bus event"),
            }
        }
    });

    // Supervise a trivial task that exits normally after a short delay.
    let opts = SupervisorOptions::new("demo_service");
    let _h = spawn_supervised(opts, bus.clone(), shutdown.clone(), || async {
        sleep(Duration::from_millis(500)).await;
        Ok::<_, anyhow::Error>(())
    });

    tokio::signal::ctrl_c().await?;
    warn!("Ctrl-C received — shutting down");
    shutdown.cancel();
    sleep(Duration::from_millis(50)).await;

    info!("kernel_demo stopped cleanly.");
    Ok(())
}
