//! ron-app-sdk2 — library facade (scaffold).
//! Keep implementation small and modular; expose public surface & planes.

pub mod config;
pub mod context;
pub mod errors;
pub mod retry;
pub mod idempotency;
pub mod tracing;
pub mod metrics;
pub mod cache;
pub mod transport;
pub mod planes;
pub mod ready;
pub mod types;