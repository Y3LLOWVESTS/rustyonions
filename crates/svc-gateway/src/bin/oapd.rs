#![forbid(unsafe_code)]

use std::net::SocketAddr;

use clap::Parser;
use gateway::oap::OapServer;
use ron_kernel::{bus::Bus, Metrics};
use tokio::signal;

#[derive(Debug, Parser)]
#[command(
    name = "gateway-oapd",
    about = "Gateway OAP/1 server with BLAKE3 (b3:<hex>) verification"
)]
struct Args {
    /// Address to bind the OAP server on, e.g. 127.0.0.1:9444
    #[arg(long = "oap")]
    oap_addr: SocketAddr,

    /// ACK credit window in bytes (server grants more credit after ~half this is consumed)
    #[arg(long = "oap-ack-window", default_value_t = 64 * 1024)]
    oap_ack_window: usize,

    /// Maximum allowed frame size in bytes (default 1 MiB)
    #[arg(long = "oap-max-frame", default_value_t = 1 << 20)]
    oap_max_frame: usize,

    /// Maximum concurrent OAP connections before returning a BUSY error
    #[arg(long = "oap-concurrency", default_value_t = 1024)]
    oap_concurrency: usize,

    /// Optional metrics/health server address, e.g. 127.0.0.1:9909
    #[arg(long = "metrics")]
    metrics_addr: Option<SocketAddr>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Kernel bus + metrics (shared)
    let bus = Bus::new(256);
    let metrics = Metrics::new();
    if let Some(addr) = args.metrics_addr {
        let (_h, bound) = metrics.clone().serve(addr).await?;
        eprintln!("metrics at http://{bound}/metrics  (healthz/readyz also available)");
        metrics.health().set("gateway_oapd", true);
    }

    // Configure OAP server
    let mut srv = OapServer::new(bus.clone());
    srv.ack_window_bytes = args.oap_ack_window;
    srv.max_frame = args.oap_max_frame;
    srv.concurrency_limit = args.oap_concurrency;

    // Capture fields BEFORE we call serve(self, …) which moves `srv`
    let ack = srv.ack_window_bytes;
    let maxf = srv.max_frame;
    let conc = srv.concurrency_limit;

    // Start OAP server
    let (_handle, bound) = srv.serve(args.oap_addr).await?;
    eprintln!(
        "OAP/1 server listening on {bound}  (ack_window={}B, max_frame={}B, concurrency={})",
        ack, maxf, conc
    );

    // Stay alive until Ctrl-C
    signal::ctrl_c().await?;
    eprintln!("ctrl-c received, shutting down");
    Ok(())
}
