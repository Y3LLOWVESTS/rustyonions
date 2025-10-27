//! RO:WHAT — Overlay listener using transport facade + metrics.
//! RO:NEXT — When `transport` facade switches to ron-transport, no changes needed here.
//! RO:INVARIANTS — one writer per connection; bounded queue; no locks across .await

use crate::admin::metrics::overlay_metrics;
use crate::admin::ReadyProbe;
use crate::config::Config;
use crate::conn::tx::spawn_writer;
use crate::gossip::publish;
use crate::protocol::flags::Caps;
use crate::protocol::handshake::handshake;
use crate::protocol::oap::{try_parse_frame, Frame, FrameKind};
use crate::transport::{bind_listener, TransportStream};
use crate::tuning; // <— NEW

use anyhow::Result;
use bytes::BytesMut;
use std::net::SocketAddr;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tracing::{error, info, trace, warn, Instrument};

pub struct ListenerHandle {
    addr: SocketAddr,
    task: JoinHandle<()>,
}

impl ListenerHandle {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
    pub async fn shutdown(self) -> Result<()> {
        self.task.abort();
        Ok(())
    }
}

/// Observe "accept → handshake start" latency even if early-return happens.
struct AcceptTimer {
    start: Instant,
    observed: bool,
}
impl AcceptTimer {
    fn start() -> Self {
        Self {
            start: Instant::now(),
            observed: false,
        }
    }
    fn observe_once(&mut self) {
        if !self.observed {
            overlay_metrics::accept_latency_seconds(self.start.elapsed().as_secs_f64());
            self.observed = true;
        }
    }
}
impl Drop for AcceptTimer {
    fn drop(&mut self) {
        self.observe_once();
    }
}

pub async fn spawn_listener(cfg: &Config, probe: &ReadyProbe) -> Result<ListenerHandle> {
    let (listener, addr) = bind_listener(cfg.transport.addr).await?;
    info!(%addr, "overlay listener bound");
    probe.set(|s| s.listeners_bound = true).await;

    // Readiness sampler — flips `queues_ok` based on TX queue depth.
    let probe_clone = probe.clone();
    tokio::spawn(async move {
        loop {
            let depth = overlay_metrics::get_peer_tx_depth();
            let active = overlay_metrics::get_sessions_active();
            let watermark = tuning::tx_queue_watermark(); // <— NEW
            let ok = if active > 0 { depth < watermark } else { true };
            probe_clone.set(|s| s.queues_ok = ok).await;
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });

    let task = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    metrics::counter!("overlay_connections_total").increment(1);
                    overlay_metrics::inc_sessions_active();
                    tokio::spawn(handle_conn(peer, stream).in_current_span());
                }
                Err(e) => {
                    warn!(error=?e, "accept failed");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
    });

    Ok(ListenerHandle { addr, task })
}

async fn handle_conn(peer: SocketAddr, mut stream: TransportStream) {
    let mut accept_timer = AcceptTimer::start();

    // Handshake on the unified stream first (before splitting).
    {
        use tokio::io::{AsyncRead, AsyncWrite};

        trait HandshakeBorrow {
            type Dyn: AsyncRead + AsyncWrite + Unpin;
            fn as_io_mut(&mut self) -> &mut Self::Dyn;
        }
        impl HandshakeBorrow for TransportStream {
            type Dyn = tokio::net::TcpStream;
            fn as_io_mut(&mut self) -> &mut Self::Dyn {
                &mut self.inner
            }
        }

        let caps = Caps::GOSSIP_V1;
        let tmo = tuning::handshake_timeout(); // <— NEW
        let _neg = match tokio::time::timeout(
            tmo,
            handshake(
                <TransportStream as HandshakeBorrow>::as_io_mut(&mut stream),
                caps,
                tmo,
            ),
        )
        .await
        {
            Ok(Ok(n)) => {
                accept_timer.observe_once();
                info!(%peer, ver = n.version, caps = ?n.caps, "conn: negotiated");
                n
            }
            Ok(Err(_e)) => {
                overlay_metrics::handshake_fail("io");
                warn!(%peer, "conn: handshake failed");
                overlay_metrics::dec_sessions_active();
                return;
            }
            Err(_elapsed) => {
                overlay_metrics::handshake_fail("timeout");
                warn!(%peer, "conn: handshake timeout");
                overlay_metrics::dec_sessions_active();
                return;
            }
        };
    }

    // Split into owned halves; writer task owns the write half.
    let (mut rd, wr) = stream.into_split();
    let (tx, _writer_task) = spawn_writer(wr, 128);

    // Reader loop: parse frames; echo Data via bounded TX; publish demo gossip.
    let mut inbuf = BytesMut::with_capacity(8 * 1024);
    let start_ok = Instant::now();

    async fn read_more(rd: &mut OwnedReadHalf, buf: &mut BytesMut) -> std::io::Result<usize> {
        rd.read_buf(buf).await
    }

    loop {
        // Drain any complete frames already in the buffer.
        while let Some(frame) = match try_parse_frame(&mut inbuf) {
            Ok(f) => f,
            Err(e) => {
                warn!(%peer, error=?e, "conn: frame parse error");
                overlay_metrics::dec_sessions_active();
                return;
            }
        } {
            match frame.kind {
                FrameKind::Data => {
                    publish(frame.payload.clone());
                    let echo = Frame {
                        kind: FrameKind::Data,
                        payload: frame.payload,
                    };
                    // On backpressure, drop and record.
                    if tx.try_send(echo).is_err() {
                        overlay_metrics::inc_peer_tx_dropped();
                    }
                }
                FrameKind::Ctrl => {
                    // TODO: handle control frames when defined
                }
            }
        }

        // Refill buffer from reader
        match read_more(&mut rd, &mut inbuf).await {
            Ok(0) => {
                let secs = start_ok.elapsed().as_secs_f64();
                overlay_metrics::conn_lifetime_seconds(secs);
                info!(%peer, dt_ms = (secs * 1000.0) as u64, "conn: closed");
                overlay_metrics::dec_sessions_active();
                return;
            }
            Ok(n) => {
                trace!(%peer, read = n, buf_len = inbuf.len(), "read bytes");
            }
            Err(e) => {
                error!(%peer, error=?e, "conn: read error");
                overlay_metrics::dec_sessions_active();
                return;
            }
        }
    }
}
