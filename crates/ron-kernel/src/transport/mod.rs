//! Minimal TransportService with:
//! - TCP listener + global connection cap (Semaphore)
//! - Per-connection tasks with echo
//! - Bytes in/out telemetry via Bus
//! - Live stats channel (open/accepted/closed/bytes)
//! - Graceful shutdown

use std::{
    io,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    select,
    sync::{mpsc, oneshot, Semaphore},
    time::{timeout, Duration},
};
use tracing::{debug, error, info, warn};

use crate::{Bus, Event};
use crate::cancel::Shutdown;

#[derive(Debug, Clone)]
pub struct TransportOptions {
    /// Address to bind, e.g., "127.0.0.1:0" to pick a free port.
    pub bind_addr: String,
    /// Max concurrent connections allowed.
    pub max_conns: usize,
    /// Per-connection read buffer size.
    pub read_buf_bytes: usize,
    /// Read idle timeout per await (to avoid stuck tasks).
    pub read_timeout_ms: u64,
}

impl Default for TransportOptions {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:0".to_string(),
            max_conns: 1024,
            read_buf_bytes: 16 * 1024,
            read_timeout_ms: 15_000,
        }
    }
}

/// A lightweight handle with the bound address.
#[derive(Debug, Clone)]
pub struct TransportHandle {
    pub local_addr: SocketAddr,
}

/// Snapshot of live transport stats.
#[derive(Debug, Clone)]
pub struct TransportStatsSnapshot {
    pub open: u64,
    pub accepted: u64,
    pub closed: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

/// Client to query stats via oneshot responses.
#[derive(Clone)]
pub struct TransportStatsClient {
    tx: mpsc::Sender<oneshot::Sender<TransportStatsSnapshot>>,
}

impl TransportStatsClient {
    /// Query a live snapshot. Returns None if the service has shut down.
    pub async fn query(&self) -> Option<TransportStatsSnapshot> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send(reply_tx).await.ok()?;
        reply_rx.await.ok()
    }
}

#[derive(Default)]
struct TransportState {
    open: AtomicU64,
    accepted: AtomicU64,
    closed: AtomicU64,
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
}

impl TransportState {
    fn snapshot(&self) -> TransportStatsSnapshot {
        TransportStatsSnapshot {
            open: self.open.load(Ordering::Relaxed),
            accepted: self.accepted.load(Ordering::Relaxed),
            closed: self.closed.load(Ordering::Relaxed),
            bytes_in: self.bytes_in.load(Ordering::Relaxed),
            bytes_out: self.bytes_out.load(Ordering::Relaxed),
        }
    }
}

/// Start the transport accept loop and per-connection tasks.
/// Returns the bound address handle and a stats client.
pub async fn spawn_transport(
    opts: TransportOptions,
    bus: Bus,
    shutdown: Shutdown,
) -> Result<(TransportHandle, TransportStatsClient)> {
    let listener = TcpListener::bind(&opts.bind_addr).await?;
    let local_addr = listener.local_addr()?;
    info!(%local_addr, "Transport bound");

    // Global connection limiter
    let permits = Arc::new(Semaphore::new(opts.max_conns));

    // Stats state + query channel
    let state = Arc::new(TransportState::default());
    let (stats_tx, mut stats_rx) = mpsc::channel::<oneshot::Sender<TransportStatsSnapshot>>(16);
    let state_for_task = state.clone();
    tokio::spawn(async move {
        while let Some(reply_tx) = stats_rx.recv().await {
            let _ = reply_tx.send(state_for_task.snapshot());
        }
    });

    // Accept loop
    let acc_bus = bus.clone();
    let acc_shutdown = shutdown.clone();
    let acc_permits = permits.clone();
    let acc_opts = opts.clone();
    let acc_state = state.clone();

    tokio::spawn(async move {
        loop {
            select! {
                biased;

                _ = acc_shutdown.cancelled() => {
                    warn!("Transport accept loop: shutdown");
                    break;
                }

                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((stream, peer)) => {
                            if let Some(permit) = acc_permits.clone().try_acquire_owned().ok() {
                                acc_state.accepted.fetch_add(1, Ordering::Relaxed);
                                acc_state.open.fetch_add(1, Ordering::Relaxed);
                                acc_bus.publish(Event::ConnOpened { peer });
                                spawn_conn_task(
                                    stream, peer, acc_bus.clone(), acc_shutdown.clone(),
                                    acc_opts.read_buf_bytes, acc_opts.read_timeout_ms,
                                    acc_state.clone(), permit
                                );
                            } else {
                                // Too many connections; politely refuse.
                                if let Err(e) = refuse(stream).await {
                                    debug!(?e, "refuse connection failed");
                                }
                            }
                        }
                        Err(e) => {
                            // Transient accept error; don't spin.
                            error!(?e, "accept error");
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                    }
                }
            }
        }
    });

    Ok((
        TransportHandle { local_addr },
        TransportStatsClient { tx: stats_tx },
    ))
}

async fn refuse(mut stream: TcpStream) -> io::Result<()> {
    let _ = stream.write_all(b"BUSY\r\n").await;
    let _ = stream.shutdown().await;
    Ok(())
}

fn spawn_conn_task(
    mut stream: TcpStream,
    peer: SocketAddr,
    bus: Bus,
    shutdown: Shutdown,
    read_buf_bytes: usize,
    read_timeout_ms: u64,
    state: Arc<TransportState>,
    _permit: tokio::sync::OwnedSemaphorePermit,
) {
    tokio::spawn(async move {
        let mut buf = vec![0u8; read_buf_bytes];

        loop {
            select! {
                _ = shutdown.cancelled() => {
                    debug!(%peer, "conn: shutdown");
                    break;
                }

                readres = timeout(Duration::from_millis(read_timeout_ms), stream.read(&mut buf)) => {
                    match readres {
                        Err(_) => {
                            // Idle timeout; drop the connection.
                            debug!(%peer, "conn: read timeout");
                            break;
                        }
                        Ok(Ok(0)) => {
                            // EOF
                            debug!(%peer, "conn: EOF");
                            break;
                        }
                        Ok(Ok(n)) => {
                            state.bytes_in.fetch_add(n as u64, Ordering::Relaxed);
                            bus.publish(Event::BytesIn { n: n as u64 });

                            // Echo back (demo)
                            if let Err(e) = stream.write_all(&buf[..n]).await {
                                debug!(%peer, ?e, "conn: write error");
                                break;
                            }
                            state.bytes_out.fetch_add(n as u64, Ordering::Relaxed);
                            bus.publish(Event::BytesOut { n: n as u64 });
                        }
                        Ok(Err(e)) => {
                            debug!(%peer, ?e, "conn: read error");
                            break;
                        }
                    }
                }
            }
        }

        state.closed.fetch_add(1, Ordering::Relaxed);
        state.open.fetch_sub(1, Ordering::Relaxed);
        bus.publish(Event::ConnClosed { peer });
    });
}
