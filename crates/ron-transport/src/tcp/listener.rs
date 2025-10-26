//! RO:WHAT — TCP accept loop with optional TLS, limits, metrics, cancel.
//! RO:INVARIANTS — readiness flips when bound; single writer; deadlines enforced.

use crate::config::TransportConfig;
use crate::conn::reader::{self, ReaderStats};
use crate::conn::writer;
use crate::metrics::TransportMetrics;
use crate::readiness::ReadyGate;
use crate::reason::RejectReason;
use crate::util::cancel::Cancel;
use crate::TlsServerConfig;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration, Instant};

/// Back-compat API (no shutdown handle). Internally creates a cancel token and drops it.
pub async fn spawn_listener(
    cfg: TransportConfig,
    metrics: TransportMetrics,
    health: Arc<ron_kernel::HealthState>,
    gate: ReadyGate,
    tls: Option<Arc<TlsServerConfig>>,
) -> anyhow::Result<(JoinHandle<()>, SocketAddr)> {
    let cancel = Cancel::new();
    spawn_listener_with_cancel(cfg, metrics, health, gate, tls, cancel).await
}

/// New API that takes a `Cancel` token so callers can trigger graceful shutdown.
pub async fn spawn_listener_with_cancel(
    cfg: TransportConfig,
    metrics: TransportMetrics,
    _health: Arc<ron_kernel::HealthState>,
    _gate: ReadyGate,
    tls: Option<Arc<TlsServerConfig>>,
    cancel: Cancel,
) -> anyhow::Result<(JoinHandle<()>, SocketAddr)> {
    let listener = TcpListener::bind(cfg.addr)
        .await
        .map_err(crate::error::TransportError::Bind)?;
    let addr = listener.local_addr().unwrap();
    let permits = Arc::new(Semaphore::new(cfg.max_conns));

    let jh = tokio::spawn(async move {
        tracing::info!(%addr, name=%cfg.name, "ron-transport listener bound");
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    tracing::info!(%addr, "listener shutdown requested");
                    break;
                }
                res = listener.accept() => {
                    match res {
                        Ok((stream, peer)) => {
                            let cfgc = cfg.clone();
                            let m = metrics.clone();
                            let tls_cfg = tls.clone();

                            // Connection limit: clone Arc before try_acquire_owned (it consumes its Arc).
                            match permits.clone().try_acquire_owned() {
                                Ok(permit) => {
                                    m.connections.with_label_values(&[cfg.name]).inc();
                                    tokio::spawn(handle_conn(stream, peer, cfgc, m, permit, tls_cfg));
                                }
                                Err(_) => {
                                    m.rejected_total
                                        .with_label_values(&[cfg.name, RejectReason::OverCapacity.as_str()])
                                        .inc();
                                    drop(stream);
                                }
                            }
                        }
                        Err(e) => {
                            metrics
                                .rejected_total
                                .with_label_values(&[cfg.name, RejectReason::Io.as_str()])
                                .inc();
                            tracing::warn!(error=%e, "accept failed; backing off");
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            }
        }
        tracing::info!(%addr, "listener exited");
    });

    Ok((jh, addr))
}

async fn handle_conn(
    stream: TcpStream,
    peer: SocketAddr,
    cfg: TransportConfig,
    metrics: TransportMetrics,
    _permit: OwnedSemaphorePermit, // holds a slot until this task ends
    tls: Option<Arc<TlsServerConfig>>,
) {
    tracing::debug!(%peer, "accepted");
    let started = Instant::now();

    let result = match maybe_tls(stream, tls).await {
        Ok(IoUpgraded::Plain(s)) => run_plain(s, &cfg, &metrics).await,
        #[cfg(feature = "tls")]
        Ok(IoUpgraded::Tls(s)) => run_tls(s, &cfg, &metrics).await,
        Err(e) => Err(e),
    };
    let elapsed = started.elapsed().as_secs_f64();

    match result {
        Ok(stats) => {
            metrics
                .bytes_in
                .with_label_values(&[cfg.name])
                .inc_by(stats.bytes_in as u64);
            metrics
                .latency_seconds
                .with_label_values(&[cfg.name])
                .observe(elapsed);
            tracing::debug!(%peer, bytes_in=%stats.bytes_in, dur=%elapsed, "closed ok");
        }
        Err(e) => {
            metrics
                .rejected_total
                .with_label_values(&[cfg.name, RejectReason::Io.as_str()])
                .inc();
            tracing::debug!(%peer, error=%e, "closed with error");
        }
    }
}

async fn run_plain(
    stream: TcpStream,
    cfg: &TransportConfig,
    metrics: &TransportMetrics,
) -> std::io::Result<ReaderStats> {
    use tokio::io::split;
    let (rd, wr) = split(stream);

    // Spawn writer (currently unused by upper layers; metrics ready).
    let (_wh, writer_task) = writer::spawn_writer(wr, cfg.name, metrics.clone());

    // Run reader until EOF/timeout/error.
    let stats = reader::run(rd, cfg.read_timeout, cfg.idle_timeout).await;

    // Drop handle, await the task to flush+shutdown (sends FIN).
    drop(_wh);
    let _ = writer_task.await;

    stats
}

#[cfg(feature = "tls")]
async fn run_tls(
    stream: tokio_rustls::server::TlsStream<TcpStream>,
    cfg: &TransportConfig,
    metrics: &TransportMetrics,
) -> std::io::Result<ReaderStats> {
    use tokio::io::split;
    let (rd, wr) = split(stream);

    // Spawn writer (TLS): will send close_notify during shutdown().
    let (_wh, writer_task) = writer::spawn_writer(wr, cfg.name, metrics.clone());

    // Reader loop (generic over AsyncRead).
    let stats = reader::run(rd, cfg.read_timeout, cfg.idle_timeout).await;

    // Drop handle and wait for close_notify.
    drop(_wh);
    let _ = writer_task.await;

    stats
}

/// Unified return type for maybe_tls()
enum IoUpgraded {
    Plain(TcpStream),
    #[cfg(feature = "tls")]
    Tls(tokio_rustls::server::TlsStream<TcpStream>),
}

// Feature-safe TLS accept wrapper: if TLS feature disabled or None provided, pass-through.
async fn maybe_tls(
    stream: TcpStream,
    tls: Option<Arc<TlsServerConfig>>,
) -> std::io::Result<IoUpgraded> {
    match tls {
        #[cfg(feature = "tls")]
        Some(cfg) => {
            use tokio_rustls::TlsAcceptor;
            let acceptor = TlsAcceptor::from(cfg);
            let tls_stream = acceptor.accept(stream).await?;
            Ok(IoUpgraded::Tls(tls_stream))
        }
        _ => Ok(IoUpgraded::Plain(stream)),
    }
}
