#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::await_holding_lock)]
#![allow(missing_docs)]

//! svc-registry — authoritative, append-only registry service (foundation build)

pub mod build_info;
pub mod config;
pub mod eligibility;
pub mod enforcement;
pub mod error;
pub mod governance;
pub mod http;
pub mod observability;
pub mod rewards;
pub mod shutdown;
pub mod storage; // <-- new

pub use build_info::BuildInfo;
pub use config::model::Config;
pub use error::Error;
pub use observability::metrics::RegistryMetrics as Metrics;
