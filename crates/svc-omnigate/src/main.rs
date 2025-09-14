#![forbid(unsafe_code)]

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

mod admin_http;
mod config;
mod handlers;
mod mailbox; // Mailbox state
mod metrics;
mod oap_limits; // NEW: expose OAP limits to the crate
mod oap_metrics;
mod server;
mod storage; // FsStorage helper
mod tls; // NEW: expose OAP metrics to the crate

use crate::config::Config;
use crate::mailbox::Mailbox;
use crate::metrics::Metrics;
use crate::storage::FsStorage;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load service config from environment.
    let cfg = Config::from_env();

    // IMPORTANT:
    // tls::load_tls() returns a tokio_rustls::TlsAcceptor already.
    // Do NOT wrap it again with TlsAcceptor::from(Arc<...>).
    let acceptor = tls::load_tls()?;

    // Shared state for handlers.
    let storage = Arc::new(FsStorage::new(&cfg.tiles_root, cfg.max_file_bytes));
    let mailbox = Arc::new(Mailbox::new(std::time::Duration::from_secs(30)));
    let metrics = Arc::new(Metrics::default());

    // Admin HTTP (health/ready/metrics)
    tokio::spawn(admin_http::run(
        cfg.http_addr,
        cfg.max_inflight,
        metrics.clone(),
    ));

    info!("svc-omnigate starting on {}", cfg.addr);

    // Run server and listen for shutdown concurrently.
    tokio::select! {
        r = server::run(cfg.clone(), acceptor.clone(), storage.clone(), mailbox.clone(), metrics.clone()) => {
            r?;
        },
        _ = wait_for_shutdown() => {
            info!("shutdown signal received");
        }
    }

    Ok(())
}

async fn wait_for_shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}
