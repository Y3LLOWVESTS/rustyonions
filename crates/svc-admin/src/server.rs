// crates/svc-admin/src/server.rs
//
//! Server bootstrap for svc-admin.

use crate::{
    config::Config,
    metrics::{
        prometheus_bridge,
        sampler::{self, NodeMetricsTarget},
    },
    observability, router,
    state::AppState,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::{net::TcpListener, signal, sync::watch};

/// Run svc-admin with the given config.
pub async fn run(config: Config) -> Result<()> {
    observability::init_tracing();

    // Initialize static metrics derived from config.
    prometheus_bridge::init_node_inventory_metrics(&config);

    let state = Arc::new(AppState::new(config.clone())?);

    // Spawn background facet samplers for any configured nodes.
    let sampler_shutdown_tx = spawn_facet_samplers(&config, &state);

    // Build routers (both are Router<Arc<AppState>>).
    let app = router::build_router(state.clone());
    let metrics_app = router::build_metrics_router(state.clone());

    // We validated these addresses during config load.
    let ui_bind_addr = &config.server.bind_addr;
    let metrics_bind_addr = &config.server.metrics_addr;

    let main_listener = TcpListener::bind(ui_bind_addr).await?;
    let metrics_listener = TcpListener::bind(metrics_bind_addr).await?;

    tracing::info!(bind_addr = %ui_bind_addr, "svc-admin listening for UI/API");
    tracing::info!(bind_addr = %metrics_bind_addr, "svc-admin listening for health/metrics");

    // NOTE (axum 0.7):
    // `axum::serve(listener, router)` accepts a Router directly (no into_make_service needed).
    // This is the same pattern we already use in svc-admin tests.
    let mut metrics_task = tokio::spawn(async move {
        axum::serve(metrics_listener, metrics_app).await
    });

    let shutdown = shutdown_signal(sampler_shutdown_tx);
    let main_task = tokio::spawn(async move {
        axum::serve(main_listener, app)
            .with_graceful_shutdown(shutdown)
            .await
    });

    tokio::select! {
        res = main_task => {
            // If the main server exits, abort the metrics server task so we don't
            // leave a listener running unexpectedly.
            metrics_task.abort();

            match res {
                Ok(Ok(())) => {
                    tracing::info!("main server exited cleanly");
                }
                Ok(Err(err)) => {
                    tracing::error!(error = ?err, "main server error");
                }
                Err(join_err) => {
                    tracing::error!(error = ?join_err, "main server task join error");
                }
            }
        }
        res = &mut metrics_task => {
            match res {
                Ok(Ok(())) => {
                    tracing::warn!("metrics/health server exited cleanly");
                }
                Ok(Err(err)) => {
                    tracing::error!(error = ?err, "metrics/health server error");
                }
                Err(join_err) => {
                    tracing::error!(error = ?join_err, "metrics/health task join error");
                }
            }
        }
    }

    Ok(())
}

/// Build `NodeMetricsTarget`s from config and spawn facet samplers.
///
/// Returns a shutdown sender that can be used to stop all samplers.
fn spawn_facet_samplers(config: &Config, state: &Arc<AppState>) -> watch::Sender<bool> {
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
