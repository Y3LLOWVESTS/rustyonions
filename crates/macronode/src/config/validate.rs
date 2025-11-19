//! RO:WHAT — Config validation for Macronode.
//! RO:WHY  — Centralize invariants (ports, timeouts, limits) so we can
//!           evolve them without touching callers.
//! RO:INVARIANTS —
//!   - All durations must be > 0.
//!   - HTTP addr must be a valid SocketAddr (already enforced earlier).

use crate::errors::{Error, Result};

use super::schema::Config;

/// Validate a fully materialized config.
///
/// Returns `Ok(())` if the config is usable, or `Error::Config` with a
/// human-readable message if any invariant is violated.
pub fn validate_config(cfg: &Config) -> Result<()> {
    if cfg.read_timeout.as_millis() == 0 {
        return Err(Error::config("read_timeout must be > 0"));
    }
    if cfg.write_timeout.as_millis() == 0 {
        return Err(Error::config("write_timeout must be > 0"));
    }
    if cfg.idle_timeout.as_millis() == 0 {
        return Err(Error::config("idle_timeout must be > 0"));
    }

    Ok(())
}
