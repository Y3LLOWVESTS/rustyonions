//! RO:WHAT — Translate ron-bus events into metrics + health updates.
//! RO:WHY  — Unified observability; keep ron-bus lean, exporter lives here.
//! RO:INTERACTS — ron_bus::{Bus, Event}; Metrics; HealthState.
//! RO:INVARIANTS — single subscriber; bounded; no lock across .await.
//! RO:TODO — If ron-bus later emits lag/overwrite events, extend the match arms here behind the same feature.

#![allow(dead_code)]

#[cfg(feature = "bus")]
mod impls {
    use crate::Metrics;
    use ron_bus::{Bus, Event};
    use tokio::task::JoinHandle;
    use tokio::time::{sleep, Duration};
    use tracing::{info, warn};

    /// Start a watcher that consumes events from the shared bus and updates metrics/health.
    /// We only borrow the `Bus` to create a subscriber; the subscriber is moved into the task.
    pub fn start_bus_watcher(metrics: Metrics, bus: &Bus, watcher_name: &'static str) -> JoinHandle<()> {
        // Each subscriber has its own cursor by design.
        let mut sub = bus.subscribe();

        tokio::spawn(async move {
            info!(watcher=%watcher_name, "ron-metrics: bus watcher started");
            loop {
                match sub.recv().await {
                    Ok(ev) => match ev {
                        Event::Health { service, ok } => {
                            metrics.set_ready(&service, ok);
                        }
                        Event::Shutdown => {
                            info!(watcher=%watcher_name, "ron-metrics: bus watcher received Shutdown; exiting");
                            break;
                        }
                        // Extend here when new events are added (e.g., ConfigUpdated, BusLag, etc.)
                        _ => {}
                    },
                    Err(e) => {
                        // Channel closed or transient error — back off a hair to avoid hot loop.
                        warn!(watcher=%watcher_name, error=?e, "bus watcher recv error; backing off");
                        sleep(Duration::from_millis(5)).await;
                    }
                }
            }
        })
    }
}

#[cfg(feature = "bus")]
pub use impls::start_bus_watcher;

// If the feature is off, expose a stub so callsites can compile behind cfg.
#[cfg(not(feature = "bus"))]
pub fn start_bus_watcher(_: crate::Metrics, _: (), _: &'static str) -> () { () }
