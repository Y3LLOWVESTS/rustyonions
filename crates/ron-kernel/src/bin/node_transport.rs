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
        addr: "127.0.0.1:8090".parse::<SocketAddr>()?,
        name: "node_transport",
        max_conns: 256,
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(30),
    };

    let (_jh, bound) =
        spawn_transport(cfg, metrics.clone(), health.clone(), bus.clone(), None).await?;
    info!(%bound, "node transport listening");

    let _ = bus.publish(KernelEvent::Health {
        service: "node_transport".into(),
        ok: true,
    });

    ron_kernel::wait_for_ctrl_c().await?;
    Ok(())
}
