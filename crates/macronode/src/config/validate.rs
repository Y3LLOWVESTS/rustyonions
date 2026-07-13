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

    if !cfg.admin_ui_bind.ip().is_loopback() {
        return Err(Error::config(
            "admin_ui_bind must remain loopback-only for the CrabLink service-node profile",
        ));
    }

    if !cfg.headless_mode {
        return Err(Error::config(
            "headless_mode must remain true for the CrabLink service-node profile",
        ));
    }

    if cfg.admin_ui_runtime_required {
        return Err(Error::config(
            "admin_ui_runtime_required must be false; service-node runtime cannot depend on the UI",
        ));
    }

    if cfg.operator_ui_profile.trim().is_empty() {
        return Err(Error::config("operator_ui_profile must not be empty"));
    }

    Ok(())
}
