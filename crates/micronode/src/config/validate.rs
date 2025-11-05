//! RO:WHAT — Config validation.
//! RO:WHY  — Ship fast fail messages before boot.
//! RO:INVARIANTS — Deterministic; no IO.

use super::schema::Config;
use crate::errors::{Error, Result};

pub fn validate(cfg: &Config) -> Result<()> {
    // Minimal baseline; expand with limits/policies later.
    if cfg.server.bind.port() == 0 {
        return Err(Error::Config("bind port must be non-zero".into()));
    }
    Ok(())
}
