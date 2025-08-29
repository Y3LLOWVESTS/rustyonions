//! ron-kernel: core microkernel primitives for RustyOnions.

pub mod bus;
pub mod cancel;
pub mod supervisor;
pub mod tracing_init;
pub mod transport;

pub use bus::{Bus, Event};
pub use cancel::Shutdown;
pub use supervisor::{spawn_supervised, Backoff, SupervisorOptions};
pub use tracing_init::tracing_init;
pub use transport::{
    TransportOptions, TransportHandle, TransportStatsClient, TransportStatsSnapshot,
    spawn_transport,
};
