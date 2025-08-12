use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId([u8; 32]);

impl NodeId {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut h = Hasher::new();
        h.update(bytes);
        Self(*h.finalize().as_bytes())
    }
    pub fn to_hex(&self) -> String { hex::encode(self.0) }
}
impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.to_hex())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransportChoice { Tcp, Tor }
impl Default for TransportChoice {
    fn default() -> Self { TransportChoice::Tcp }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Accounting / overlay basics
    pub accounting_window_secs: u64,
    pub inbox_listen: SocketAddr,
    pub peers: Vec<String>,
    pub contribution_ratio: f32,

    // Transport selection
    #[serde(default)]
    pub transport: TransportChoice,

    // Tor outbound (SOCKS)
    #[serde(default = "default_tor_socks")]
    pub tor_socks: String,

    // Tor Hidden Service (Control Port)
    #[serde(default = "default_tor_control")]
    pub tor_control: String, // e.g., "127.0.0.1:9051" or "127.0.0.1:9151"
    #[serde(default)]
    pub tor_control_cookie: Option<PathBuf>, // optional override; if None, we probe TOR’s COOKIEFILE via PROTOCOLINFO
    #[serde(default = "default_hs_service_port")]
    pub tor_service_port: u16, // the virtual port on the .onion (e.g., 9001)

    // Legacy placeholders (ignored here)
    #[serde(default)]
    pub tor_enabled: bool,
    #[serde(default = "default_cache_dir")]
    pub tor_cache_dir: PathBuf,
    #[serde(default = "default_hs_dir")]
    pub tor_hs_dir: PathBuf,
    #[serde(default = "default_hs_inbox_port")]
    pub tor_inbox_port: u16,

    #[serde(default)]
    pub relay_enabled: bool,
}

fn default_tor_socks() -> String { "127.0.0.1:9050".to_string() }
fn default_tor_control() -> String { "127.0.0.1:9051".to_string() }
fn default_hs_service_port() -> u16 { 9001 }
fn default_cache_dir() -> PathBuf { PathBuf::from("data/tor-cache") }
fn default_hs_dir() -> PathBuf { PathBuf::from("data/hs") }
fn default_hs_inbox_port() -> u16 { 47333 }

impl Default for Config {
    fn default() -> Self {
        Self {
            accounting_window_secs: 24 * 3600,
            contribution_ratio: 2.0,
            inbox_listen: "127.0.0.1:47110".parse().unwrap(),
            peers: vec![],
            transport: TransportChoice::Tcp,

            tor_socks: default_tor_socks(),
            tor_control: default_tor_control(),
            tor_control_cookie: None,
            tor_service_port: default_hs_service_port(),

            tor_enabled: false,
            tor_cache_dir: default_cache_dir(),
            tor_hs_dir: default_hs_dir(),
            tor_inbox_port: default_hs_inbox_port(),

            relay_enabled: false,
        }
    }
}

impl Config {
    pub fn load_from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
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

pub fn secs(d: Duration) -> u64 { d.as_secs() }
