#![forbid(unsafe_code)]

//! Typed configuration structures and file loading.

use serde::Deserialize;
use std::{fs, path::Path};

/// Optional nested transport section.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct TransportConfig {
    pub max_conns: Option<u64>,
    pub idle_timeout_ms: Option<u64>,
    pub read_timeout_ms: Option<u64>,
    pub write_timeout_ms: Option<u64>,
}

/// Workspace-wide config with typed fields, plus a raw table for ad-hoc lookups.
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub raw: toml::Table,
    pub admin_addr: String,
    pub overlay_addr: String,
    pub dev_inbox_addr: String,
    pub socks5_addr: String,
    pub tor_ctrl_addr: String,
    pub data_dir: String,
    pub chunk_size: u64,
    pub connect_timeout_ms: u64,
    pub transport: TransportConfig,
}

impl Config {
    pub(crate) fn from_table(t: toml::Table) -> Self {
        fn get_string(tbl: &toml::Table, key: &str) -> Option<String> {
            tbl.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
        }
        fn get_u64(tbl: &toml::Table, key: &str) -> Option<u64> {
            tbl.get(key).and_then(|v| v.as_integer()).map(|n| n as u64)
        }

        let admin_addr         = get_string(&t, "admin_addr").unwrap_or_else(|| "127.0.0.1:9096".to_string());
        let overlay_addr       = get_string(&t, "overlay_addr").unwrap_or_else(|| "127.0.0.1:1777".to_string());
        let dev_inbox_addr     = get_string(&t, "dev_inbox_addr").unwrap_or_else(|| "127.0.0.1:2888".to_string());
        let socks5_addr        = get_string(&t, "socks5_addr").unwrap_or_else(|| "127.0.0.1:9050".to_string());
        let tor_ctrl_addr      = get_string(&t, "tor_ctrl_addr").unwrap_or_else(|| "127.0.0.1:9051".to_string());
        let data_dir           = get_string(&t, "data_dir").unwrap_or_else(|| ".data".to_string());
        let chunk_size         = get_u64(&t, "chunk_size").unwrap_or(65536);
        let connect_timeout_ms = get_u64(&t, "connect_timeout_ms").unwrap_or(5000);

        let transport = t.get("transport").and_then(|v| v.clone().try_into().ok()).unwrap_or_default();

        Self {
            raw: t,
            admin_addr,
            overlay_addr,
            dev_inbox_addr,
            socks5_addr,
            tor_ctrl_addr,
            data_dir,
            chunk_size,
            connect_timeout_ms,
            transport,
        }
    }
}

/// Synchronously load and parse a TOML config file.
pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let txt = fs::read_to_string(path)?;
    let table: toml::Table = toml::from_str(&txt)?;
    Ok(Config::from_table(table))
}
