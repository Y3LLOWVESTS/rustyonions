//! RO:WHAT   Downstream (egress) HTTP client stack for calling RON services.
//! RO:WHY    Centralize retries, timeouts, and error taxonomy; keep wrappers thin.
//! RO:INTERACTS reqwest (rustls), tokio, crate::observability (corr-id later).
//! RO:INVARS  Finite timeouts; 4xx never retried; retries use jitter; no panics.

mod retry;
mod error;

pub mod latency;
pub mod hedge;

pub mod index_client;
pub mod storage_client;
pub mod mailbox_client;
pub mod dht_client;

pub use error::DsError;
pub use retry::{RetryPolicy, full_jitter_backoff};

use std::time::Duration;

/// RO:WHAT Build a default reqwest client suitable for internal calls.
/// RO:WHY  Ensure consistent TLS & connection settings.
pub fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("reqwest client")
}
