// crates/macronode/src/supervisor/mod.rs

//! RO:WHAT — Process supervisor scaffold for Macronode.
//! RO:WHY  — Central place to coordinate service startup/shutdown.
//! RO:INVARIANTS —
//!   - Crash policy + backoff are wired but *not yet* used to restart tasks.
//!   - Graceful shutdown orchestration is still a future slice.
//!   - Health reporting to readiness/admin planes is still a future slice.
//!   - This slice adds task watchers that log service exits and tick
//!     restart counters, but they do NOT restart services yet.

#![allow(dead_code)]

mod backoff;
mod crash_policy;
mod health_reporter;
mod lifecycle;
mod shutdown;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::{errors::Result, readiness::ReadyProbes, services};

pub use lifecycle::ManagedTask;
pub use shutdown::ShutdownToken;

use backoff::Backoff;
use crash_policy::CrashPolicy;
use health_reporter::HealthSnapshot;
use lifecycle::LifecycleState;
use tracing::{error, info};

/// Macronode process supervisor (MVP).
///
/// Currently minimal: only boots services. This module now also contains the
/// internal logic for combining CrashPolicy + Backoff into a restart
/// decision API. In this slice we additionally attach *watchers* to service
/// tasks so we log when they exit, but we still DO NOT restart anything.
#[derive(Debug)]
pub struct Supervisor {
    /// Shared readiness probes used by admin plane and readiness endpoints.
    probes: Arc<ReadyProbes>,
    /// Cooperative shutdown token shared with all managed services.
    shutdown: ShutdownToken,
    /// Coarse lifecycle state.
    lifecycle: LifecycleState,
    /// Aggregated view of per-service health.
    health: HealthSnapshot,
    /// Policy controlling restart aggressiveness.
    crash_policy: CrashPolicy,
    /// Exponential backoff state used between restarts.
    backoff: Backoff,
    /// Rolling crash timestamps per logical service name.
    ///
    /// This log is consulted by `crash_policy.should_restart(...)` to decide
    /// whether a service is still within its restart budget.
    crash_log: HashMap<&'static str, Vec<Instant>>,
}

impl Supervisor {
    /// Construct a new supervisor handle.
    pub fn new(probes: Arc<ReadyProbes>, shutdown: ShutdownToken) -> Self {
        let crash_policy = CrashPolicy::new(5, Duration::from_secs(60));
        let backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));

        Supervisor {
            probes,
            shutdown,
            lifecycle: LifecycleState::Starting,
            health: HealthSnapshot::default(),
            crash_policy,
            backoff,
            crash_log: HashMap::new(),
        }
    }

    /// Start all managed services.
    ///
    /// This now delegates to `services::spawn_all`, receives a vector of
    /// `ManagedTask` handles, and attaches lightweight watcher tasks that
    /// simply log when services exit (cleanly, cancelled, or crashed).
    /// There is STILL no restart logic in this slice.
    pub async fn start(&self) -> Result<()> {
        let tasks: Vec<ManagedTask> =
            services::spawn_all(self.probes.clone(), self.shutdown.clone()).await?;

        self.spawn_watchers(tasks);
        Ok(())
    }

    /// Attach background watchers to each managed service task.
    ///
    /// Each watcher:
    ///   - awaits the JoinHandle
    ///   - logs on clean exit, cancellation, or crash
    ///   - ticks the appropriate restart counter on crash
    ///   - does NOT restart or mutate readiness gates yet
    fn spawn_watchers(&self, tasks: Vec<ManagedTask>) {
        if tasks.is_empty() {
            return;
        }

        for task in tasks {
            let service = task.service_name;
            let handle = task.handle;
            let probes = Arc::clone(&self.probes);

            tokio::spawn(async move {
                match handle.await {
                    Ok(()) => {
                        info!(
                            %service,
                            "macronode supervisor: service task exited cleanly"
                        );
                    }
                    Err(err) if err.is_cancelled() => {
                        info!(
                            %service,
                            "macronode supervisor: service task cancelled (likely shutdown)"
                        );
                    }
                    Err(err) => {
                        error!(
                            %service,
                            %err,
                            "macronode supervisor: service task crashed"
                        );
                        // Record a crash for this service so that admin-plane
                        // `/api/v1/status` can expose restart counters.
                        probes.inc_restart_for(service);
                    }
                }
            });
        }
    }

    // ---------------------------------------------------------------------
    //  Crash policy + backoff glue (internal API, not used for restarts yet)
    // ---------------------------------------------------------------------

    /// Record a crash event for a service.
    ///
    /// This is a pure bookkeeping method: it appends `Instant::now()` to the
    /// crash log for `service`. A future watcher loop will call this whenever
    /// a worker task exits with an error or panic.
    pub fn record_crash(&mut self, service: &'static str) {
        let now = Instant::now();
        let entry = self.crash_log.entry(service).or_default();
        entry.push(now);
    }

    /// Decide how long to wait before restarting a crashed service.
    ///
    /// Returns:
    ///   - `Some(delay)` if we are still within the restart budget for
    ///     `service`, where `delay` is derived from the exponential backoff.
    ///   - `None` if we should *not* restart anymore (permanent failure).
    ///
    /// This method is intentionally side-effect-free except for advancing
    /// the backoff state; it does not spawn tasks or toggle readiness flags.
    pub fn restart_delay(&mut self, service: &'static str) -> Option<Duration> {
        let now = Instant::now();

        let crashes = self
            .crash_log
            .get(service)
            .map(Vec::as_slice)
            .unwrap_or(&[]);

        // If we've exceeded the allowed number of restarts within the window,
        // bail out and let the caller surface a permanent failure.
        if !self.crash_policy.should_restart(crashes, now) {
            return None;
        }

        // Otherwise we are allowed to restart; advance the backoff sequence.
        Some(self.backoff.next_delay())
    }

    /// Reset backoff state and optionally clear crash history.
    ///
    /// For now we only reset the backoff counter and leave the crash log
    /// intact. A future slice may choose to clear `crash_log` entries for
    /// services that have been healthy for long enough.
    pub fn reset_backoff(&mut self) {
        self.backoff.reset();
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Supervisor::new(Arc::new(ReadyProbes::new()), ShutdownToken::new())
    }
}
