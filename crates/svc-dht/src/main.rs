//! RO:WHAT — Binary entrypoint: init tracing/metrics, load config, spawn supervisor, serve admin HTTP
//! RO:WHY — Service bootstrap; Concerns SEC/RES/PERF/GOV with observable readiness

use axum::{
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use ron_kernel::{wait_for_ctrl_c, HealthState};
use svc_dht::provider::ttl::spawn_pruner;
use svc_dht::rpc::http;
use svc_dht::{
    bootstrap, config::Config, metrics::DhtMetrics, pipeline::lookup::LookupCtx,
    readiness::ReadyGate, ro_tracing, ProviderStore,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    ro_tracing::init();
    let cfg = Config::from_env()?;
    let health = Arc::new(HealthState::default());
    let ready = Arc::new(ReadyGate::new());
    let metrics = Arc::new(DhtMetrics::new()?);
    let providers = Arc::new(ProviderStore::new(Duration::from_secs(600)));
    let _pruner = spawn_pruner(providers.clone());

    // Pipeline context — set a sane global leg concurrency
    let lookup_ctx = Arc::new(LookupCtx::new(providers.clone(), /*max_legs*/ 64));

    // Admin HTTP
    let (admin_task, admin_addr) = serve_admin(
        cfg.admin_bind,
        health.clone(),
        ready.clone(),
        metrics.clone(),
        providers.clone(),
        // pipeline knobs from Config
        cfg.alpha,
        cfg.beta,
        cfg.hop_budget,
        /* default_deadline */ Duration::from_millis(300),
        /* hedge_stagger   */ Duration::from_millis(25),
        /* min_leg_budget  */ Duration::from_millis(50),
        lookup_ctx.clone(),
    )
    .await?;
    info!(%admin_addr, "svc-dht admin up");

    // Bootstrap routing state & supervision
    let sup = bootstrap::spawn_bootstrap_supervisor(
        cfg.clone(),
        health.clone(),
        ready.clone(),
        metrics.clone(),
    )
    .await?;

    // Wait for Ctrl-C and shutdown
    wait_for_ctrl_c().await;
    warn!("shutdown requested");
    sup.shutdown().await;
    admin_task.abort();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn serve_admin(
    bind: SocketAddr,
    health: Arc<HealthState>,
    ready: Arc<ReadyGate>,
    metrics: Arc<DhtMetrics>,
    providers: Arc<ProviderStore>,
    alpha: usize,
    beta: usize,
    hop_budget: usize,
    default_deadline: Duration,
    hedge_stagger: Duration,
    min_leg_budget: Duration,
    lookup_ctx: Arc<LookupCtx>,
) -> anyhow::Result<(JoinHandle<()>, SocketAddr)> {
    let app = Router::new()
        .route("/healthz", get(http::healthz))
        .route("/readyz", get(http::readyz))
        .route("/version", get(http::version))
        .route("/metrics", get(http::metrics))
        .route("/dht/find_providers/:cid", get(http::find_providers))
        .route("/dht/provide", post(http::provide))
        .route("/dht/_debug/list", get(http::debug_list))
        .with_state(http::State::new(
            health,
            ready,
            metrics,
            providers,
            alpha,
            beta,
            hop_budget,
            default_deadline,
            hedge_stagger,
            min_leg_budget,
            lookup_ctx,
        ));

    let listener = tokio::net::TcpListener::bind(bind).await?;
    let addr = listener.local_addr()?;
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Ok((task, addr))
}
