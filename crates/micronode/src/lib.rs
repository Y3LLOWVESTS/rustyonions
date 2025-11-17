//! RO:WHAT — Micronode library surface (router assembly, config, observability).
//! RO:WHY  — Expose the pieces that tests, benches, and other crates need:
//!           router builder, config, concurrency plane, adapters, and HTTP layers.
//! RO:INTERACTS — Used by integration tests (admin, KV, facets, backpressure, CLI).

#![forbid(unsafe_code)]

pub mod adapters;
pub mod app;
pub mod cli;
pub mod concurrency;
pub mod config;
pub mod errors;
pub mod facets;
pub mod http;
pub mod layers;
pub mod limits;
pub mod observability;
pub mod security;
pub mod state;
pub mod storage;
pub mod types;

pub use app::build_router;
