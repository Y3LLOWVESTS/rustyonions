//! RO:WHAT — Config validation for Macronode.
//! RO:WHY  — Centralize invariants (ports, timeouts, limits) so we can
//!           evolve them without touching callers.
//! RO:INVARIANTS —
//!   - All durations must be > 0.
//!   - Setup-token TTL is bounded from 1 second through 1 hour.
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

    if cfg.admin_setup_token_ttl.as_secs() == 0 {
        return Err(Error::config(
            "admin_setup_token_ttl must be at least 1 second",
        ));
    }

    if cfg.admin_setup_token_ttl > std::time::Duration::from_secs(60 * 60) {
        return Err(Error::config(
            "admin_setup_token_ttl must not exceed 1 hour",
        ));
    }

    if cfg.operator_ui_profile.trim().is_empty() {
        return Err(Error::config("operator_ui_profile must not be empty"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn setup_token_ttl_accepts_short_lived_values() {
        let cfg = Config {
            admin_setup_token_ttl: Duration::from_secs(1),
            ..Config::default()
        };

        validate_config(&cfg).expect("one-second Phase 23 TTL must validate");
    }

    #[test]
    fn setup_token_ttl_rejects_subsecond_values() {
        let cfg = Config {
            admin_setup_token_ttl: Duration::from_millis(500),
            ..Config::default()
        };

        let error = validate_config(&cfg)
            .expect_err("subsecond TTL would truncate to immediate expiry and must reject");

        assert!(
            error.to_string().contains("at least 1 second"),
            "unexpected validation error: {error}",
        );
    }

    #[test]
    fn setup_token_ttl_rejects_effectively_permanent_values() {
        let cfg = Config {
            admin_setup_token_ttl: Duration::from_secs(60 * 60 + 1),
            ..Config::default()
        };

        let error = validate_config(&cfg)
            .expect_err("setup credentials must not remain active over one hour");

        assert!(
            error.to_string().contains("must not exceed 1 hour"),
            "unexpected validation error: {error}",
        );
    }
}
