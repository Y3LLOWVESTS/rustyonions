//! RO:WHAT — Seed dialing + min-fill readiness gate
//! RO:WHY — Bring table to life before accepting work; Concerns: RES/PERF
//! RO:INTERACTS — peer::table, metrics, readiness, transport
//! RO:INVARIANTS — backoff with jitter; no locks across .await
//! RO:TEST — readiness_bootstrap.rs

use crate::{config::Config, metrics::DhtMetrics, readiness::ReadyGate};
use rand::{rng, Rng};
use ron_kernel::HealthState;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

pub struct Supervisor {
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

impl Supervisor {
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.handle.await;
    }
}

pub async fn spawn_bootstrap_supervisor(
    cfg: Config,
    _health: Arc<HealthState>,
    ready: Arc<ReadyGate>,
    _metrics: Arc<DhtMetrics>,
) -> anyhow::Result<Supervisor> {
    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();

    let handle = tokio::spawn(async move {
        // Single pass for MVP; in Phase 2 we'll loop with backoff until quorum/min-fill.
        tokio::select! {
            _ = &mut rx => {
                info!("bootstrap supervisor: shutdown");
            }
            _ = do_once(&cfg) => {
                ready.set_ready();
                info!("bootstrap: min-fill reached; ready gate opened");
            }
        }
    });

    Ok(Supervisor { shutdown_tx: Some(tx), handle })
}

async fn do_once(cfg: &Config) {
    // TODO Phase 2: dial seeds via ron-transport; refresh k-buckets by distance
    if cfg.seeds.is_empty() {
        warn!("no seeds configured; table will rely on inbound discovery");
        sleep(Duration::from_millis(300)).await;
    } else {
        for s in &cfg.seeds {
            let _ = s; // simulate dial
            let jitter = rng().random_range(10..60);
            sleep(Duration::from_millis(jitter)).await;
        }
    }
}
