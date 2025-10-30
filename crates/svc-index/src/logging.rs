//! RO:WHAT — Tracing subscriber initialization with EnvFilter.
//! RO:WHY  — Observability baseline.
//! RO:CONFIG — RUST_LOG; defaults to info,hyper=warn.

use tracing_subscriber::{fmt, EnvFilter};

pub fn init() {
    let env = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hyper=warn,tower_http=warn"));
    fmt().with_env_filter(env).compact().init();
}
