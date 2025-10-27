//! RO:WHAT — Library entry for svc-overlay
#![forbid(unsafe_code)]

pub mod admin;
pub mod auth;
pub mod bootstrap;
pub mod cli;
pub mod config;
pub mod conn;
pub mod errors;
pub mod gossip;
pub mod limits;
pub mod listener;
pub mod observe;
pub mod pq;
pub mod protocol;
pub mod readiness;
pub mod shutdown;
pub mod supervisor;
pub mod transport;
pub mod tuning;
pub mod types;

use tracing_subscriber::{fmt, EnvFilter};

pub fn init_tracing() {
    let filter = EnvFilter::from_default_env().add_directive(
        "svc_overlay=info"
            .parse()
            .unwrap_or_else(|_| "info".parse().unwrap()),
    );
    let _ = fmt()
        .with_env_filter(filter)
        .json()
        .flatten_event(true)
        .try_init();
}
