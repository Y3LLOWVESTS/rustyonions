//! RO:WHAT — svc-dht configuration (binds, α/β, k, seeds, timeouts, amnesia)
//! RO:WHY — Centralized knobs; Concerns: GOV/RES/PERF; hot-reload-friendly shape
//! RO:INTERACTS — bootstrap, peer::table, rpc/http handlers, transport
//! RO:INVARIANTS — values bounded; α ≤ k; β ≤ α; timeouts sane; amnesia honored
//! RO:TEST — config parse unit tests; trybuild for compile-fail when invalid

use serde::{Deserialize, Serialize};
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub admin_bind: SocketAddr,
    pub alpha: usize,
    pub beta: usize,
    pub k: usize,
    pub hop_budget: usize,
    pub dial_timeout_ms: u64,
    pub idle_timeout_ms: u64,
    pub seeds: Vec<String>,
    pub amnesia: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            admin_bind: SocketAddr::from((IpAddr::V4(Ipv4Addr::LOCALHOST), 5301)),
            alpha: 3,
            beta: 1,
            k: 20,
            hop_budget: 6,
            dial_timeout_ms: 1_500,
            idle_timeout_ms: 5_000,
            seeds: vec![],
            amnesia: true,
        }
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let mut cfg = Self::default();
        if let Ok(s) = env::var("DHT_ADMIN_BIND") {
            cfg.admin_bind = s.parse()?;
        }
        if let Ok(v) = env::var("DHT_ALPHA") {
            cfg.alpha = v.parse()?;
        }
        if let Ok(v) = env::var("DHT_BETA") {
            cfg.beta = v.parse()?;
        }
        if let Ok(v) = env::var("DHT_K") {
            cfg.k = v.parse()?;
        }
        if let Ok(v) = env::var("DHT_HOP_BUDGET") {
            cfg.hop_budget = v.parse()?;
        }
        if let Ok(v) = env::var("DHT_DIAL_TIMEOUT_MS") {
            cfg.dial_timeout_ms = v.parse()?;
        }
        if let Ok(v) = env::var("DHT_IDLE_TIMEOUT_MS") {
            cfg.idle_timeout_ms = v.parse()?;
        }
        if let Ok(v) = env::var("DHT_SEEDS") {
            cfg.seeds =
                v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
        if let Ok(v) = env::var("RON_AMNESIA") {
            cfg.amnesia = matches!(v.as_str(), "1") || v.eq_ignore_ascii_case("true");
        }
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        use anyhow::bail;
        if self.alpha == 0 || self.k == 0 {
            bail!("alpha and k must be > 0");
        }
        if self.beta > self.alpha {
            bail!("beta must be <= alpha");
        }
        if self.k < self.alpha {
            bail!("k (bucket size) should be >= alpha");
        }
        if self.hop_budget == 0 {
            bail!("hop budget must be > 0");
        }
        if self.dial_timeout_ms < 100 || self.idle_timeout_ms < 500 {
            bail!("timeouts too small");
        }
        if self.seeds.iter().any(|s| s.len() > 255) {
            bail!("seed too long");
        }
        Ok(())
    }

    pub fn dial_timeout(&self) -> Duration {
        Duration::from_millis(self.dial_timeout_ms)
    }
    pub fn idle_timeout(&self) -> Duration {
        Duration::from_millis(self.idle_timeout_ms)
    }
}
