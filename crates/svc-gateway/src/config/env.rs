//! Env-driven config overrides for `svc-gateway`.
//!
//! RO:WHAT — Project selected environment variables onto `Config`.
//! RO:WHY — Keeps `Config::load()` small, explicit, and testable.
//! RO:INTERACTS — `config::Config`, gateway bootstrap, and app/paid proxy routes.
//! RO:INVARIANTS — fail closed on malformed socket/numeric values; do not silently repair bad env.
//! RO:METRICS — none.
//! RO:CONFIG — `SVC_GATEWAY_BIND_ADDR`, `BIND_ADDR`, `SVC_GATEWAY_MAX_BODY_BYTES`,
//!              `SVC_GATEWAY_DECODE_ABS_CAP_BYTES`, `SVC_GATEWAY_OMNIGATE_BASE_URL`,
//!              `SVC_GATEWAY_STORAGE_BASE_URL`.
//! RO:SECURITY — treats upstream base URLs as local operator config; no token material is read here.
//! RO:TEST — `app_proxy.rs`, `paid_storage_estimate_proxy.rs`.

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
/// - `SVC_GATEWAY_STORAGE_BASE_URL`
///
/// # Errors
///
/// Returns an error if any present env value is malformed, such as an invalid
/// socket address or integer.
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

    // Storage service base URL for paid-storage preflight/proxy flows.
    if let Ok(v) = std::env::var("SVC_GATEWAY_STORAGE_BASE_URL") {
        cfg.upstreams.storage_base_url = v;
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
