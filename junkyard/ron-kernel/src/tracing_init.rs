use tracing_subscriber::{fmt, EnvFilter};

/// Initialize compact tracing with an env-driven filter.
/// Example: RUST_LOG=info,ron_kernel=debug,actor_spike=debug
pub fn tracing_init(default_filter: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_filter));
    fmt().with_env_filter(filter).compact().init();
}
