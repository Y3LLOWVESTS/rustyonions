//! svc-passport2 library surface (scaffold). Keep modules <300 LOC when implemented.
pub mod config;
pub mod error;
pub mod metrics;
pub mod health;
pub mod bootstrap;

pub mod http;
pub mod dto;
pub mod token;
pub mod kms;
pub mod verify;
pub mod state;
pub mod policy;
pub mod bus;
pub mod telemetry;
pub mod util;

// Re-export candidates (uncomment when implemented):
// pub use crate::{config::Config, metrics::Metrics, health::Health};
