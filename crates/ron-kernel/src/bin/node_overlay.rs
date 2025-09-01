#![forbid(unsafe_code)]

use std::{
    io,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use bytes::BytesMut;
use prometheus::{
    Encoder, IntCounter, IntGauge, TextEncoder,
    register_int_counter, register_int_gauge,
};
use ron_kernel::{
    wait_for_ctrl_c, Bus, Config, HealthState, Metrics,
};
use ron_kernel::cancel::Shutdown;
use ron_kernel::supervisor::Supervisor;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    time::timeout,
};
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

/// ===== TLS helpers (Rustls) =====
mod tls {
    use std::{fs::File, io::BufReader, sync::Arc};
    use anyhow::Context;
    use rustls_pemfile::{certs, pkcs8_private_keys};
    use tokio_rustls::rustls::{
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
        ServerConfig,
    };

    pub fn try_build_server_config(cert_path: &str, key_path: &str) -> anyhow::Result<Arc<ServerConfig>> {
        // Load cert chain
        let mut cert_reader = BufReader::new(File::open(cert_path).with_context(|| format!("open cert {cert_path}"))?);
        let certs: Vec<CertificateDer<'static>> =
            certs(&mut cert_reader).collect::<Result<Vec<_>, _>>().with_context(|| "parse certs")?;

        // Load private key (PKCS#8)
        let mut key_reader = BufReader::new(File::open(key_path).with_context(|| format!("open key {key_path}"))?);
        let mut keys: Vec<PrivatePkcs8KeyDer<'static>> =
            pkcs8_private_keys(&mut key_reader).collect::<Result<Vec<_>, _>>().with_context(|| "parse pkcs8 key")?;

        let pkcs8 = keys.pop().ok_or_else(|| anyhow::anyhow!("no pkcs8 private key found"))?;
        let key_der: PrivateKeyDer<'static> = PrivateKeyDer::Pkcs8(pkcs8);

        // Build server config
        let cfg = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key_der)
            .with_context(|| "with_single_cert")?;

        Ok(Arc::new(cfg))
    }
}

/* ============================== Admin HTTP ================================ */

async fn run_admin_http(
    sdn: Shutdown,
    health: Arc<HealthState>,
    _metrics: Arc<Metrics>,
    addr: SocketAddr,
) -> anyhow::Result<()> {
    #[derive(Clone)]
    struct AdminState {
        health: Arc<HealthState>,
    }

    async fn healthz(State(st): State<AdminState>) -> impl IntoResponse {
        if st.health.all_ready() {
            (StatusCode::OK, "ok").into_response()
        } else {
            (StatusCode::SERVICE_UNAVAILABLE, "not ready").into_response()
        }
    }
    async fn readyz(State(st): State<AdminState>) -> impl IntoResponse {
        if st.health.all_ready() {
            (StatusCode::OK, "ready").into_response()
        } else {
            (StatusCode::SERVICE_UNAVAILABLE, "not ready").into_response()
        }
    }
    async fn metrics_route() -> impl IntoResponse {
        let mf = prometheus::gather();
        let mut buf = Vec::new();
        let enc = TextEncoder::new();
        let _ = enc.encode(&mf, &mut buf);
        (StatusCode::OK, [("Content-Type", enc.format_type().to_string())], buf).into_response()
    }

    let state = AdminState { health };
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics_route))
        .with_state(state);

    let listener = TcpListener::bind(addr).await?;
    info!("admin HTTP listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { sdn.cancelled().await })
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

/* ============================= Overlay Service ============================ */

#[derive(Clone)]
struct OverlayCfg {
    bind: SocketAddr,
    max_conns: usize,
    handshake_timeout: Duration,
    idle_timeout: Duration,
    read_timeout: Duration,
    write_timeout: Duration,
    tls_acceptor: Option<TlsAcceptor>,
}

fn overlay_cfg_from(config: &Config) -> anyhow::Result<OverlayCfg> {
    let bind: SocketAddr = config.overlay_addr.parse()?;
    let t = &config.transport;
    let max_conns = t.max_conns.unwrap_or(2048) as usize;
    let idle_timeout = Duration::from_millis(t.idle_timeout_ms.unwrap_or(30_000));
    let read_timeout = Duration::from_millis(t.read_timeout_ms.unwrap_or(5_000));
    let write_timeout = Duration::from_millis(t.write_timeout_ms.unwrap_or(5_000));
    let handshake_timeout = Duration::from_millis(3_000);

    // Optional TLS via raw keys in config table
    let tls_acceptor = match (
        config.raw.get("tls_cert_file").and_then(|v| v.as_str()),
        config.raw.get("tls_key_file").and_then(|v| v.as_str()),
    ) {
        (Some(cert), Some(key)) => match tls::try_build_server_config(cert, key) {
            Ok(cfg) => {
                info!("overlay TLS enabled (cert: {cert})");
                Some(TlsAcceptor::from(cfg))
            }
            Err(e) => {
                warn!("overlay TLS disabled (failed to load cert/key): {e:#}");
                None
            }
        },
        _ => {
            warn!("overlay TLS disabled (no tls_cert_file/tls_key_file in config)");
            None
        }
    };

    Ok(OverlayCfg {
        bind,
        max_conns,
        handshake_timeout,
        idle_timeout,
        read_timeout,
        write_timeout,
        tls_acceptor,
    })
}

// ---- Overlay metrics (registered into the default Prometheus registry) ----

#[derive(Clone)]
struct OverlayMetrics {
    accepted_total: IntCounter,
    rejected_total: IntCounter,
    active_conns: IntGauge,
}
fn init_overlay_metrics() -> OverlayMetrics {
    let accepted_total = register_int_counter!("overlay_accepted_total", "Total accepted overlay connections")
        .expect("register overlay_accepted_total");
    let rejected_total = register_int_counter!("overlay_rejected_total", "Total rejected overlay connections (at capacity)")
        .expect("register overlay_rejected_total");
    let active_conns   = register_int_gauge!("overlay_active_connections", "Current active overlay connections")
        .expect("register overlay_active_connections");
    OverlayMetrics { accepted_total, rejected_total, active_conns }
}

// Flip health "capacity" key based on available permits.
// If capacity is exhausted, /readyz will return 503 via `all_ready()`.
fn set_capacity_health(health: &Arc<HealthState>, limiter: &Arc<Semaphore>) {
    let ok = limiter.available_permits() > 0;
    health.set("capacity", ok);
}

async fn run_overlay(
    sdn: Shutdown,
    health: Arc<HealthState>,
    metrics: Arc<Metrics>,
    cfg: OverlayCfg,
    om: OverlayMetrics,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(cfg.bind).await?;
    info!("overlay listening on {}", cfg.bind);

    health.set("overlay", true);

    let limiter = Arc::new(Semaphore::new(cfg.max_conns));
    // reflect initial capacity + active gauge (0)
    set_capacity_health(&health, &limiter);
    om.active_conns.set(0);

    loop {
        tokio::select! {
            _ = sdn.cancelled() => {
                info!("overlay: shutdown requested");
                break;
            }
            Ok((sock, peer)) = listener.accept() => {
                let permit = match limiter.clone().try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        warn!("overlay: connection rejected (at capacity)");
                        om.rejected_total.inc();
                        set_capacity_health(&health, &limiter);
                        continue;
                    }
                };

                // reflect capacity after accepting a connection
                set_capacity_health(&health, &limiter);
                om.accepted_total.inc();
                om.active_conns.inc();

                let sdn_child = sdn.child();
                let metrics = metrics.clone();
                let cfg = cfg.clone();
                let om_child = om.clone();

                // When the task ends, decrement active gauge (via a small guard)
                tokio::spawn(async move {
                    let _active_guard = ActiveConnGuard { gauge: om_child.active_conns.clone() };
                    if let Err(e) = handle_conn(sdn_child, metrics, sock, peer, cfg, permit).await {
                        warn!("overlay: connection error from {peer}: {e:#}");
                    }
                    // _active_guard drops here => dec()
                });
            }
        }
    }

    Ok(())
}

struct ActiveConnGuard {
    gauge: IntGauge,
}
impl Drop for ActiveConnGuard {
    fn drop(&mut self) {
        self.gauge.dec();
    }
}

/// Either a plain TCP stream or a TLS-wrapped stream (no async trait needed).
enum IoEither {
    Plain(TcpStream),
    Tls(tokio_rustls::server::TlsStream<TcpStream>),
}

impl IoEither {
    async fn read_buf(&mut self, buf: &mut BytesMut) -> io::Result<usize> {
        match self {
            IoEither::Plain(s) => s.read_buf(buf).await,
            IoEither::Tls(s) => s.read_buf(buf).await,
        }
    }
    async fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        match self {
            IoEither::Plain(s) => s.write_all(data).await,
            IoEither::Tls(s) => s.write_all(data).await,
        }
    }
}

async fn handle_conn(
    sdn: Shutdown,
    metrics: Arc<Metrics>,
    sock: TcpStream,
    peer: SocketAddr,
    cfg: OverlayCfg,
    _permit: tokio::sync::OwnedSemaphorePermit,
) -> anyhow::Result<()> {
    // Optional TLS handshake
    let mut stream = if let Some(acc) = cfg.tls_acceptor {
        let accepted = timeout(cfg.handshake_timeout, acc.accept(sock))
            .await
            .map_err(|_| anyhow::anyhow!("tls handshake timeout"))??;
        IoEither::Tls(accepted)
    } else {
        IoEither::Plain(sock)
    };

    let mut buf = BytesMut::with_capacity(16 * 1024);
    let mut last_activity = Instant::now();

    loop {
        // If we've been idle too long, drop the connection.
        if last_activity.elapsed() >= cfg.idle_timeout {
            warn!("overlay: idle timeout from {peer}");
            break;
        }

        // Read with a deadline that respects BOTH read_timeout and remaining idle budget.
        let remaining_idle = cfg.idle_timeout - last_activity.elapsed();
        let deadline = remaining_idle.min(cfg.read_timeout);

        let read_res = tokio::select! {
            _ = sdn.cancelled() => break,
            res = timeout(deadline, stream.read_buf(&mut buf)) => res,
        };

        match read_res {
            Ok(Ok(0)) => break, // EOF
            Ok(Ok(n)) => {
                if n > 0 {
                    last_activity = Instant::now();
                    let to_send = &buf.split_to(n);
                    timeout(cfg.write_timeout, stream.write_all(to_send)).await??;
                }
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                // Timed out: check if it was idle budget or per-read deadline
                if last_activity.elapsed() >= cfg.idle_timeout {
                    warn!("overlay: idle timeout from {peer}");
                    break;
                } else {
                    warn!("overlay: read timeout from {peer}");
                    continue;
                }
            }
        }
    }

    // record a tiny “request” latency to keep metrics plumbing visible
    metrics.request_latency_seconds.observe(0.001);
    Ok(())
}

/* ================================  main  =================================== */

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).pretty().init();

    info!("Starting node_overlay…");

    // Shared infra
    let metrics = Arc::new(Metrics::new());
    let health  = Arc::new(HealthState::new());
    let bus     = Bus::new(1024);
    let sdn     = Shutdown::new();

    // Load config + derive overlay config
    let cfg = ron_kernel::config::load_from_file("config.toml").unwrap_or_else(|_| Config::default());
    let overlay_cfg = overlay_cfg_from(&cfg)?;
    let overlay_metrics = init_overlay_metrics();

    // Supervisor
    let mut sup = Supervisor::new(bus.clone(), metrics.clone(), health.clone(), sdn.clone());

    // Service #1: overlay TCP (TLS/plain)
    {
        let h  = health.clone();
        let m  = metrics.clone();
        let oc = overlay_cfg.clone();
        let om = overlay_metrics.clone();
        sup.add_service("overlay", move |sdn| {
            let h = h.clone();
            let m = m.clone();
            let cfg = oc.clone();
            let om = om.clone();
            async move { run_overlay(sdn, h, m, cfg, om).await }
        });
    }

    // Service #2: admin HTTP (health/ready/metrics) at cfg.admin_addr
    let admin_addr: SocketAddr = cfg.admin_addr.parse()?;
    {
        let h = health.clone();
        let m = metrics.clone();
        sup.add_service("admin_http", move |sdn| {
            let h = h.clone();
            let m = m.clone();
            async move { run_admin_http(sdn, h, m, admin_addr).await }
        });
    }

    // Config watcher (publishes KernelEvent::ConfigUpdated on change)
    let _cfg_watch = ron_kernel::config::spawn_config_watcher("config.toml", bus.clone(), health.clone());

    let handle = sup.spawn();

    info!("node_overlay up. Try:");
    info!("  # plaintext (if TLS not configured)");
    info!("  nc -v {}", overlay_cfg.bind);
    info!("  # with TLS configured in config.toml (tls_cert_file / tls_key_file)");
    info!("  # metrics to watch:");
    info!("  #   overlay_accepted_total, overlay_rejected_total, overlay_active_connections");

    // Wait for Ctrl-C
    let _ = wait_for_ctrl_c().await;
    info!("Ctrl-C received; shutting down…");
    handle.shutdown();
    handle.join().await?;

    Ok(())
}
