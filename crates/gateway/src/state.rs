// crates/gateway/src/state.rs
#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::pay_enforce::Enforcer;

/// Shared application state for the gateway.
#[derive(Debug, Clone)]
pub struct AppState {
    pub index_db: PathBuf,
    pub enforcer: Enforcer,
}

impl AppState {
    pub fn new(index_db: PathBuf, enforce_payments: bool) -> Self {
        Self {
            index_db,
            enforcer: Enforcer::new(enforce_payments),
        }
    }
}
