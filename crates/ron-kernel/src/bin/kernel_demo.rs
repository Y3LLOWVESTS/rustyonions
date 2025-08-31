#![forbid(unsafe_code)]

use std::{error::Error, net::SocketAddr};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

use ron_kernel::{wait_for_ctrl_c, Metrics};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Logging: RUST_LOG=info (overridable via env)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .with_target(false)
        .compact()
        .init();

    // Start admin (metrics/health/ready) server
    let admin_addr: SocketAddr = "127.0.0.1:9090".parse()?;
    let metrics = Metrics::new();

    // IMPORTANT: Metrics::serve(self, ...) consumes a value, so call it on a CLONE.
    let (admin_task, bound) = metrics.clone().serve(admin_addr).await?;
    info!("Admin endpoints: /metrics /healthz /readyz at http://{bound}/");

    // Mark this process healthy so /readyz returns 200
    metrics.health().set("kernel_demo", true);
    info!("kernel_demo marked healthy; press Ctrl-C to shut down…");

    // Wait for Ctrl-C (ignore the Result to avoid unused_must_use warnings)
    let _ = wait_for_ctrl_c().await;

    info!("Shutting down…");
    // Optionally flip health on shutdown
    metrics.health().set("kernel_demo", false);

    // End the admin task (if needed). Dropping it will abort on runtime shutdown anyway.
    admin_task.abort();

    Ok(())
}
