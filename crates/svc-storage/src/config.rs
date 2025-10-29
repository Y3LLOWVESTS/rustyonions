//! RO:WHAT — Configuration for svc-storage.
//! RO:ENV  —
//!   RON_STORAGE_ADDR        (default "127.0.0.1:5303")
//!   RON_STORAGE_DATA_DIR    (default "./data/storage")
//!   RON_STORAGE_MAX_BODY    (bytes, default 64 MiB)

use anyhow::Context;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

pub struct Config {
    pub http_addr: SocketAddr,
    pub data_dir: PathBuf,
    pub max_body_bytes: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let http_addr_str =
            std::env::var("RON_STORAGE_ADDR").unwrap_or_else(|_| "127.0.0.1:5303".to_string());
        let http_addr = SocketAddr::from_str(&http_addr_str)
            .with_context(|| format!("invalid RON_STORAGE_ADDR: {}", http_addr_str))?;

        let data_dir = std::env::var("RON_STORAGE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/storage"));

        let max_body_bytes = std::env::var("RON_STORAGE_MAX_BODY")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(64 * 1024 * 1024);

        Ok(Self {
            http_addr,
            data_dir,
            max_body_bytes,
        })
    }

    pub fn read_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
    pub fn write_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}
