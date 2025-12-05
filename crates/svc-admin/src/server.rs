// crates/svc-admin/src/server.rs

//! Server bootstrap for svc-admin.

use crate::{
    config::Config,
    metrics::{
        prometheus_bridge,
        sampler::{self, NodeMetricsTarget},
    },
    observability,
    router,
    state::AppState,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    signal,
    sync::watch,
};

/// Run svc-admin with the given config.
pub async fn run(config: Config) -> Result<()> {
    observability::init_tracing();

    // Initialize static metrics derived from config.
    prometheus_bridge::init_node_inventory_metrics(&config);

    let state = Arc::new(AppState::new(config.clone()));

    // Spawn background facet samplers for any configured nodes.
    //
    // These will:
    // - scrape `<base_url>/metrics` on each node
    // - aggregate facet counters into `state.facet_metrics`
    // - exit promptly when the shutdown channel is tripped
    let sampler_shutdown_tx = spawn_facet_samplers(&config, &state);

    let app = router::build_router(state.clone());

    // We validated these addresses during config load.
    let ui_bind_addr = &config.server.bind_addr;
    let metrics_bind_addr = &config.server.metrics_addr;

    let main_listener = TcpListener::bind(ui_bind_addr).await?;
    let metrics_listener = TcpListener::bind(metrics_bind_addr).await?;

    tracing::info!(
        bind_addr = %ui_bind_addr,
        "svc-admin listening for UI/API",
    );
    tracing::info!(
        bind_addr = %metrics_bind_addr,
        "svc-admin listening for health/metrics",
    );

    // For now we run the metrics/health server as a simple background task.
    // If we want full graceful shutdown here as well, we can add a separate
    // shutdown future, but Ctrl+C will terminate the process either way.
    let metrics_app = app.clone();
    let metrics_task = tokio::spawn(async move {
        if let Err(err) = axum::serve(metrics_listener, metrics_app).await {
            tracing::error!(error = ?err, "metrics/health server error");
        }
    });

    // Main UI/API server with graceful shutdown.
    let shutdown = shutdown_signal(sampler_shutdown_tx);
    let main_task = axum::serve(main_listener, app)
        .with_graceful_shutdown(shutdown);

    tokio::select! {
        res = main_task => {
            if let Err(err) = res {
                tracing::error!(error = ?err, "main server error");
            }
        }
        _ = metrics_task => {
            tracing::warn!("metrics/health task exited");
        }
    }

    Ok(())
}

/// Build `NodeMetricsTarget`s from config and spawn facet samplers.
///
/// Returns a shutdown sender that can be used to stop all samplers.
fn spawn_facet_samplers(
    config: &Config,
    state: &Arc<AppState>,
) -> watch::Sender<bool> {
    // Channel used to broadcast shutdown to all sampler tasks.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Derive one metrics target per configured node.
    let mut targets = Vec::new();
    for (id, node_cfg) in &config.nodes {
        let base = node_cfg.base_url.trim_end_matches('/');

        // For now we assume each node exposes a Prometheus text endpoint at `/metrics`.
        let metrics_url = format!("{base}/metrics");

        targets.push(NodeMetricsTarget {
            node_id: id.clone(),
            metrics_url,
            timeout: node_cfg.default_timeout,
        });
    }

    if targets.is_empty() {
        tracing::info!("no nodes configured; facet sampler pool will remain empty");
        return shutdown_tx;
    }

    let facet_metrics = state.facet_metrics.clone();
    let interval = config.polling.metrics_interval;

    tracing::info!(
        node_count = targets.len(),
        interval_secs = ?interval.as_secs(),
        "spawning facet metrics samplers for configured nodes",
    );

    // Spawn one sampler task per node. We intentionally do not track the
    // JoinHandles yet; they are long-lived background tasks whose lifetime
    // is bound to the process. In a future slice we can add explicit
    // supervision and restart logic if needed.
    let _handles = sampler::spawn_samplers(targets, interval, facet_metrics, shutdown_rx);

    shutdown_tx
}

/// Shutdown future used by Axum to drain the main server.
///
/// This waits for Ctrl+C and then:
/// - logs that we're shutting down
/// - signals all sampler tasks to exit by sending `true` on the watch channel
async fn shutdown_signal(sampler_shutdown_tx: watch::Sender<bool>) {
    if let Err(err) = signal::ctrl_c().await {
        tracing::error!(error = ?err, "failed to install Ctrl+C handler");
        return;
    }

    tracing::info!("shutdown signal received, draining svc-admin");

    if let Err(err) = sampler_shutdown_tx.send(true) {
        tracing::warn!(
            error = ?err,
            "failed to send shutdown signal to facet samplers (receiver dropped?)"
        );
    }
}
