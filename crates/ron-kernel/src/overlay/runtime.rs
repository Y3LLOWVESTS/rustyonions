#![forbid(unsafe_code)]

use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use tokio_rustls::TlsAcceptor;
use tracing::{error, info, warn};

use crate::Config;
use super::tls;

#[derive(Clone)]
pub struct OverlayCfg {
    pub bind: std::net::SocketAddr,
    pub max_conns: usize,
    pub handshake_timeout: Duration,
    pub idle_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub tls_acceptor: Option<TlsAcceptor>,
}

pub fn overlay_cfg_from(config: &Config) -> anyhow::Result<OverlayCfg> {
    let bind: std::net::SocketAddr = config.overlay_addr.parse()?;
    let t = &config.transport;
    let max_conns = t.max_conns.unwrap_or(2048) as usize;
    let idle_timeout = Duration::from_millis(t.idle_timeout_ms.unwrap_or(30_000));
    let read_timeout = Duration::from_millis(t.read_timeout_ms.unwrap_or(5_000));
    let write_timeout = Duration::from_millis(t.write_timeout_ms.unwrap_or(5_000));
    let handshake_timeout = Duration::from_millis(3_000);

    let tls_acceptor = match (
        config.raw.get("tls_cert_file").and_then(|v| v.as_str()),
        config.raw.get("tls_key_file").and_then(|v| v.as_str()),
    ) {
        (Some(cert), Some(key)) => match tls::try_build_server_config(cert, key) {
            Ok(cfg) => { info!("overlay TLS enabled (cert: {cert})"); Some(TlsAcceptor::from(cfg)) }
            Err(e)  => { warn!("overlay TLS disabled (failed to load cert/key): {e:#}"); None }
        },
        _ => { warn!("overlay TLS disabled (no tls_cert_file/tls_key_file in config)"); None }
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

#[derive(Clone)]
pub struct OverlayRuntime {
    pub max_conns: Arc<AtomicUsize>,
    pub idle_ms: Arc<AtomicU64>,
    pub read_ms: Arc<AtomicU64>,
    pub write_ms: Arc<AtomicU64>,
    pub tls_acceptor: Arc<RwLock<Option<TlsAcceptor>>>,
}

impl OverlayRuntime {
    pub fn from_cfg(cfg: &OverlayCfg) -> Self {
        Self {
            max_conns: Arc::new(AtomicUsize::new(cfg.max_conns)),
            idle_ms: Arc::new(AtomicU64::new(cfg.idle_timeout.as_millis() as u64)),
            read_ms: Arc::new(AtomicU64::new(cfg.read_timeout.as_millis() as u64)),
            write_ms: Arc::new(AtomicU64::new(cfg.write_timeout.as_millis() as u64)),
            tls_acceptor: Arc::new(RwLock::new(cfg.tls_acceptor.clone())),
        }
    }

    pub fn idle_timeout(&self) -> Duration  { Duration::from_millis(self.idle_ms.load(Ordering::Relaxed)) }
    pub fn read_timeout(&self) -> Duration  { Duration::from_millis(self.read_ms.load(Ordering::Relaxed)) }
    pub fn write_timeout(&self) -> Duration { Duration::from_millis(self.write_ms.load(Ordering::Relaxed)) }
    pub fn max(&self) -> usize { self.max_conns.load(Ordering::Relaxed) }

    pub fn apply(&self, newc: &OverlayCfg) {
        self.max_conns.store(newc.max_conns, Ordering::Relaxed);
        self.idle_ms.store(newc.idle_timeout.as_millis() as u64, Ordering::Relaxed);
        self.read_ms.store(newc.read_timeout.as_millis() as u64, Ordering::Relaxed);
        self.write_ms.store(newc.write_timeout.as_millis() as u64, Ordering::Relaxed);
        *write_lock_ignore_poison(&self.tls_acceptor) = newc.tls_acceptor.clone();
    }
}

/// Recover from poisoned RwLock writes without panicking (log and continue).
#[inline]
fn write_lock_ignore_poison<'a, T>(rw: &'a RwLock<T>) -> std::sync::RwLockWriteGuard<'a, T> {
    match rw.write() {
        Ok(g) => g,
        Err(p) => {
            error!("overlay/runtime: write lock poisoned, recovering");
            p.into_inner()
        }
    }
}
