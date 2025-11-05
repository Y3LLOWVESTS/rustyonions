//! RO:WHAT — Tracing subscriber initialization.
//! RO:WHY  — Deterministic logs with env filter.

use tracing_subscriber::{fmt, EnvFilter};

pub fn init() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,micronode=debug".to_string());
    let _ = fmt().with_env_filter(EnvFilter::new(filter)).try_init();
}
