//! Map environment variables (`SVC_GATEWAY`_*) onto `Config`.
//!
//! Precedence today (highest → lowest):
//! 1) Explicit env vars (`SVC_GATEWAY`_*)
//! 2) Built-in defaults
//!
//! The optional TOML file layer (`SVC_GATEWAY_CONFIG`) is intentionally
//! omitted in this cut to avoid introducing a new dependency mid-flight.
//! We can add it back once we pin `toml` in `Cargo.toml`.

use super::{Amnesia, Config, Pq, Safety};
use std::net::SocketAddr;

fn get<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok()?.parse().ok()
}

fn truthy(key: &str) -> Option<bool> {
    std::env::var(key).ok().map(|v| {
        let s = v.trim().to_ascii_lowercase();
        matches!(s.as_str(), "1" | "true" | "yes" | "on")
    })
}

/// Overlay explicit environment variables on top of the provided `Config`.
#[must_use]
pub fn apply_env(mut cfg: Config) -> Config {
    // ----- server -----
    if let Ok(s) = std::env::var("BIND_ADDR") {
        if let Ok(addr) = s.parse::<SocketAddr>() {
            cfg.server.bind_addr = addr;
        }
    }
    if let Ok(s) = std::env::var("METRICS_BIND_ADDR") {
        if let Ok(addr) = s.parse::<SocketAddr>() {
            cfg.server.metrics_addr = addr;
        }
    }
    if let Some(v) = get::<usize>("SVC_GATEWAY_MAX_CONNS") {
        cfg.server.max_conns = v;
    }
    if let Some(v) = get::<u64>("SVC_GATEWAY_READ_TIMEOUT_SECS") {
        cfg.server.read_timeout_secs = v;
    }
    if let Some(v) = get::<u64>("SVC_GATEWAY_WRITE_TIMEOUT_SECS") {
        cfg.server.write_timeout_secs = v;
    }
    if let Some(v) = get::<u64>("SVC_GATEWAY_IDLE_TIMEOUT_SECS") {
        cfg.server.idle_timeout_secs = v;
    }

    // ----- caps/limits -----
    if let Some(v) = get::<usize>("SVC_GATEWAY_MAX_BODY_BYTES") {
        cfg.limits.max_body_bytes = v;
    }
    if let Some(v) = get::<usize>("SVC_GATEWAY_DECODE_ABS_CAP_BYTES") {
        cfg.limits.decode_abs_cap_bytes = v;
    }
    if let Some(v) = get::<usize>("SVC_GATEWAY_DECODE_RATIO_MAX") {
        cfg.limits.decode_ratio_max = v;
    }

    // ----- DRR / RL (informational only in this crate) -----
    if let Some(v) = get::<u64>("SVC_GATEWAY_RL_RPS") {
        cfg.drr.rate_limit_rps = v;
    }

    // ----- amnesia / pq / safety / log -----
    if let Some(v) = truthy("SVC_GATEWAY_AMNESIA") {
        cfg.amnesia = Amnesia { enabled: v };
    }
    if let Ok(mode) = std::env::var("SVC_GATEWAY_PQ_MODE") {
        cfg.pq = Pq { mode };
    }
    if let Some(v) = truthy("SVC_GATEWAY_DANGER_OK") {
        cfg.safety = Safety { danger_ok: v };
    }
    if let Ok(level) = std::env::var("SVC_GATEWAY_LOG_LEVEL") {
        cfg.log.level = level;
    }
    if let Ok(fmt) = std::env::var("SVC_GATEWAY_LOG_FORMAT") {
        cfg.log.format = fmt;
    }

    cfg
}

/// Load defaults and overlay env vars.
///
/// # Errors
///
/// This function is infallible today and always returns `Ok`.
/// The `Result` is retained for forward compatibility (e.g., when
/// re-enabling file-based config that can fail to parse).
pub fn load_with_env() -> anyhow::Result<Config> {
    Ok(apply_env(Config::default()))
}
