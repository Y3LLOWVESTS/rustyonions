#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Minimal HELLO payload (JSON).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hello {
    pub server_version: String,
    pub max_frame: u64,
    pub max_inflight: u64,
    pub supported_flags: Vec<String>,
    pub oap_versions: Vec<u8>,
    pub transports: Vec<String>,
}
