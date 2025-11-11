#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]
//! svc-edge — stateless edge service (admin + api scaffold).
//!
//! RO:WHAT
//! - Minimal library surface that re-exports kernel facilities used by the binary.
//! - No stable SDK; this is a service crate.
//!
//! RO:WHY
//! - Keep compile surface tiny so we can add features incrementally.
//!
//! RO:INVARIANTS
//! - Admin plane is always available: /metrics, /healthz, /readyz.
//! - Readiness is degrade-first until gates flip ready.

pub mod cli;
pub mod config;
pub mod errors;
pub mod metrics;
pub mod readiness;
pub mod routes;
pub mod state;
pub mod admission; // <— expose admission chain builder

pub use ron_kernel::{wait_for_ctrl_c, HealthState};

/// Public convenience re-exports for the binary.
pub use config::Config;
pub use metrics::EdgeMetrics;
pub use state::AppState;
