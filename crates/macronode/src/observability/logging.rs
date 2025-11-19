//! RO:WHAT — Tracing subscriber initialization for Macronode.
//! RO:WHY  — Deterministic logs with env filter and JSON-friendly format.

use tracing_subscriber::{fmt, EnvFilter};

pub fn init(log_level: &str) {
    let default = format!("macronode={log_level},info");
    let filter = std::env::var("RUST_LOG").unwrap_or(default);

    let _ = fmt().with_env_filter(EnvFilter::new(filter)).try_init();
}
