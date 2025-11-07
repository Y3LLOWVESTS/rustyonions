//! RO:WHAT — Init tracing subscriber with env filter.
use tracing_subscriber::{fmt, EnvFilter};

pub fn init() {
    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .try_init();
}
