use ron_kernel::{Bus, HealthState};
use ron_transport::{
    config::TransportConfig, metrics::TransportMetrics, spawn_transport, types::TransportEvent,
};
use std::{io::Write, net::TcpStream as StdTcp, sync::Arc, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_timeout_closes() -> anyhow::Result<()> {
    let mut cfg = TransportConfig::default();
    cfg.read_timeout = Duration::from_millis(50);
    cfg.idle_timeout = Duration::from_millis(100);
    cfg.name = "test";

    let metrics = TransportMetrics::new("ron");
    let health = Arc::new(HealthState::new());
    let bus: Bus<TransportEvent> = Bus::new();

    let (_jh, addr) = spawn_transport(cfg, metrics, health, bus, None).await?;

    // Connect and stay idle; server should close within ~idle_timeout.
    let mut s = StdTcp::connect(addr)?;
    tokio::time::sleep(Duration::from_millis(200)).await; // cross idle timeout

    // Poll for closure up to a small budget to avoid race with FIN propagation.
    let deadline = std::time::Instant::now() + Duration::from_millis(600);
    loop {
        match s.write(&[1, 2, 3]) {
            Ok(_) => {
                if std::time::Instant::now() >= deadline {
                    anyhow::bail!(
                        "expected write to fail after idle timeout (connection still open)"
                    );
                }
                // FIN may not have arrived yet; wait and retry.
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(e) => {
                use std::io::ErrorKind::*;
                assert!(
                    matches!(
                        e.kind(),
                        BrokenPipe | ConnectionReset | NotConnected | UnexpectedEof
                    ),
                    "unexpected error kind: {e}"
                );
                break;
            }
        }
    }
    Ok(())
}
