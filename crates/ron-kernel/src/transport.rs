#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration as TokioDuration};

use tokio_rustls::rustls::ServerConfig as TlsServerConfig;

use crate::bus::Bus;
use crate::KernelEvent;
use crate::{HealthState, Metrics};

#[derive(Clone, Debug)]
pub struct TransportConfig {
    pub addr: SocketAddr,
    pub name: &'static str,
    pub max_conns: usize,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub idle_timeout: Duration,
}

pub async fn spawn_transport(
    cfg: TransportConfig,
    _metrics: Metrics,
    _health: Arc<HealthState>,
    bus: Bus,
    _tls_override: Option<TlsServerConfig>,
) -> std::io::Result<(JoinHandle<()>, SocketAddr)> {
    let listener = TcpListener::bind(cfg.addr).await?;
    let local_addr = listener.local_addr()?;

    let _ = bus.publish(KernelEvent::Health {
        service: cfg.name.to_string(),
        ok: true,
    });

    let permits = Arc::new(tokio::sync::Semaphore::new(cfg.max_conns));

    let name = cfg.name;
    let idle = cfg.idle_timeout;
    let write_to = cfg.write_timeout;

    let handle = tokio::spawn(async move {
        loop {
            let permit = permits.clone().acquire_owned().await;
            let permit = match permit {
                Ok(p) => p,
                Err(_) => break,
            };

            let (mut socket, peer) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    let _ = bus.publish(KernelEvent::ServiceCrashed {
                        service: name.to_string(),
                        reason: format!("accept error: {e}"),
                    });
                    continue;
                }
            };

            let bus_clone = bus.clone();

            tokio::spawn(async move {
                let idle_deadline = TokioDuration::from_secs(idle.as_secs().max(1));
                let _ = timeout(idle_deadline, async {
                    tokio::time::sleep(TokioDuration::from_millis(10)).await;
                })
                .await;

                let _ = timeout(TokioDuration::from_secs(write_to.as_secs().max(1)), async {
                    use tokio::io::AsyncWriteExt;
                    let _ = socket.write_all(b"").await;
                })
                .await;

                drop(permit);

                let _ = bus_clone.publish(KernelEvent::Health {
                    service: format!("{name}:{peer}"),
                    ok: true,
                });
            });
        }
    });

    Ok((handle, local_addr))
}
