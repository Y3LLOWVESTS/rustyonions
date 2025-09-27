//! Microkernel event bus (module root).
//!
//! Structure:
//! - core.rs: `Bus` type + core logic (capacity, publish, overflow throttling).
//! - metrics.rs: Prometheus counter for dropped events.
//! - helpers.rs: lag-aware recv helpers (blocking and non-blocking).
//! - sub.rs: topic-style helpers (timeouts, matching, try-now).
//!
//! Public surface keeps compatibility with the previous single-file version.

#![forbid(unsafe_code)]

mod core;
mod helpers;
mod metrics;
pub mod sub;

pub use core::Bus;
pub use helpers::{recv_lag_aware, try_recv_lag_aware};
