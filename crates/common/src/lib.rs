use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddr, path::PathBuf, time::Duration};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Overlay (public data plane)
    pub overlay_listen: SocketAddr,
    pub db_path: PathBuf,
    pub chunk_size: usize,
    pub replication: u8,

    // Transport (private small messages plane; dev TCP for now)
    pub inbox_listen: SocketAddr, // for dev transport
    pub peers: Vec<SocketAddr>,

    // Accounting
    pub accounting_window_secs: u64,
    pub contribution_ratio: f32,

    // Tor / Arti settings
    pub tor_enabled: bool,
    pub tor_cache_dir: PathBuf,
    pub tor_hs_dir: PathBuf,
    pub tor_inbox_port: u16,

    // Relay toggle (future)
    pub relay_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Overlay defaults
            overlay_listen: "127.0.0.1:47010".parse().unwrap(),
            db_path: "data/ro_db".into(),
            chunk_size: 256 * 1024,
            replication: 3,

            // Dev transport defaults
            inbox_listen: "127.0.0.1:47110".parse().unwrap(),
            peers: vec![],

            // Accounting defaults
            accounting_window_secs: 24 * 3600,
            contribution_ratio: 2.0,

            // Tor defaults (disabled until Arti is wired)
            tor_enabled: false,
            tor_cache_dir: "data/tor-cache".into(),
            tor_hs_dir: "data/hs".into(),
            tor_inbox_port: 47333,

            // Relay
            relay_enabled: false,
        }
    }
}

pub fn secs(d: Duration) -> u64 { d.as_secs() }
