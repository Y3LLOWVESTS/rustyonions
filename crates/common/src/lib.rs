#![forbid(unsafe_code)]
//! common: shared types and configuration loading.

pub mod hash;
pub use hash::{b3_hex, b3_hex_file, format_addr, parse_addr, shard2};
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId([u8; 32]);

impl NodeId {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut h = Hasher::new();
        h.update(bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(h.finalize().as_bytes());
        Self(out)
    }
    pub fn from_text(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.to_hex())
    }
}

impl FromStr for NodeId {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        let mut out = [0u8; 32];
        if bytes.len() != 32 {
            anyhow::bail!("expected 32 bytes");
        }
        out.copy_from_slice(&bytes);
        Ok(Self(out))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub data_dir: PathBuf,
    pub overlay_addr: SocketAddr,
    pub dev_inbox_addr: SocketAddr,
    pub socks5_addr: String,
    pub tor_ctrl_addr: String,
    pub chunk_size: usize,
    pub connect_timeout_ms: u64,
    /// Optional persistent HS private key file (used if provided).
    #[serde(default)]
    pub hs_key_file: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from(".data"),
            overlay_addr: "127.0.0.1:1777".parse().unwrap(),
            dev_inbox_addr: "127.0.0.1:2888".parse().unwrap(),
            socks5_addr: "127.0.0.1:9050".to_string(),
            tor_ctrl_addr: "127.0.0.1:9051".to_string(),
            chunk_size: 1 << 16,
            connect_timeout_ms: 5000,
            hs_key_file: None,
        }
    }
}

impl Config {
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.connect_timeout_ms)
    }
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        use anyhow::Context;
        let path = path.as_ref();
        let data = fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let cfg: Config = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::from_str(&data).context("parsing TOML config")?
        } else {
            serde_json::from_str(&data).context("parsing JSON config")?
        };
        Ok(cfg)
    }
}
