#![forbid(unsafe_code)]

//! Overlay TCP listener + connection loop. Supports hot-reload via bus events.

use std::{io, net::SocketAddr, sync::Arc, time::{Duration, Instant}};
use bytes::BytesMut;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, time::timeout};
use tracing::{info, warn};

use crate::{cancel::Shutdown, bus::Bus, metrics::HealthState, Metrics};
use crate::{config, KernelEvent};
use super::{metrics::OverlayMetrics, runtime::{OverlayCfg, OverlayRuntime}};

enum IoEither { Plain(TcpStream), Tls(Box<tokio_rustls::server::TlsStream<TcpStream>>) }
impl IoEither {
    async fn read_buf(&mut self, buf: &mut BytesMut) -> io::Result<usize> { match self { IoEither::Plain(s)=>s.read_buf(buf).await, IoEither::Tls(s)=>s.read_buf(buf).await } }
    async fn write_all(&mut self, data: &[u8]) -> io::Result<()> { match self { IoEither::Plain(s)=>s.write_all(data).await, IoEither::Tls(s)=>s.write_all(data).await } }
}

pub async fn run(
    sdn: Shutdown,
    health: Arc<HealthState>,
    metrics: Arc<Metrics>,
    cfg: OverlayCfg,
    om: OverlayMetrics,
    bus: Bus,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(cfg.bind).await?; info!("overlay listening on {}", cfg.bind);
    health.set("overlay", true);

    let rt = OverlayRuntime::from_cfg(&cfg);
    om.max_conns_gauge.set(rt.max() as i64);
    om.cfg_version.set(0);
    om.active_conns.set(0);
    health.set("capacity", 0 < rt.max());

    // Subscribe to config changes and hot-apply
    {
        let health = health.clone();
        let om = om.clone();
        let rt = rt.clone();
        tokio::spawn(async move {
            let mut rx = bus.subscribe();
            loop {
                match rx.recv().await {
                    Ok(KernelEvent::ConfigUpdated { version }) => {
                        match config::load_from_file("config.toml").ok().and_then(|c| super::runtime::overlay_cfg_from(&c).ok()) {
                            Some(newc) => {
                                // Log TLS change explicitly for visibility
                                if newc.tls_acceptor.is_some() {
                                    info!("overlay: TLS configuration updated/enabled");
                                } else {
                                    warn!("overlay: TLS configuration disabled or failed to load");
                                }

                                rt.apply(&newc);
                                om.max_conns_gauge.set(newc.max_conns as i64);
                                om.cfg_version.set(version as i64);

                                let active_now = om.active_conns.get() as usize;
                                health.set("capacity", active_now < newc.max_conns);
                                info!("overlay hot-reloaded config version {version} (max_conns={})", newc.max_conns);
                            }
                            None => { warn!("overlay received ConfigUpdated {version} but failed to apply new config"); }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });
    }

    loop {
        tokio::select! {
            _ = sdn.cancelled() => { info!("overlay: shutdown requested"); break; }
            Ok((sock, peer)) = listener.accept() => {
                // Capacity check (hot-reload aware).
                let current_active = om.active_conns.get() as usize;
                if current_active >= rt.max() {
                    warn!("overlay: connection rejected (at capacity)");
                    om.rejected_total.inc();
                    health.set("capacity", (om.active_conns.get() as usize) < rt.max());
                    continue;
                }

                om.accepted_total.inc();
                om.active_conns.inc();
                health.set("capacity", (om.active_conns.get() as usize) < rt.max());

                let sdn_child = sdn.child();
                let metrics = metrics.clone();
                let om_child = om.clone();
                let rt_child = rt.clone();

                tokio::spawn(async move {
                    let _dec = ActiveConnGuard { gauge: om_child.active_conns.clone(), health: None, max: rt_child.max() };
                    if let Err(e) = handle_conn(sdn_child, metrics, sock, peer, rt_child, om_child, cfg.handshake_timeout).await {
                        warn!("overlay: connection error from {peer}: {e:#}");
                    }
                });
            }
        }
    }
    Ok(())
}

struct ActiveConnGuard { gauge: prometheus::IntGauge, health: Option<Arc<HealthState>>, max: usize }
impl Drop for ActiveConnGuard {
    fn drop(&mut self) {
        self.gauge.dec();
        if let Some(h) = &self.health {
            let active_now = self.gauge.get() as usize;
            h.set("capacity", active_now < self.max);
        }
    }
}

async fn handle_conn(
    sdn: Shutdown,
    metrics: Arc<Metrics>,
    sock: TcpStream,
    peer: SocketAddr,
    rt: OverlayRuntime,
    om: OverlayMetrics,
    handshake_timeout: Duration,
) -> anyhow::Result<()> {
    // IMPORTANT: grab the TLS acceptor into a local Option<TlsAcceptor> so we
    // don't hold an RwLockReadGuard across an await (required for Send).
    let acc_opt = match rt.tls_acceptor.read() {
    Ok(g) => g.clone(),
    Err(_) => None, // poisoned; treat as no TLS
    };

    let mut stream = if let Some(acc) = acc_opt {
        match timeout(handshake_timeout, acc.accept(sock)).await {
            Ok(Ok(accepted)) => IoEither::Tls(Box::new(accepted)),
            Ok(Err(e)) => { om.handshake_failures_total.inc(); return Err(e.into()); }
            Err(_) => { om.handshake_failures_total.inc(); return Err(anyhow::anyhow!("tls handshake timeout")); }
        }
    } else {
        IoEither::Plain(sock)
    };

    let mut buf = BytesMut::with_capacity(16 * 1024);
    let mut last_activity = Instant::now();

    loop {
        if last_activity.elapsed() >= rt.idle_timeout() {
            warn!("overlay: idle timeout from {peer}");
            om.idle_timeouts_total.inc();
            break;
        }

        let remaining_idle = rt.idle_timeout().saturating_sub(last_activity.elapsed());
        let deadline = remaining_idle.min(rt.read_timeout());

        let read_res = tokio::select! {
            _ = sdn.cancelled() => break,
            res = timeout(deadline, stream.read_buf(&mut buf)) => res,
        };

        match read_res {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                if n > 0 {
                    last_activity = Instant::now();
                    let to_send = &buf.split_to(n);
                    timeout(rt.write_timeout(), stream.write_all(to_send)).await??;
                }
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                if last_activity.elapsed() >= rt.idle_timeout() {
                    warn!("overlay: idle timeout from {peer}");
                    om.idle_timeouts_total.inc();
                    break;
                } else {
                    warn!("overlay: read timeout from {peer}");
                    om.read_timeouts_total.inc();
                    continue;
                }
            }
        }
    }

    metrics.request_latency_seconds.observe(0.001);
    Ok(())
}
