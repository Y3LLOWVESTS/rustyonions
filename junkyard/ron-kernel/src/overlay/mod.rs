#![forbid(unsafe_code)]

//! Overlay service modules and re-exports.

pub mod admin_http;
pub mod metrics;
pub mod runtime;
pub mod service;
pub mod tls;

// Re-exports for ergonomic imports
pub use metrics::{init_overlay_metrics, OverlayMetrics};
pub use runtime::{overlay_cfg_from, OverlayCfg, OverlayRuntime};
// Callers should use `overlay::service::run(...)` directly.
// Re-export admin http runner for convenience:
pub use admin_http::run as run_admin_http;
