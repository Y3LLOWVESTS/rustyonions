#![forbid(unsafe_code)]

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use prometheus::{IntCounterVec, GaugeVec};
use tokio::{
    task::JoinHandle,
    time::{sleep, timeout},
};
use tracing::{info, warn};

use crate::{bus::Bus, cancel::Shutdown, metrics::HealthState, KernelEvent, Metrics};
use super::policy::{RestartPolicy, compute_backoff};
use super::metrics::{restarts_metric, backoff_metric};

type BoxFut = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;
type ServiceFactory = Arc<dyn Fn(Shutdown) -> BoxFut + Send + Sync>;

struct Service { name: String, factory: ServiceFactory }

pub struct Supervisor {
    bus: Bus,
    _metrics: Arc<Metrics>,
    health: Arc<HealthState>,
    root_sdn: Shutdown,
    services: Vec<Service>,
}

pub struct SupervisorHandle {
    root_sdn: Shutdown,
    join: JoinHandle<()>,
}

impl Supervisor {
    pub fn new(bus: Bus, metrics: Arc<Metrics>, health: Arc<HealthState>, sdn: Shutdown) -> Self {
        Self { bus, _metrics: metrics, health, root_sdn: sdn, services: Vec::new() }
    }

    pub fn add_service<F, Fut>(&mut self, name: &str, f: F)
    where
        F: Fn(Shutdown) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let name = name.to_string();
        let factory: ServiceFactory = Arc::new(move |sdn| { let fut = f(sdn); Box::pin(fut) });
        self.services.push(Service { name, factory });
    }

    pub fn spawn(self) -> SupervisorHandle {
        info!("Supervisor starting {} services…", self.services.len());
        let join = tokio::spawn(run_supervisor(self));
        SupervisorHandle { root_sdn: join_root(&join), join }
    }
}

impl SupervisorHandle {
    pub fn shutdown(&self) { self.root_sdn.cancel(); }
    pub async fn join(self) -> anyhow::Result<()> { let _ = self.join.await; Ok(()) }
}

/* =========================== internal runner ============================== */

async fn run_supervisor(mut sup: Supervisor) {
    let restarts = restarts_metric();
    let backoff_g = backoff_metric();

    let mut tasks: HashMap<String, JoinHandle<()>> = HashMap::new();
    let policy = RestartPolicy::default();

    for svc in sup.services.drain(..) {
        let name = svc.name.clone();
        let j = spawn_service_loop(
            name.clone(), svc.factory.clone(),
            sup.bus.clone(), sup.health.clone(), sup.root_sdn.clone(),
            restarts.clone(), backoff_g.clone(), policy,
        );
        tasks.insert(name, j);
    }

    // Wait for root shutdown, then let tasks exit when their children see it.
    sup.root_sdn.cancelled().await;

    // Join all children best-effort
    for (_, j) in tasks.into_iter() {
        let _ = j.await;
    }
}

/* ============================= restart loop =============================== */

fn spawn_service_loop(
    name: String,
    factory: ServiceFactory,
    bus: Bus,
    health: Arc<HealthState>,
    root_sdn: Shutdown,
    restarts: IntCounterVec,
    backoff_g: GaugeVec,
    policy: RestartPolicy,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut gen: u64 = 0;

        loop {
            // Non-blocking check: if the shutdown has already been requested, stop restarting.
            if timeout(Duration::from_millis(0), root_sdn.cancelled()).await.is_ok() {
                break;
            }

            health.set(&name, false);
            info!(target: "ron_kernel::supervisor", service = %name, "service starting");
            let sdn = root_sdn.child();

            let fut = (factory)(sdn.clone());
            let name_clone = name.clone();

            match fut.await {
                Ok(()) => {
                    health.set(&name_clone, false);

                    // If root shutdown was requested while the service was running, do not restart.
                    if timeout(Duration::from_millis(0), root_sdn.cancelled()).await.is_ok() {
                        break;
                    }

                    // Treat a clean exit as a crash for supervision semantics, then back off and restart.
                    let reason = "exited_ok";
                    let _ = bus.publish(KernelEvent::ServiceCrashed {
                        service: name_clone.clone(),
                        reason: reason.to_string(),
                    });
                    let delay = compute_backoff(&policy, gen);
                    backoff_g.with_label_values(&[&name_clone]).set(delay.as_secs_f64());
                    restarts.with_label_values(&[&name_clone]).inc();
                    sleep(delay).await;
                    gen = gen.saturating_add(1);
                }
                Err(e) => {
                    health.set(&name_clone, false);
                    let reason = format!("error: {e:#}");
                    let _ = bus.publish(KernelEvent::ServiceCrashed { service: name_clone.clone(), reason });
                    let delay = compute_backoff(&policy, gen);
                    backoff_g.with_label_values(&[&name_clone]).set(delay.as_secs_f64());
                    restarts.with_label_values(&[&name_clone]).inc();
                    warn!(target="ron_kernel::supervisor", service=%name_clone, "service crashed; restarting after backoff");
                    sleep(delay).await;
                    gen = gen.saturating_add(1);
                }
            }
        }
    })
}

/* ================================ helpers ================================= */

fn join_root(_j: &JoinHandle<()>) -> Shutdown {
    // Kept for API parity with previous handle.shutdown().
    Shutdown::new()
}
