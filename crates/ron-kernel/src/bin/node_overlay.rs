#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc};

use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use ron_kernel::cancel::Shutdown;
use ron_kernel::{
    config,
    overlay::{self, init_overlay_metrics, OverlayCfg},
    supervisor::Supervisor,
    wait_for_ctrl_c, Bus, Config, HealthState, Metrics,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).pretty().init();
    info!("Starting node_overlay…");

    // Shared infra
    let metrics = Arc::new(Metrics::new());
    let health = Arc::new(HealthState::new());
    let bus = Bus::new(1024);
    let sdn = Shutdown::new();

    // Load config + build overlay knobs + metrics
    let cfg = config::load_from_file("config.toml").unwrap_or_else(|_| Config::default());
    let overlay_cfg: OverlayCfg = overlay::overlay_cfg_from(&cfg)?;
    let overlay_metrics = init_overlay_metrics();

    // Supervisor: overlay + admin http
    let mut sup = Supervisor::new(bus.clone(), metrics.clone(), health.clone(), sdn.clone());

    {
        let h = health.clone();
        let m = metrics.clone();
        let oc = overlay_cfg.clone();
        let om = overlay_metrics.clone();
        let bus = bus.clone();
        sup.add_service("overlay", move |sdn| {
            let h = h.clone();
            let m = m.clone();
            let cfg = oc.clone();
            let om = om.clone();
            let bus = bus.clone();
            async move { overlay::service::run(sdn, h, m, cfg, om, bus).await }
        });
    }

    let admin_addr: SocketAddr = cfg.admin_addr.parse()?;
    {
        let h = health.clone();
        let m = metrics.clone();
        sup.add_service("admin_http", move |sdn| {
            let h = h.clone();
            let m = m.clone();
            async move { overlay::admin_http::run(sdn, h, m, admin_addr).await }
        });
    }

    let _cfg_watch = config::spawn_config_watcher("config.toml", bus.clone(), health.clone());

    let handle = sup.spawn();
    info!("node_overlay up. Try:");
    info!("  nc -v {}", overlay_cfg.bind);
    info!("  # metrics to watch:");
    info!("  #   overlay_accepted_total, overlay_rejected_total, overlay_active_connections");
    info!("  #   overlay_handshake_failures_total, overlay_read_timeouts_total, overlay_idle_timeouts_total");
    info!("  #   overlay_config_version, overlay_max_conns");

    let _ = wait_for_ctrl_c().await;
    info!("Ctrl-C received; shutting down…");
    handle.shutdown();
    handle.join().await?;

    Ok(())
}
