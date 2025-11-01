//! RO:WHAT — Lightweight runtime layer: supervised background workers + cooperative shutdown.
//! RO:WHY  — Keep App/router lean; side-loops (samplers, refreshers, warmers) live here.
//! RO:INTERACTS — ron-kernel (Bus/Events), Metrics (recorders), admission, policy.
//! RO:INVARIANTS — Single owner per worker task; graceful stop within timeout; no locks across .await.

mod channels;
mod shutdown;
mod supervisor;
mod worker;

pub mod sample;

pub use channels::{mk_supervisor_bus, SupervisorMsg, WorkerMsg};
pub use shutdown::{pair as shutdown_pair, Shutdown, ShutdownTrigger};
pub use supervisor::{spawn_supervisor, SupervisorHandle};
pub use worker::{spawn_worker, DynWorker, Worker};
