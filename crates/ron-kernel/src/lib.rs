#![forbid(unsafe_code)]
//! ron-kernel2: microkernel scaffold. Public API is intentionally tiny and frozen.

pub use bus::Bus;
pub use events::KernelEvent;
pub use metrics::{Metrics, HealthState};
pub use config::Config;
pub use shutdown::wait_for_ctrl_c;

pub mod bus;
mod events;
pub mod metrics;
pub mod config;
pub mod supervisor;
pub mod amnesia;
pub mod shutdown;
mod internal;
