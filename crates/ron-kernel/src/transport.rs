//! Minimal TCP transport server with metrics + health + bus events.
//! Adds:
//!   - Connection cap (bounded concurrency) via Semaphore
//!   - Read/Write timeouts + idle timeout
//!   - Echo protocol for demo visibility
//!
//! Metrics touched:
//!   - ron_active_connections (gauge) ++/-- on accept/close
//!   - ron_bytes_in_total / ron_bytes_out_total (counters)
//!   - ron_request_latency_seconds (histogram) per echo round-trip

#![forbid(unsafe_code)]

use crate::bus::Bus;
use crate::{HealthState, KernelEvent, Metrics};
use std::{io, net::SocketAddr, sync::Arc, time::Instant};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{OwnedSemaphorePermit, Semaphore},
    task::JoinHandle,
    time::{timeout, Duration},
};
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct TransportConfig {
    pub addr: SocketAddr,
    /// Service name used in logs and health
    pub name: &'static str,
    /// Max concurrent accepted connections (excess will be refused)
    pub max_conns: usize,
    /// Per-read timeout; closing connection if exceeded
    pub read_timeout: Duration,
    /// Per-write timeout; closing connection if exceeded
    pub write_timeout: Duration,
    /// Idle timeout (no reads for this long => close)
    pub idle_timeout: Duration,
}

/// Spawn the transport server. Returns the join handle and the bound local addr.
pub async fn spawn_transport(
    cfg: TransportConfig,
    metrics: Arc<Metrics>,
    health: HealthState,
    bus: Bus<KernelEvent>,
) -> io::Result<(JoinHandle<()>, SocketAddr)> {
    let listener = TcpListener::bind(cfg.addr).await?;
    let local = listener.local_addr()?;
    info!(addr=%local, name=cfg.name, "transport listening");

    // Mark healthy once bound.
    health.set(cfg.name, true);
    let _ = bus.publish(KernelEvent::Health {
        service: cfg.name.to_string(),
        ok: true,
    });

    // Global connection semaphore to cap concurrency.
    let sem = Arc::new(Semaphore::new(cfg.max_conns));

    // Main accept loop task.
    let handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, peer)) => {
                    // Try non-blocking acquire; if at cap, refuse quickly.
                    match sem.clone().try_acquire_owned() {
                        Ok(permit) => {
                            let m = metrics.clone();
                            let h = health.clone();
                            let b = bus.clone();
                            let cfg2 = cfg.clone();
                            tokio::spawn(async move {
                                if let Err(err) =
                                    handle_conn(cfg2, socket, peer, m, h, b, permit).await
                                {
                                    warn!(%err, "connection handler ended with error");
                                }
                            });
                        }
                        Err(_) => {
                            warn!(service=cfg.name, %peer, "refusing connection: at max_conns");
                            // best-effort: drop immediately; socket closes on drop
                            drop(socket);
                        }
                    }
                }
                Err(err) => {
                    error!(%err, "accept failed");
                    // Mark unhealthy on persistent errors.
                    health.set(cfg.name, false);
                    let _ = bus.publish(KernelEvent::Health {
                        service: cfg.name.to_string(),
                        ok: false,
                    });
                    // Brief backoff so we don't spin.
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
    });

    Ok((handle, local))
}

async fn handle_conn(
    cfg: TransportConfig,
    mut socket: TcpStream,
    peer: SocketAddr,
    metrics: Arc<Metrics>,
    _health: HealthState,
    bus: Bus<KernelEvent>,
    _permit: OwnedSemaphorePermit, // dropped on function exit -> frees a slot
) -> io::Result<()> {
    // Track active connections (drops back down on return).
    metrics.conns_gauge.inc();
    struct ConnGuard<'a>(&'a Metrics);
    impl<'a> Drop for ConnGuard<'a> {
        fn drop(&mut self) {
            self.0.conns_gauge.dec();
        }
    }
    let _guard = ConnGuard(&metrics);

    info!(service=cfg.name, %peer, "accepted");

    let mut buf = vec![0u8; 4096];
    let mut last_io = tokio::time::Instant::now();

    loop {
        // Enforce idle timeout.
        if last_io.elapsed() > cfg.idle_timeout {
            warn!(service=cfg.name, %peer, "idle timeout reached; closing");
            break;
        }

        // Read with timeout.
        let n = match timeout(cfg.read_timeout, socket.read(&mut buf)).await {
            Ok(Ok(n)) => n,
            Ok(Err(err)) => {
                // I/O error -> close.
                warn!(service=cfg.name, %peer, %err, "read error; closing");
                break;
            }
            Err(_) => {
                warn!(service=cfg.name, %peer, "read timeout; closing");
                break;
            }
        };

        if n == 0 {
            info!(service=cfg.name, %peer, "closed by peer");
            break;
        }
        last_io = tokio::time::Instant::now();
        metrics.bytes_in.inc_by(n as u64);

        let t0 = Instant::now();

        // Echo back with timeout and proper error handling.
        match timeout(cfg.write_timeout, socket.write_all(&buf[..n])).await {
            Ok(Ok(())) => {
                // wrote successfully
            }
            Ok(Err(err)) => {
                warn!(service=cfg.name, %peer, %err, "write error; closing");
                break;
            }
            Err(_) => {
                warn!(service=cfg.name, %peer, "write timeout; closing");
                break;
            }
        }

        metrics.bytes_out.inc_by(n as u64);
        metrics.req_latency.observe(duration_to_secs(t0.elapsed()));

        // Occasional heartbeat
        let _ = bus.publish(KernelEvent::Health {
            service: cfg.name.to_string(),
            ok: true,
        });
    }

    Ok(())
}

fn duration_to_secs(d: std::time::Duration) -> f64 {
    d.as_secs() as f64 + (d.subsec_nanos() as f64) * 1e-9
}
