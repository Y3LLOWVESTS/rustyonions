//! RO:WHAT — Micronode binary entry: load config, init logs, wire readiness, run HTTP.
//! RO:WHY  — Single-binary Micronode with truthful /readyz and dev override.
//! RO:INVARIANTS — No locks across .await; flip readiness probes at the right moments.

#![forbid(unsafe_code)]

use micronode::{app::build_router, config::load::load_config, observability::logging};
use ron_kernel::wait_for_ctrl_c;
use std::net::SocketAddr;
use tracing::{error, info};

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    logging::init();

    // Load config
    let cfg = match load_config() {
        Ok(c) => c,
        Err(e) => {
            error!("config load failed: {e:#}");
            std::process::exit(2);
        }
    };

    // Build router and capture state
    let (router, st) = build_router(cfg.clone());

    // Probe: config successfully loaded
    st.probes.set_cfg_loaded(true);

    let bind: SocketAddr = cfg.server.bind;
    info!("micronode starting on http://{bind}");

    // Bind listener (readiness depends on this)
    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(l) => {
            st.probes.set_listeners_bound(true);
            l
        }
        Err(e) => {
            error!("bind failed on {bind}: {e:#}");
            std::process::exit(98);
        }
    };

    // We've registered /metrics; treat exporter as "bound" (process-exposed).
    st.probes.set_metrics_bound(true);

    // Run server with graceful shutdown
    let server = axum::serve(listener, router).with_graceful_shutdown(async {
        wait_for_ctrl_c().await;
        info!("shutdown signal received");
    });

    if let Err(e) = server.await {
        error!("server error: {e:#}");
    }
}
