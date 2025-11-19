//! RO:WHAT — Binary entrypoint for Macronode.
//! RO:WHY  — Wire config, logging, readiness, admin HTTP plane, and supervisor.
//! RO:INVARIANTS —
//!   - No public Rust API (binary-only crate).
//!   - Admin HTTP is truthful by default; dev overrides are explicit.

#![forbid(unsafe_code)]

mod cli;
mod config;
mod errors;
mod http_admin;
mod observability;
mod readiness;
mod services;
mod supervisor;
mod types;

use crate::errors::Result;

#[tokio::main]
async fn main() -> Result<()> {
    cli::entrypoint().await
}
