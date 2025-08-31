#![forbid(unsafe_code)]

use ron_kernel::{
    transport::{spawn_transport, TransportConfig},
    Bus, HealthState, KernelEvent, Metrics,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let bus: Bus = Bus::new(1024);
    let metrics = Metrics::new();
    let health = Arc::new(HealthState::new());

    let cfg = TransportConfig {
        addr: "127.0.0.1:8089".parse::<SocketAddr>()?,
        name: "transport_supervised",
        max_conns: 64,
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(30),
    };

    let (_jh, bound) =
        spawn_transport(cfg, metrics.clone(), health.clone(), bus.clone(), None).await?;
    info!(%bound, "transport supervised listening");

    let _ = bus.publish(KernelEvent::Health {
        service: "transport_supervised".into(),
        ok: true,
    });

    ron_kernel::wait_for_ctrl_c().await?;
    Ok(())
}
