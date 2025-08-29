//! Transport under Supervisor with live stats and restart logging.
//! Run: cargo run -p ron-kernel --bin transport_supervised

use anyhow::Result;
use std::time::Duration;
use tracing::{info, warn};

use ron_kernel::{
    Bus, Event, Shutdown, tracing_init,
    spawn_supervised, SupervisorOptions,
    TransportOptions, spawn_transport,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_init("info,ron_kernel=debug,transport_supervised=debug");
    info!("transport_supervised starting… (Ctrl-C to stop)");

    let bus = Bus::new(4096);
    let shutdown = Shutdown::new();

    // Log Bus events (including Supervisor restarts)
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            match ev {
                Event::Restart { service, reason } => {
                    warn!(service, %reason, "Supervisor Restart");
                }
                Event::ConnOpened { .. } | Event::ConnClosed { .. } | Event::BytesIn { .. } | Event::BytesOut { .. } => {
                    // quiet; transport_demo prints these loudly—supervised demo focuses on restarts + stats
                }
                other => info!(?other, "Event"),
            }
        }
    });

    // Supervisor-managed service: starts transport and runs until shutdown.
    let bus_for_service = bus.clone();
    let shutdown_for_service = shutdown.clone();
    let _svc = spawn_supervised(
        SupervisorOptions::new("transport_service"),
        bus.clone(),
        shutdown.clone(),
        move || {
            let bus = bus_for_service.clone();
            let shutdown = shutdown_for_service.clone();
            async move {
                let (handle, stats) = spawn_transport(TransportOptions::default(), bus.clone(), shutdown.clone()).await?;
                info!(addr=%handle.local_addr, "transport_service listening");
                // Print live stats every second until shutdown
                loop {
                    tokio::select! {
                        _ = shutdown.cancelled() => break,
                        _ = tokio::time::sleep(Duration::from_secs(1)) => {
                            if let Some(s) = stats.query().await {
                                info!(open=s.open, accepted=s.accepted, closed=s.closed, bin=s.bytes_in, bout=s.bytes_out, "transport stats");
                            } else {
                                break;
                            }
                        }
                    }
                }
                Ok::<_, anyhow::Error>(())
            }
        }
    );

    // Wait for Ctrl-C then clean shutdown of everything.
    tokio::signal::ctrl_c().await?;
    warn!("Ctrl-C received — stopping supervised transport");
    shutdown.cancel();
    tokio::time::sleep(Duration::from_millis(50)).await;

    info!("transport_supervised stopped cleanly.");
    Ok(())
}
