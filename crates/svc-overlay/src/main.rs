//! Binary entry: parse config, init tracing, run admin + overlay runtime.

use svc_overlay::admin::version::BuildInfo;
use svc_overlay::admin::ReadyProbe;
use svc_overlay::bootstrap;
use svc_overlay::config::Config;
use svc_overlay::supervisor::OverlayRuntime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) Config
    let cfg = Config::from_env_and_cli()?;
    cfg.validate()?;

    // 2) Tracing
    bootstrap::init_tracing("info");

    // 3) Admin server
    let probe = ReadyProbe::new();
    // Early boot truth; listener flips listeners_bound later.
    probe
        .set(|s| {
            s.metrics_bound = true; // Keep true: exporter can be added later
            s.cfg_loaded = true;
            s.listeners_bound = false;
            s.queues_ok = true;
            s.shed_rate_ok = true;
            s.fd_headroom = true;
        })
        .await;

    let build = BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        git: option_env!("GIT_SHA").unwrap_or("unknown"),
        build: option_env!("BUILD_TS").unwrap_or("unknown"),
        features: &[],
    };

    let admin = bootstrap::AdminServer::spawn(cfg.admin.http_addr, probe.clone(), build).await?;

    // 4) Overlay runtime (bind temporary TCP listener -> flips /readyz green)
    let overlay = OverlayRuntime::start(cfg.clone(), probe.clone()).await?;

    // 5) Wait until admin server exits (CTRL-C or test harness)
    admin.join().await?;

    // 6) Shutdown overlay
    overlay.shutdown().await?;

    Ok(())
}
