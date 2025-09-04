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
        Self { tokens: cap, capacity: cap, refill_per_sec: rps, last: Instant::now() }
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

pub async fn run(
    cfg: Config,
    acceptor: TlsAcceptor,
    storage: Arc<FsStorage>,
    mailbox: Arc<Mailbox>,
    metrics: Arc<Metrics>,
) -> Result<()> {
    let listener = TcpListener::bind(cfg.addr).await.context("bind oap addr")?;
    info!("svc-omnigate OAP listener on {}", cfg.addr);

    // Global inflight gate
    let inflight = Arc::new(Semaphore::new(cfg.max_inflight as usize));
    // Per-tenant quotas
    let rate = Arc::new(Mutex::new(RateLimiter::default()));
    let tile_rps = cfg.quota_tile_rps as f64;
    let mb_rps = cfg.quota_mailbox_rps as f64;

    loop {
        let (tcp, peer) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let storage = storage.clone();
        let mailbox = mailbox.clone();
        let metrics = metrics.clone();
        let inflight = inflight.clone();
        let rate = rate.clone();
        let cfg_clone = cfg.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_conn(
                tcp, peer, acceptor, storage, mailbox, metrics, inflight, rate, tile_rps, mb_rps,
                cfg_clone,
            )
            .await
            {
                error!("conn error: {e:?}");
            }
        });
    }
}

async fn handle_conn(
    tcp: TcpStream,
    peer: std::net::SocketAddr,
    acceptor: TlsAcceptor,
    storage: Arc<FsStorage>,
    mailbox: Arc<Mailbox>,
    metrics: Arc<Metrics>,
    inflight: Arc<Semaphore>,
    rate: Arc<Mutex<RateLimiter>>,
    tile_rps: f64,
    mb_rps: f64,
    cfg: Config,
) -> Result<()> {
    let tls = acceptor.accept(tcp).await.context("tls accept")?;
    let mut framed = Framed::new(tls, OapCodec::new(cfg.max_frame, DEFAULT_MAX_DECOMPRESSED));
    debug!("conn established from {}", peer);

    loop {
        match framed.next().await {
            Some(Ok(frame)) => {
                // Global capacity gate (per frame/request)
                let permit = match inflight.clone().try_acquire_owned() {
                    Ok(p) => {
                        metrics.inflight_inc();
                        Some(p)
                    }
                    Err(_) => {
                        metrics.inc_overload();
                        let body = br#"{"error":"overload","retry_after_ms":1000}"#.to_vec();
                        let resp = OapFrame {
                            ver: OAP_VERSION,
                            flags: OapFlags::RESP | OapFlags::END,
                            code: 503,
                            app_proto_id: frame.app_proto_id,
                            tenant_id: frame.tenant_id,
                            cap: Bytes::new(),
                            corr_id: frame.corr_id,
                            payload: Bytes::from(body),
                        };
                        let _ = framed.send(resp).await;
                        continue;
                    }
                };

                // Per-tenant per-proto quotas
                let allow = {
                    let mut rl = rate.lock().await;
                    let rps = if frame.app_proto_id == TILE_APP_PROTO_ID {
                        tile_rps
                    } else if frame.app_proto_id == MAILBOX_APP_PROTO_ID {
                        mb_rps
                    } else {
                        f64::INFINITY // no quota on HELLO/unknown
                    };
                    rl.check(frame.tenant_id, frame.app_proto_id, rps)
                };
                if !allow {
                    metrics.inc_overload(); // counting 429s with overload for now
                    let body = br#"{"error":"over_quota","retry_after_ms":1000}"#.to_vec();
                    let resp = OapFrame {
                        ver: OAP_VERSION,
                        flags: OapFlags::RESP | OapFlags::END,
                        code: 429,
                        app_proto_id: frame.app_proto_id,
                        tenant_id: frame.tenant_id,
                        cap: Bytes::new(),
                        corr_id: frame.corr_id,
                        payload: Bytes::from(body),
                    };
                    framed.send(resp).await?;
                    drop(permit);
                    metrics.inflight_dec();
                    continue;
                }

                // Count request
                metrics.inc_requests();

                // Dispatch
                let res = match frame.app_proto_id {
                    0 => handle_hello(&mut framed, &cfg, &frame).await,
                    p if p == TILE_APP_PROTO_ID => {
                        // storage handler updates bytes_out_total internally
                        handle_storage_get(&mut framed, &cfg, &*storage, &frame, metrics.clone())
                            .await
                    }
                    p if p == MAILBOX_APP_PROTO_ID => {
                        // mailbox handler may not update byte counters; OK for now
                        handle_mailbox(&mut framed, &*mailbox, &frame, &metrics).await
                    }
                    _ => {
                        // Unknown protocol id
                        let resp = OapFrame {
                            ver: OAP_VERSION,
                            flags: OapFlags::RESP | OapFlags::END,
                            code: 400,
                            app_proto_id: frame.app_proto_id,
                            tenant_id: frame.tenant_id,
                            cap: Bytes::new(),
                            corr_id: frame.corr_id,
                            payload: Bytes::from_static(br#"{"error":"bad_request"}"#),
                        };
                        framed.send(resp).await?;
                        Ok(())
                    }
                };

                drop(permit);
                metrics.inflight_dec();

                if let Err(e) = res {
                    let (code, body) = map_err(&e);
                    let resp = OapFrame {
                        ver: OAP_VERSION,
                        flags: OapFlags::RESP | OapFlags::END,
                        code,
                        app_proto_id: frame.app_proto_id,
                        tenant_id: frame.tenant_id,
                        cap: Bytes::new(),
                        corr_id: frame.corr_id,
                        payload: Bytes::from(body),
                    };
                    if let Err(se) = framed.send(resp).await {
                        error!("send error after handler failure: {se}");
                        break;
                    }
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
