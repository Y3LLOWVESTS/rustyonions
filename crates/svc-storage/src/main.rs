use std::{net::SocketAddr, sync::Arc};

use svc_storage::http::{extractors::AppState, server::serve_http};
use svc_storage::storage::{MemoryStorage, Storage};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = std::env::var("ADDR")
        .unwrap_or_else(|_| "127.0.0.1:5303".to_string())
        .parse()?;

    // Default runtime storage is bounded, evictable, and process-local.
    // It does not claim filesystem or restart durability.
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    let state = AppState { store };

    // Handle the Result so clippy’s unused_must_use stays green.
    if let Err(e) = serve_http(addr, state).await {
        eprintln!("server error: {e:#}");
        std::process::exit(1);
    }
    Ok(())
}
