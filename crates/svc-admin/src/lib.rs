// crates/svc-admin/src/lib.rs

pub mod auth;
pub mod cli;
pub mod config;
pub mod dto;
pub mod error;
pub mod metrics;
pub mod nodes;
pub mod observability;
pub mod router;
pub mod server;
pub mod state;
pub mod interop;

pub use crate::config::Config;
pub use crate::error::Error;
