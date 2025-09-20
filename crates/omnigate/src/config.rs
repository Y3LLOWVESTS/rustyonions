// crates/svc-omnigate/src/config.rs
#![forbid(unsafe_code)]

use ron_app_sdk::DEFAULT_MAX_FRAME;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub addr: SocketAddr,      // OAP listener
    pub http_addr: SocketAddr, // admin /readyz
    pub max_frame: usize,
    pub max_inflight: u64,
    pub chunk_bytes: usize,
    pub tiles_root: String,
    pub max_file_bytes: u64,
    // Quotas (per-tenant, per-proto)
    pub quota_tile_rps: u32,
    pub quota_mailbox_rps: u32,
}

impl Default for Config {
    fn default() -> Self {
        // Avoid unwraps on literal parses; fall back to localhost if parsing ever fails.
        let addr = "127.0.0.1:9443"
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 9443)));
        let http_addr = "127.0.0.1:9096"
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 9096)));

        Self {
            addr,
            http_addr,
            max_frame: DEFAULT_MAX_FRAME,
            max_inflight: 128,
            chunk_bytes: 64 * 1024,
            tiles_root: "testing/tiles".to_string(),
            max_file_bytes: 8 * 1024 * 1024,
            quota_tile_rps: 50,
            quota_mailbox_rps: 100,
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut c = Self::default();

        if let Ok(s) = std::env::var("ADDR") {
            if let Ok(addr) = s.parse::<SocketAddr>() {
                c.addr = addr;
            } else {
                tracing::warn!("ADDR must be host:port (got {s})");
            }
        }
        if let Ok(s) = std::env::var("ADMIN_ADDR") {
            if let Ok(addr) = s.parse::<SocketAddr>() {
                c.http_addr = addr;
            } else {
                tracing::warn!("ADMIN_ADDR must be host:port (got {s})");
            }
        }
        if let Ok(s) = std::env::var("MAX_FRAME") {
            if let Ok(v) = s.parse::<usize>() {
                c.max_frame = v;
            } else {
                tracing::warn!("MAX_FRAME must be integer (got {s})");
            }
        }
        if let Ok(s) = std::env::var("MAX_INFLIGHT") {
            if let Ok(v) = s.parse::<u64>() {
                c.max_inflight = v;
            } else {
                tracing::warn!("MAX_INFLIGHT must be integer (got {s})");
            }
        }
        if let Ok(s) = std::env::var("CHUNK_BYTES") {
            if let Ok(v) = s.parse::<usize>() {
                c.chunk_bytes = v;
            } else {
                tracing::warn!("CHUNK_BYTES must be integer (got {s})");
            }
        }
        if let Ok(s) = std::env::var("TILES_ROOT") {
            c.tiles_root = s;
        }
        if let Ok(s) = std::env::var("MAX_FILE_BYTES") {
            if let Ok(v) = s.parse::<u64>() {
                c.max_file_bytes = v;
            } else {
                tracing::warn!("MAX_FILE_BYTES must be integer (got {s})");
            }
        }
        if let Ok(s) = std::env::var("QUOTA_TILE_RPS") {
            if let Ok(v) = s.parse::<u32>() {
                c.quota_tile_rps = v;
            } else {
                tracing::warn!("QUOTA_TILE_RPS must be integer (got {s})");
            }
        }
        if let Ok(s) = std::env::var("QUOTA_MAILBOX_RPS") {
            if let Ok(v) = s.parse::<u32>() {
                c.quota_mailbox_rps = v;
            } else {
                tracing::warn!("QUOTA_MAILBOX_RPS must be integer (got {s})");
            }
        }
        c
    }
}
