//! RO:WHAT — Omnigate binary entrypoint: loads config, boots admin plane, serves HTTP.
//! RO:WHY  — Small main to keep logic in lib; Concerns: GOV/RES (truthful health/ready, graceful shutdown).
//! RO:INTERACTS — omnigate::config, omnigate::bootstrap::server; ron-kernel surfaces.
//! RO:INVARIANTS — no locks across .await; graceful shutdown.

use omnigate::{bootstrap, config::Config};
use ron_kernel::wait_for_ctrl_c;
use tracing::{error, info};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    omnigate::observability::init_tracing();

    // Minimal arg scan so `--config path` works (used by smoke script).
    let mut args = std::env::args().skip(1);
    let mut cfg_path: Option<String> = None;
    while let Some(arg) = args.next() {
        if arg == "--config" {
            if let Some(p) = args.next() {
                cfg_path = Some(p);
            }
            break;
        }
    }

    let cfg = match cfg_path {
        Some(p) => Config::from_toml_file(p),
        None => Config::load(),
    }
    .map_err(|e| {
        error!(error=?e, "failed to load config");
        e
    })?;

    info!(amnesia = %cfg.server.amnesia, cfg=?cfg, "omnigate config");

    // Build app (starts admin plane) and serve the API listener.
    let app = omnigate::App::build(cfg.clone()).await?;

    let server_cfg = cfg.server;
    let admin_addr = server_cfg.metrics_addr;

    let (server_task, bind) = bootstrap::server::serve(server_cfg, app.router).await?;
    info!(%bind, %admin_addr, "omnigate up");

    let workers: Vec<omnigate::runtime::DynWorker> =
        vec![omnigate::runtime::sample::TickWorker::new() as _];
    let supervisor = omnigate::runtime::spawn_supervisor(workers, 128);

    wait_for_ctrl_c().await;

    let _ = supervisor
        .tx_cmd
        .send(omnigate::runtime::SupervisorMsg::Stop);
    supervisor.shutdown.cancel();
    let _ = supervisor.join.await;

    server_task.abort();
    Ok(())
}
