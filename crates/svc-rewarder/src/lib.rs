// Public surface (stub). Re-export types as they are added.
pub mod prelude;
pub mod config;
pub mod http;
pub mod core;
pub mod inputs;
pub mod outputs;
pub mod metrics;
pub mod readiness;
pub mod bus;
pub mod security;
pub mod util;

// Re-exports (will be real types later)
pub struct Metrics;
pub struct HealthState;
pub struct Bus;
pub enum KernelEvent { Health { service: String, ok: bool } }

pub fn wait_for_ctrl_c() {}
