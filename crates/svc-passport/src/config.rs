//! RO:WHAT — Service configuration model + loader (env/file), with sane defaults.

use serde::Deserialize;
use std::{env, fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: Server,
    pub passport: Passport,
    pub verify: VerifyPolicy,
    pub cache: Cache,
    pub limits: Limits,
    pub security: Security,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    pub bind: String,
    pub admin_bind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Passport {
    pub issuer: String,
    pub default_ttl_s: u64,
    pub max_ttl_s: u64,
    pub clock_skew_s: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VerifyPolicy {
    pub target_batch: usize,
    pub max_batch: usize,
    pub max_wait_us: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cache {
    pub vk_ttl_s: u64,
    pub jwks_ttl_s: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Limits {
    pub max_msg_bytes: usize,
    pub max_batch: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Security {
    pub require_aud: bool,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        if let Ok(s) = env::var("PASSPORT_CONFIG") {
            return Ok(toml::from_str(&s)?);
        }
        let path = env::var("PASSPORT_CONFIG_FILE")
            .unwrap_or_else(|_| "crates/svc-passport/config/default.toml".to_string());
        let text = fs::read_to_string(Path::new(&path))?;
        Ok(toml::from_str(&text)?)
    }
}
