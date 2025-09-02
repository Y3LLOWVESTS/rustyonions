#![forbid(unsafe_code)]

use ron_app_sdk::DEFAULT_MAX_FRAME;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub addr: SocketAddr,       // OAP listener
    pub http_addr: SocketAddr,  // admin /readyz
    pub max_frame: usize,
    pub max_inflight: u64,
    pub chunk_bytes: usize,
    pub tiles_root: String,
    pub max_file_bytes: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:9443".parse().unwrap(),
            http_addr: "127.0.0.1:9096".parse().unwrap(),
            max_frame: DEFAULT_MAX_FRAME,
            max_inflight: 64,
            chunk_bytes: 64 * 1024,
            tiles_root: "./testing/tiles".to_string(),
            max_file_bytes: 128 * 1024 * 1024,
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut c = Self::default();
        if let Ok(s) = std::env::var("OVERLAY_ADDR") {
            c.addr = s.parse().expect("OVERLAY_ADDR must be host:port");
        }
        if let Ok(s) = std::env::var("ADMIN_ADDR") {
            c.http_addr = s.parse().expect("ADMIN_ADDR must be host:port");
        }
        if let Ok(s) = std::env::var("MAX_INFLIGHT") {
            c.max_inflight = s.parse().expect("MAX_INFLIGHT must be integer");
        }
        if let Ok(s) = std::env::var("CHUNK_BYTES") {
            c.chunk_bytes = s.parse().expect("CHUNK_BYTES must be integer");
        }
        if let Ok(s) = std::env::var("TILES_ROOT") {
            c.tiles_root = s;
        }
        if let Ok(s) = std::env::var("MAX_FILE_BYTES") {
            c.max_file_bytes = s.parse().expect("MAX_FILE_BYTES must be integer");
        }
        c
    }
}
