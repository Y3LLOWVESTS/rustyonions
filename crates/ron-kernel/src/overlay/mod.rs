#![forbid(unsafe_code)]

//! Overlay service modules and re-exports.

pub mod tls;
pub mod metrics;
pub mod runtime;
pub mod service;
pub mod admin_http;

// Re-exports for ergonomic imports
pub use metrics::{OverlayMetrics, init_overlay_metrics};
pub use runtime::{OverlayCfg, OverlayRuntime, overlay_cfg_from};
// Callers should use `overlay::service::run(...)` directly.
// Re-export admin http runner for convenience:
pub use admin_http::run as run_admin_http;
