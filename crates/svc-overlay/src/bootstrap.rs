//! Boot wiring for tracing + admin server.

use crate::admin::version::BuildInfo;
use crate::admin::{self, ReadyProbe};
use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

pub struct AdminServer {
    pub addr: SocketAddr,
    handle: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl AdminServer {
    pub async fn spawn(
        bind: SocketAddr,
        probe: ReadyProbe,
        ver: BuildInfo,
    ) -> anyhow::Result<Self> {
        let app: Router = admin::router(probe, ver);
        let listener = TcpListener::bind(bind).await?;
        let addr = listener.local_addr()?;
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await?;
            Ok::<_, anyhow::Error>(())
        });
        info!("admin server listening on {}", addr);
        Ok(Self { addr, handle })
    }

    pub async fn join(self) -> anyhow::Result<()> {
        self.handle.await??;
        Ok(())
    }
}

/// Install JSON tracing with an env-filter override.
/// Example: `RUST_LOG=svc-overlay=debug,axum=warn`
pub fn init_tracing(default_level: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    fmt()
        .with_max_level(Level::INFO)
        .with_env_filter(filter)
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .flatten_event(true)
        .init();
}

/// Start the minimal gossip engine and register the global publish hook.
/// Returns the spawned worker task so supervisors can hold/join it.
pub fn start_gossip_engine(capacity: usize) -> tokio::task::JoinHandle<()> {
    let (gossip, task) = crate::gossip::GossipEngine::start(capacity);
    gossip.install_global();
    info!("gossip engine online (cap={})", capacity);
    task
}
