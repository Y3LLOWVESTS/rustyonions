// Strongly-typed config (stub)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub http_addr: String,
    pub metrics_addr: String,
    pub max_inflight: u32,
}
