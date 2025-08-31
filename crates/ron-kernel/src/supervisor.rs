#![forbid(unsafe_code)]

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use rand::{rng, Rng};
use tokio::{task::JoinHandle, time::sleep};
use tracing::{error, info, warn};

use crate::{bus::Bus, metrics::HealthState, KernelEvent, Metrics};
use crate::cancel::Shutdown;

/// A supervised service defined by a name and an async entrypoint that respects `Shutdown`.
type BoxedSvc =
    Box<dyn Fn(Shutdown) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>> + Send + Sync>;

struct ServiceSpec {
    name: &'static str,
    make_task: BoxedSvc,
}

/// Supervisor spawns and restarts services with backoff and reports via Bus/Metrics/Health.
pub struct Supervisor {
    bus: Bus,
    metrics: Arc<Metrics>,
    health: Arc<HealthState>,
    services: Vec<ServiceSpec>,
    shutdown: Shutdown,
}

impl Supervisor {
    pub fn new(bus: Bus, metrics: Arc<Metrics>, health: Arc<HealthState>, shutdown: Shutdown) -> Self {
        Self { bus, metrics, health, services: Vec::new(), shutdown }
    }

    pub fn add_service<F, Fut>(&mut self, name: &'static str, f: F)
    where
        F: Fn(Shutdown) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let make_task: BoxedSvc = Box::new(move |sdn: Shutdown| Box::pin(f(sdn)));
        self.services.push(ServiceSpec { name, make_task });
    }

    pub fn spawn(self) -> SupervisorHandle {
        let (bus, metrics, health, shutdown, services) =
            (self.bus.clone(), self.metrics.clone(), self.health.clone(), self.shutdown.clone(), self.services);

        let handle = tokio::spawn(async move {
            info!("Supervisor starting {} services…", services.len());

            let mut joins: Vec<JoinHandle<()>> = Vec::with_capacity(services.len());
            for svc in services {
                let svc_bus = bus.clone();
                let svc_metrics = metrics.clone();
                let svc_health = health.clone();
                let parent_shutdown = shutdown.clone();
                joins.push(tokio::spawn(run_supervised_service(
                    svc.name,
                    svc.make_task,
                    svc_bus,
                    svc_metrics,
                    svc_health,
                    parent_shutdown,
                )));
            }

            shutdown.cancelled().await;
            info!("Supervisor received shutdown, waiting for services…");

            for j in joins {
                let _ = j.await;
            }

            info!("Supervisor exited cleanly.");
        });

        SupervisorHandle { join: handle, shutdown: self.shutdown.clone() }
    }
}

pub struct SupervisorHandle {
    join: tokio::task::JoinHandle<()>,
    shutdown: Shutdown,
}

impl SupervisorHandle {
    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }

    pub async fn join(self) -> anyhow::Result<()> {
        self.join.await.map_err(|e| anyhow::anyhow!("supervisor join error: {e}"))?;
        Ok(())
    }
}

async fn run_supervised_service(
    name: &'static str,
    make_task: BoxedSvc,
    bus: Bus,
    metrics: Arc<Metrics>,
    health: Arc<HealthState>,
    parent_sd: Shutdown,
) {
    let min = Duration::from_millis(200);
    let max = Duration::from_secs(30);
    let mut backoff = min;
    let mut last_ok: Option<Instant> = None;
    let mut child = parent_sd.child();

    loop {
        health.set(name, true);
        info!(service = name, "service starting");

        let result = {
            let task = (make_task)(child.clone());
            tokio::select! {
                biased;
                _ = parent_sd.cancelled() => {
                    child.cancel();
                    let _ = tokio::time::timeout(Duration::from_secs(2), child.cancelled()).await;
                    info!(service = name, "service shutdown requested");
                    return;
                }
                res = task => res,
            }
        };

        match result {
            Ok(()) => {
                info!(service = name, "service returned Ok; treating as graceful stop");
                health.set(name, false);
                return;
            }
            Err(err) => {
                health.set(name, false);
                let reason = format!("{err:#}");
                let _ = bus.publish(KernelEvent::ServiceCrashed {
                    service: name.to_string(),
                    reason: reason.clone(),
                });
                error!(service = name, %reason, "service crashed");

                metrics.service_restarts_total.with_label_values(&[name]).inc();

                let now = Instant::now();
                if let Some(t) = last_ok {
                    if now.duration_since(t) > Duration::from_secs(10) {
                        backoff = min;
                    }
                }
                let jitter = 0.85 + (rng().random::<f64>() * 0.30);
                let wait = mul_f64(backoff, jitter).min(max);
                warn!(service = name, secs = wait.as_secs_f64(), "restarting after backoff");

                tokio::select! {
                    _ = parent_sd.cancelled() => {
                        info!(service = name, "shutdown during backoff; not restarting");
                        return;
                    }
                    _ = sleep(wait) => {}
                }

                backoff = (backoff * 2).min(max);
                child = parent_sd.child();
            }
        }

        last_ok = Some(Instant::now());
    }
}

#[inline]
fn mul_f64(d: Duration, by: f64) -> Duration {
    let nanos = (d.as_nanos() as f64 * by) as u128;
    Duration::from_nanos(nanos as u64)
}
