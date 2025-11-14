#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]
//! svc-edge — stateless edge service (admin + api scaffold).
//!
//! RO:WHAT
//! - Minimal library surface that re-exports kernel facilities used by the binary.
//! - Admission middleware, config, metrics, readiness, and routes are exposed for tests.
//!
//! RO:WHY
//! - Keep compile surface tiny so we can add features incrementally.
//!
//! RO:INVARIANTS
//! - Admin plane is always available: /metrics, /healthz, /readyz.
//! - Readiness is degrade-first until gates flip ready.
//! - `AppState` is `Clone + Send + Sync + 'static` so routers can use `into_make_service()`
//!   cleanly under Axum 0.7.

/// Command-line parsing and process flags.
pub mod cli;

/// Runtime configuration parsing/validation.
pub mod config;

/// Service-local error types.
pub mod errors;

/// Prometheus metrics (edge_* counters/histograms).
pub mod metrics;

/// Readiness gates and handler.
pub mod readiness;

/// HTTP route handlers (admin + API).
pub mod routes;

/// Shared application state (must be Clone + Send + Sync).
pub mod state;

/// Admission chain (ingress guards applied to API router).
pub mod admission;

// Re-export kernel helpers used by the bin.
pub use ron_kernel::{wait_for_ctrl_c, HealthState};

/// Public convenience re-exports for the binary and integration tests.
pub use config::Config;
pub use metrics::EdgeMetrics;
pub use state::AppState;
