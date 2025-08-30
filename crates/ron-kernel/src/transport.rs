// crates/ron-kernel/src/transport.rs

#![forbid(unsafe_code)]

use crate::bus::{Bus, KernelEvent};
use crate::metrics::{HealthState, Metrics};
use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio_rustls::{rustls::ServerConfig, TlsAcceptor};
use tracing::{info, warn};

/// Public config exported so bins can construct with a struct literal.
/// NOTE: bins set `name: "transport"` (a `&'static str`), so we use that type
/// to avoid forcing bins to call `.to_string()`.
#[derive(Clone)]
pub struct TransportConfig {
    pub addr: SocketAddr,
    pub name: &'static str,
    pub max_conns: usize,
    pub read_timeout: std::time::Duration,
    pub write_timeout: std::time::Duration,
    pub idle_timeout: std::time::Duration,
}

impl TransportConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        addr: SocketAddr,
        name: &'static str,
        max_conns: usize,
        read_timeout: std::time::Duration,
        write_timeout: std::time::Duration,
        idle_timeout: std::time::Duration,
    ) -> Self {
        Self { addr, name, max_conns, read_timeout, write_timeout, idle_timeout }
    }
}

/// Spawn the transport service (TCP/TLS listener).
///
/// Returns a JoinHandle and the bound address.
/// `tls_override`: if Some, wraps accepted sockets with TLS.
pub async fn spawn_transport(
    cfg: TransportConfig,
    metrics: Metrics,
    health: HealthState,
    bus: Bus<KernelEvent>,
    tls_override: Option<Arc<ServerConfig>>,
) -> Result<(JoinHandle<()>, SocketAddr)> {
    let listener = TcpListener::bind(cfg.addr).await?;
    let actual = listener.local_addr()?;
    info!(addr=%actual, "transport: listening");

    let permits = Arc::new(Semaphore::new(cfg.max_conns));
    let task = tokio::spawn(async move {
        health.set(cfg.name, true);
        loop {
            let (socket, _peer) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    let reason = format!("accept error: {e}");
                    warn!(%reason, "transport: listener error");
                    let _ = bus.publish(KernelEvent::ServiceCrashed {
                        service: cfg.name.to_string(),
                        reason,
                    });
                    break;
                }
            };

            let permits = permits.clone();
            let metrics = metrics.clone();
            let tls = tls_override.clone();
            let service_name = cfg.name.to_string();

            tokio::spawn(async move {
                let _permit = permits.acquire().await.unwrap();
                metrics.connections_active.inc();

                let res = if let Some(server) = tls {
                    let acceptor = TlsAcceptor::from(server);
                    handle_conn_tls(socket, acceptor, &metrics).await
                } else {
                    handle_conn_plain(socket, &metrics).await
                };

                metrics.connections_active.dec();
                if let Err(e) = res {
                    metrics.error_counter.with_label_values(&[&service_name]).inc();
                    tracing::error!("connection error: {e}");
                }
            });
        }
        health.set(cfg.name, false);
    });

    Ok((task, actual))
}

async fn handle_conn_plain(mut s: TcpStream, metrics: &Metrics) -> Result<()> {
    let mut buf = [0u8; 1024];
    let n = s.read(&mut buf).await?;
    if n > 0 {
        metrics.bytes_in.inc_by(n as u64);
        // Echo back in demos; real transport will hand off to overlay.
        s.write_all(&buf[..n]).await?;
        metrics.bytes_out.inc_by(n as u64);
    }
    Ok(())
}

async fn handle_conn_tls(
    s: TcpStream,
    acceptor: TlsAcceptor,
    metrics: &Metrics,
) -> Result<()> {
    let mut tls_stream = acceptor.accept(s).await?;
    let mut buf = [0u8; 1024];
    let n = tls_stream.read(&mut buf).await?;
    if n > 0 {
        metrics.bytes_in.inc_by(n as u64);
        tls_stream.write_all(&buf[..n]).await?;
        metrics.bytes_out.inc_by(n as u64);
    }
    Ok(())
}
