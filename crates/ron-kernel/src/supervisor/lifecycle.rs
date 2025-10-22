use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::events::KernelEvent;
use crate::internal::types::{BoxError, ServiceName};
use crate::metrics::{exporter::Metrics, health::HealthState};
use crate::Bus;

use super::backoff::Backoff;
use super::child::run_once;

pub struct Supervisor {
    name: ServiceName,
    metrics: Metrics,
    bus: Bus<KernelEvent>,
    health: HealthState,
    backoff: Backoff,
    max_restarts: u32,
    window: Duration,
    recent: VecDeque<Instant>,
}

impl Supervisor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: ServiceName,
        metrics: Metrics,
        bus: Bus<KernelEvent>,
        health: HealthState,
        backoff: Backoff,
        max_restarts: u32,
        window: Duration,
    ) -> Self {
        Self {
            name,
            metrics,
            bus,
            health,
            backoff,
            max_restarts,
            window,
            recent: VecDeque::with_capacity(max_restarts as usize + 1),
        }
    }

    pub async fn run<F, Fut>(&mut self, work: F) -> !
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<(), BoxError>> + Send + 'static,
    {
        self.health.set(self.name, true);
        loop {
            let factory = work.clone();
            let _ = run_once(self.name, &self.metrics, &self.bus, factory).await;
            self.health.set(self.name, false);

            let now = Instant::now();
            self.recent.push_back(now);
            while let Some(&front) = self.recent.front() {
                if now.duration_since(front) > self.window {
                    self.recent.pop_front();
                } else {
                    break;
                }
            }
            if self.recent.len() as u32 > self.max_restarts {
                let _ = self.backoff.next();
                let cap = self.backoff.next();
                tokio::time::sleep(cap).await;
                continue;
            }

            let sleep = self.backoff.next();
            tokio::time::sleep(sleep).await;
            self.health.set(self.name, true);
        }
    }
}
