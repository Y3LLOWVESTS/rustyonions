//! RO:WHAT — Minimal loopback listener smoke.
//! RO:WHY  — Verify spawn_transport() binds and runs without TLS.
//! RO:INTERACTS — TransportConfig, TransportMetrics, HealthState, Bus<TransportEvent>.

use ron_kernel::{Bus, HealthState};
use ron_transport::{
    config::TransportConfig, metrics::TransportMetrics, spawn_transport, types::TransportEvent,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = TransportConfig::default();
    let metrics = TransportMetrics::new("ron");
    let health = Arc::new(HealthState::new());

    // Event bus: we won't consume events here, but the type is now TransportEvent.
    let bus: Bus<TransportEvent> = Bus::new();

    let (_jh, addr) = spawn_transport(cfg, metrics, health, bus, None).await?;
    println!("ron-transport listening on {}", addr);

    tokio::signal::ctrl_c().await.ok();
    Ok(())
}
