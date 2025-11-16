//! RO:WHAT — Micronode library surface (router assembly, config, observability).
#![forbid(unsafe_code)]

pub mod app;
pub mod config;
pub mod errors;
pub mod http;
pub mod layers;
pub mod limits;
pub mod observability;
pub mod state;
pub mod storage;
pub mod types;

pub use app::build_router;
