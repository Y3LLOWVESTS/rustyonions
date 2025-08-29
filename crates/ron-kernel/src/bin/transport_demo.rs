//! Demo that starts the TransportService, logs telemetry, prints live stats, and waits for Ctrl-C.
//! Run: cargo run -p ron-kernel --bin transport_demo
//! Then in another terminal: nc 127.0.0.1 <PORT>

use anyhow::Result;
use std::time::Duration;
use tracing::{info, warn};

use ron_kernel::{
    Bus, Event, Shutdown, tracing_init,
    TransportOptions, spawn_transport,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_init("info,ron_kernel=debug,transport_demo=debug");
    info!("transport_demo starting…  (Ctrl-C to stop)");

    let bus = Bus::new(4096);
    let shutdown = Shutdown::new();

    // Log transport events
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            match ev {
                Event::ConnOpened { peer } => info!(%peer, "ConnOpened"),
                Event::ConnClosed { peer } => info!(%peer, "ConnClosed"),
                Event::BytesIn { n } => info!(n, "BytesIn"),
                Event::BytesOut { n } => info!(n, "BytesOut"),
                other => info!(?other, "Event"),
            }
        }
    });

    // Start transport
    let opts = TransportOptions { bind_addr: "127.0.0.1:0".into(), ..Default::default() };
    let (handle, stats) = spawn_transport(opts, bus.clone(), shutdown.clone()).await?;
    info!(addr = %handle.local_addr, "Transport listening");

    // Periodic stats print
    let stats_task = tokio::spawn({
        let stats = stats.clone();
        async move {
            loop {
                if let Some(s) = stats.query().await {
                    info!(open=s.open, accepted=s.accepted, closed=s.closed, bin=s.bytes_in, bout=s.bytes_out, "stats");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                } else {
                    break;
                }
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    warn!("Ctrl-C received — shutting down transport");
    shutdown.cancel();

    // Small grace delay
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let _ = stats_task.await;

    info!("transport_demo stopped cleanly.");
    Ok(())
}
