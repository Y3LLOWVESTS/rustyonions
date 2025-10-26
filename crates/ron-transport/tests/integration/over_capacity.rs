use ron_kernel::{Bus, HealthState};
use ron_transport::{
    config::TransportConfig, metrics::TransportMetrics, spawn_transport, types::TransportEvent,
};
use std::{io::Write, net::TcpStream as StdTcp, sync::Arc, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn over_capacity_second_conn_dropped() -> anyhow::Result<()> {
    let mut cfg = TransportConfig::default();
    cfg.max_conns = 1;
    cfg.read_timeout = Duration::from_millis(200);
    cfg.idle_timeout = Duration::from_millis(500);
    cfg.name = "test";

    let metrics = TransportMetrics::new("ron");
    let health = Arc::new(HealthState::new());
    let bus: Bus<TransportEvent> = Bus::new();

    let (_jh, addr) = spawn_transport(cfg, metrics, health, bus, None).await?;

    // First connection holds the single permit.
    let _first = StdTcp::connect(addr)?;

    // Second connection should be dropped immediately by policy.
    let mut second = StdTcp::connect(addr)?;
    match second.write(&[9, 9, 9]) {
        Ok(_) => {
            tokio::time::sleep(Duration::from_millis(100)).await;
            match second.write(&[9, 9, 9]) {
                Ok(_) => anyhow::bail!("expected second connection to be dropped"),
                Err(_) => Ok(()),
            }
        }
        Err(_) => Ok(()),
    }
}
