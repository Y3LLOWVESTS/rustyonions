#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use ron_app_sdk::{
    Error as OapError, OapCodec, OapFlags, OapFrame, OAP_VERSION, DEFAULT_MAX_DECOMPRESSED,
};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, Semaphore},
};
use tokio_rustls::TlsAcceptor;
use tokio_util::codec::Framed;
use tracing::{debug, error, info};

use crate::config::Config;
use crate::handlers::{handle_hello, handle_mailbox, handle_storage_get};
use crate::mailbox::{Mailbox, MAILBOX_APP_PROTO_ID};
use crate::metrics::Metrics;
use crate::storage::{FsStorage, TILE_APP_PROTO_ID};

// NEW: OAP limits and per-topic metrics wiring
use crate::oap_limits::{OapLimits, StreamState, RejectReason};
use crate::oap_metrics;
use crate::oap_metrics::{add_data_bytes, inc_streams, inc_reject_timeout, inc_reject_too_many_bytes, inc_reject_too_many_frames};

/// Simple token-bucket rate limiter keyed by (tenant_id, app_proto_id).
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_per_sec: f64,
    last: Instant,
}
impl TokenBucket {
    fn new(rps: f64) -> Self {
        let cap = (rps * 2.0).max(1.0); // small burst allowance
        Self {
            tokens: cap,
            capacity: cap,
            refill_per_sec: rps,
            last: Instant::now(),
        }
    }
    fn allow(&mut self) -> bool {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + dt * self.refill_per_sec).min(self.capacity);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
    fn set_rate(&mut self, rps: f64) {
        self.refill_per_sec = rps;
        self.capacity = (rps * 2.0).max(1.0);
        self.tokens = self.tokens.min(self.capacity);
    }
}

#[derive(Default)]
struct RateLimiter {
    // key: (tenant_id, app_proto_id)
    buckets: HashMap<(u128, u16), TokenBucket>,
}
impl RateLimiter {
    fn check(&mut self, tenant_id: u128, app_proto_id: u16, rps_for_proto: f64) -> bool {
        let key = (tenant_id, app_proto_id);
        match self.buckets.get_mut(&key) {
            Some(b) => {
                if (b.refill_per_sec - rps_for_proto).abs() > f64::EPSILON {
                    b.set_rate(rps_for_proto);
                }
                b.allow()
            }
            None => {
                let mut b = TokenBucket::new(rps_for_proto);
                let ok = b.allow();
                self.buckets.insert(key, b);
                ok
            }
        }
    }
}

/// Bundled connection dependencies to keep function arity low (Clippy).
#[derive(Clone)]
struct Deps {
    acceptor: TlsAcceptor,
    storage: Arc<FsStorage>,
    mailbox: Arc<Mailbox>,
    metrics: Arc<Metrics>,
    inflight: Arc<Semaphore>,
    rate: Arc<Mutex<RateLimiter>>,
    tile_rps: f64,
    mb_rps: f64,
    cfg: Config,
}

pub async fn run(
    cfg: Config,
    acceptor: TlsAcceptor,
    storage: Arc<FsStorage>,
    mailbox: Arc<Mailbox>,
    metrics: Arc<Metrics>,
) -> Result<()> {
    let listen_addr = cfg.addr;
    let listener = TcpListener::bind(listen_addr).await.context("bind oap addr")?;
    info!("svc-omnigate OAP listener on {}", listen_addr);

    // NEW: initialize OAP metrics (idempotent)
    oap_metrics::init_oap_metrics();

    // Global inflight gate
    let inflight = Arc::new(Semaphore::new(cfg.max_inflight as usize));
    // Per-tenant quotas
    let rate = Arc::new(Mutex::new(RateLimiter::default()));
    let deps = Deps {
        acceptor,
        storage,
        mailbox,
        metrics,
        inflight,
        rate,
        tile_rps: cfg.quota_tile_rps as f64,
        mb_rps: cfg.quota_mailbox_rps as f64,
        cfg,
    };

    loop {
        let (tcp, peer) = listener.accept().await?;
        let deps_cloned = deps.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(tcp, peer, deps_cloned).await {
                error!("conn error: {e:?}");
            }
        });
    }
}

async fn handle_conn(
    tcp: TcpStream,
    peer: std::net::SocketAddr,
    deps: Deps,
) -> Result<()> {
    let tls = deps.acceptor.accept(tcp).await.context("tls accept")?;
    let mut framed = Framed::new(tls, OapCodec::new(deps.cfg.max_frame, DEFAULT_MAX_DECOMPRESSED));
    debug!("conn established from {}", peer);

    // NEW: per-connection stream budget & timing
    let limits = OapLimits::default();
    let mut st = StreamState::new(Instant::now());

    loop {
        match framed.next().await {
            Some(Ok(frame)) => {
                // Enforce caps/timeouts on incoming payload (per "stream" == per request here)
                let now = Instant::now();
                if let Err(reason) = st.on_frame(frame.payload.len(), now, &limits) {
                    match reason {
                        RejectReason::Timeout => {
                            inc_reject_timeout();
                            // 408 Request Timeout
                            let _ = send_error_frame(
                                &mut framed,
                                408,
                                frame.app_proto_id,
                                frame.tenant_id,
                                frame.corr_id,
                                br#"{"error":"timeout"}"#,
                            )
                            .await;
                        }
                        RejectReason::TooManyFrames { .. } => {
                            inc_reject_too_many_frames();
                            // 400 Bad Request
                            let _ = send_error_frame(
                                &mut framed,
                                400,
                                frame.app_proto_id,
                                frame.tenant_id,
                                frame.corr_id,
                                br#"{"error":"too_many_frames"}"#,
                            )
                            .await;
                        }
                        RejectReason::TooManyBytes { .. } => {
                            inc_reject_too_many_bytes();
                            // 413 Payload Too Large
                            let _ = send_error_frame(
                                &mut framed,
                                413,
                                frame.app_proto_id,
                                frame.tenant_id,
                                frame.corr_id,
                                br#"{"error":"too_large"}"#,
                            )
                            .await;
                        }
                    }
                    break; // close connection after reject
                }

                // Map app_proto_id to a coarse "topic" label for metrics
                let topic = if frame.app_proto_id == TILE_APP_PROTO_ID {
                    "tiles"
                } else if frame.app_proto_id == MAILBOX_APP_PROTO_ID {
                    "mailbox"
                } else if frame.app_proto_id == 0 {
                    "hello"
                } else {
                    "unknown"
                };
                inc_streams(topic);
                add_data_bytes(topic, frame.payload.len() as u64);

                // Global capacity gate (per frame/request)
                let permit = match deps.inflight.clone().try_acquire_owned() {
                    Ok(p) => {
                        deps.metrics.inflight_inc();
                        Some(p)
                    }
                    Err(_) => {
                        deps.metrics.inc_overload();
                        let _ = send_error_frame(
                            &mut framed,
                            503,
                            frame.app_proto_id,
                            frame.tenant_id,
                            frame.corr_id,
                            br#"{"error":"overload","retry_after_ms":1000}"#,
                        )
                        .await;
                        continue;
                    }
                };

                // Per-tenant per-proto quotas
                let allow = {
                    let mut rl = deps.rate.lock().await;
                    let rps = if frame.app_proto_id == TILE_APP_PROTO_ID {
                        deps.tile_rps
                    } else if frame.app_proto_id == MAILBOX_APP_PROTO_ID {
                        deps.mb_rps
                    } else {
                        f64::INFINITY // no quota on HELLO/unknown
                    };
                    rl.check(frame.tenant_id, frame.app_proto_id, rps)
                };
                if !allow {
                    deps.metrics.inc_overload(); // counting 429s with overload for now
                    let _ = send_error_frame(
                        &mut framed,
                        429,
                        frame.app_proto_id,
                        frame.tenant_id,
                        frame.corr_id,
                        br#"{"error":"over_quota","retry_after_ms":1000}"#,
                    )
                    .await;
                    drop(permit);
                    deps.metrics.inflight_dec();
                    continue;
                }

                // Count request
                deps.metrics.inc_requests();

                // Dispatch
                let res = match frame.app_proto_id {
                    0 => handle_hello(&mut framed, &deps.cfg, &frame).await,
                    p if p == TILE_APP_PROTO_ID => {
                        // storage handler updates bytes_out_total internally
                        handle_storage_get(
                            &mut framed,
                            &deps.cfg,
                            &deps.storage, // auto-deref; fixes clippy::explicit-auto-deref
                            &frame,
                            deps.metrics.clone(),
                        )
                        .await
                    }
                    p if p == MAILBOX_APP_PROTO_ID => {
                        // mailbox handler may not update byte counters; OK for now
                        handle_mailbox(&mut framed, &deps.mailbox, &frame, &deps.metrics).await
                    }
                    _ => {
                        // Unknown protocol id
                        let _ = send_error_frame(
                            &mut framed,
                            400,
                            frame.app_proto_id,
                            frame.tenant_id,
                            frame.corr_id,
                            br#"{"error":"bad_request"}"#,
                        )
                        .await;
                        Ok(())
                    }
                };

                drop(permit);
                deps.metrics.inflight_dec();

                if let Err(e) = res {
                    let (code, body) = map_err(&e);
                    let _ = send_error_frame(
                        &mut framed,
                        code,
                        frame.app_proto_id,
                        frame.tenant_id,
                        frame.corr_id,
                        &body,
                    )
                    .await;
                } else {
                    // Success -> refresh activity clock
                    st.touch(Instant::now());
                }
            }
            Some(Err(e)) => {
                // Treat TLS EOF without close_notify as a NORMAL close (avoid error spam).
                if is_tls_unexpected_eof(&e) {
                    debug!("conn {} closed by peer without close_notify", peer);
                    break;
                } else {
                    // Promote to anyhow with context
                    return Err(e).context("oap read");
                }
            }
            None => {
                // Graceful end of stream.
                debug!("conn {} ended (EOF)", peer);
                break;
            }
        }
    }

    Ok(())
}

/// Helper to send a RESP+END JSON error frame.
async fn send_error_frame(
    framed: &mut Framed<tokio_rustls::server::TlsStream<TcpStream>, OapCodec>,
    code: u16,
    app_proto_id: u16,
    tenant_id: u128,
    corr_id: u64,
    json_body: &[u8],
) -> Result<()> {
    let resp = OapFrame {
        ver: OAP_VERSION,
        flags: OapFlags::RESP | OapFlags::END,
        code,
        app_proto_id,
        tenant_id,
        cap: Bytes::new(),
        corr_id,
        payload: Bytes::from(json_body.to_vec()),
    };
    framed.send(resp).await.context("send error frame")
}

/// Detect the common rustls/IO wording for EOF-without-close_notify as surfaced
/// through the `ron_app_sdk::Error` decoder error.
fn is_tls_unexpected_eof(err: &OapError) -> bool {
    let s = err.to_string().to_ascii_lowercase();
    s.contains("close_notify") || s.contains("unexpected eof")
}

fn map_err(e: &anyhow::Error) -> (u16, Vec<u8>) {
    let s = format!("{e:?}");
    if s.contains("too_large") || s.contains("413") {
        (413, br#"{"error":"too_large"}"#.to_vec())
    } else if s.contains("not_found") || s.contains("404") {
        (404, br#"{"error":"not_found"}"#.to_vec())
    } else if s.contains("over_quota") || s.contains("429") {
        (429, br#"{"error":"over_quota"}"#.to_vec())
    } else if s.contains("overload") || s.contains("503") {
        (503, br#"{"error":"overload"}"#.to_vec())
    } else if s.contains("invalid json") || s.contains("bad_request") || s.contains("bad op") {
        (400, br#"{"error":"bad_request"}"#.to_vec())
    } else {
        (500, br#"{"error":"internal"}"#.to_vec())
    }
}
