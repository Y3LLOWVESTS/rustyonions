#![forbid(unsafe_code)]
//! common: shared types such as `NodeId` and configuration loading.
//!
//! This crate is part of the RustyOnions workspace. It is intentionally kept
//! small and focused. The goal of this pass is **documentation-only refactor**:
//! type/func names are preserved to avoid breaking downstream crates.
//!
//! Conventions used here:
//! - All public types/functions include rustdoc explaining *what* and *why*.
//! - Prefer explicit durations and sizes in docs (units in names where helpful).
//! - Helper functions are `pub(crate)` unless needed externally.
//!
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
/// Stable identity represented as 32 bytes (typically a BLAKE3 hash).
pub struct NodeId([u8; 32]);

impl NodeId {
    /// Construct a `NodeId` by hashing arbitrary bytes with BLAKE3.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut h = Hasher::new();
        h.update(bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(h.finalize().as_bytes());
        Self(out)
    }

    /// Convenience: construct by hashing UTF-8 text.
    /// (Use `FromStr` to **parse** a hex-encoded NodeId.)
    pub fn from_text(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }

    /// Hex string (lowercase) for logging/serialization.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Raw bytes accessor.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NodeId").field(&self.to_hex()).finish()
    }
}

impl FromStr for NodeId {
    type Err = String;

    /// Parse from a 64-char hex string into 32 raw bytes.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = hex::decode(s).map_err(|e| format!("invalid hex for NodeId: {e}"))?;
        if v.len() != 32 {
            return Err(format!("NodeId must be 32 bytes (got {})", v.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&v);
        Ok(NodeId(arr))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Runtime configuration loaded from JSON or TOML.
/// See `Config::load(path)` for details.
pub struct Config {
    /// Where to store chunks/index. May be relative to the working dir.
    pub data_dir: PathBuf,
    /// Listening address for the overlay (line protocol).
    pub overlay_addr: SocketAddr,
    /// Dev TCP inbox for `transport::SmallMsgTransport`.
    pub dev_inbox_addr: SocketAddr,
    /// SOCKS5 for Arti/Tor (e.g., `127.0.0.1:9050`).
    pub socks5_addr: String,
    /// Tor control port (e.g., `127.0.0.1:9051`).
    pub tor_ctrl_addr: String,
    /// Chunk size in bytes for the store.
    pub chunk_size: usize,
    /// Connect timeout for transports.
    pub connect_timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from(".data"),
            overlay_addr: "127.0.0.1:1777".parse().unwrap(),
            dev_inbox_addr: "127.0.0.1:2888".parse().unwrap(),
            socks5_addr: "127.0.0.1:9050".to_string(),
            tor_ctrl_addr: "127.0.0.1:9051".to_string(),
            chunk_size: 1 << 16, // 64 KiB
            connect_timeout_ms: 5000,
        }
    }
}

impl Config {
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.connect_timeout_ms)
    }

    /// Load from JSON or TOML based on filename extension.
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

/// Seconds helper for human-friendly logging/tests.
pub fn secs(d: Duration) -> u64 {
    d.as_secs()
}
