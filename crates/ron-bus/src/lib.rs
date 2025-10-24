//! RO:WHAT — Public surface for the in-process broadcast bus (bounded, lossy, observable-by-host)
//! RO:WHY  — Pillar 1 (Kernel & Orchestration); Concerns: RES/PERF (bounded backpressure, no locks across .await)
//! RO:INTERACTS — internal::channel (tokio::broadcast wrapper); public: Bus, BusConfig, Event, BusError
//! RO:INVARIANTS — bounded channel; one receiver per task; no background tasks; no secrets/PII on bus
//! RO:METRICS — none inside crate (host updates counters/gauges in recv loop)
//! RO:CONFIG — capacity fixed at construction; cutover by constructing a new Bus
//! RO:SECURITY — no network/disk I/O; no payload logging; secret-free surface
//! RO:TEST — integration tests in tests/*; loom model optional (cfg(loom))

#![forbid(unsafe_code)]
#![deny(warnings)]

mod bus;
mod config;
mod errors;
mod event;

pub mod metrics;
pub mod prelude;

pub mod internal; // kept small; still non-public APIs within it

pub use bus::Bus;
pub use config::BusConfig;
pub use errors::BusError;
pub use event::Event;

#[cfg(doctest)]
mod _doctests {
    /// Minimal host-side pattern (bounded, one receiver per task).
    ///
    /// ```ignore
    /// use ron_bus::{Bus, BusConfig, Event};
    /// # #[tokio::main(flavor="current_thread")]
    /// # async fn main() {
    /// let bus = Bus::new(BusConfig::default()).unwrap();
    /// let mut rx = bus.subscribe();
    /// tokio::spawn(async move {
    ///     loop {
    ///         match rx.recv().await {
    ///             Ok(_ev) => { /* handle */ }
    ///             Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
    ///                 // host increments metrics here (outside this library)
    ///                 // metrics::bus_overflow_dropped_total().inc_by(n as u64);
    ///             }
    ///             Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
    ///         }
    ///     }
    /// });
    /// bus.sender().send(Event::Shutdown).ok();
    /// # }
    /// ```
    fn _marker() {}
}
