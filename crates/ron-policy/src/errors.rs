//! RO:WHAT — Error taxonomy for ron-policy.
//!
//! RO:WHY  — Stable, deterministic error envelope for services/tests.
//!
//! RO:INTERACTS — `parse::{json,toml,validate}`, `engine::eval`
//!
//! RO:INVARIANTS — human-safe messages; no leaking secrets
//!
//! RO:TEST — unit tests exercise all variants

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("evaluation error: {0}")]
    Eval(String),
}
