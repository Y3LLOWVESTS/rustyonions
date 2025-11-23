//! Env-driven config overrides for svc-gateway.
//!
//! RO:WHAT  Small helper to project selected env vars onto `Config`.
//! RO:WHY   Keeps `Config::load()` simple and testable.

use std::net::SocketAddr;

use anyhow::{Context, Result};

use super::Config;

/// Apply environment variable overrides onto a mutable `Config`.
///
/// Supported keys:
/// - `SVC_GATEWAY_BIND_ADDR` or `BIND_ADDR`
/// - `SVC_GATEWAY_MAX_BODY_BYTES`
/// - `SVC_GATEWAY_DECODE_ABS_CAP_BYTES`
/// - `SVC_GATEWAY_OMNIGATE_BASE_URL`
///
/// # Errors
///
/// Returns an error if any present env value is malformed (e.g., invalid
/// socket address or integer).
pub fn apply_env_overrides(cfg: &mut Config) -> Result<()> {
    // Bind addr: prefer svc-specific, fall back to legacy BIND_ADDR.
    if let Ok(addr) = std::env::var("SVC_GATEWAY_BIND_ADDR").or_else(|_| std::env::var("BIND_ADDR"))
    {
        validate_bind_addr(&addr).context("invalid SVC_GATEWAY_BIND_ADDR/BIND_ADDR")?;
        cfg.server.bind_addr = addr;
    }

    // Max body bytes (request cap).
    if let Ok(v) = std::env::var("SVC_GATEWAY_MAX_BODY_BYTES") {
        let n: u64 = v
            .parse()
            .context("invalid SVC_GATEWAY_MAX_BODY_BYTES (expected u64)")?;
        cfg.limits.max_body_bytes = n;
    }

    // Decode absolute cap bytes.
    if let Ok(v) = std::env::var("SVC_GATEWAY_DECODE_ABS_CAP_BYTES") {
        let n: u64 = v
            .parse()
            .context("invalid SVC_GATEWAY_DECODE_ABS_CAP_BYTES (expected u64)")?;
        cfg.limits.decode_abs_cap_bytes = n;
    }

    // Omnigate app-plane base URL.
    if let Ok(v) = std::env::var("SVC_GATEWAY_OMNIGATE_BASE_URL") {
        cfg.upstreams.omnigate_base_url = v;
    }

    Ok(())
}

/// Parse and validate a `SocketAddr` for the bind address.
///
/// # Errors
///
/// Returns an error if the string is not a valid socket address.
fn validate_bind_addr(s: &str) -> Result<SocketAddr> {
    s.parse::<SocketAddr>()
        .with_context(|| format!("invalid bind addr {s}"))
}
