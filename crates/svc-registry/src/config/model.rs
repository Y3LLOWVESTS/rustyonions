//! Config model and defaults. Env prefix SVCR_.
use serde::{Deserialize, Serialize};

/// Service configuration (beta scope).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // Existing flat fields (kept for compatibility with current bootstrap)
    pub bind_addr: String,
    pub metrics_addr: String,
    pub max_conns: u32,
    pub read_timeout: String,
    pub write_timeout: String,
    pub idle_timeout: String,
    pub storage: Storage,

    // New structured sections (used by HTTP layers, SSE, and CORS).
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub timeouts: Timeouts,
    #[serde(default)]
    pub sse: Sse,
    #[serde(default)]
    pub cors: Cors,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Storage {
    pub kind: String, // "sled" | "sqlite" (foundation uses stub)
    pub data_dir: String,
    pub fsync: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Limits {
    /// Max inbound request body size (bytes)
    pub max_request_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_request_bytes: 64 * 1024,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Timeouts {
    /// Per-request timeout in milliseconds (applied at HTTP layer).
    pub request_ms: u64,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self { request_ms: 5_000 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Sse {
    /// Heartbeat interval in milliseconds.
    pub heartbeat_ms: u64,
    /// Max clients (informational; enforcement may be best-effort).
    pub max_clients: usize,
    /// Informational drop policy label (e.g., "lag-drop").
    pub drop_policy: String,
}

impl Default for Sse {
    fn default() -> Self {
        Self {
            heartbeat_ms: 5_000,
            max_clients: 8_192,
            drop_policy: "lag-drop".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Cors {
    /// Exact origins like "http://localhost:5173". Use "*" for permissive dev.
    pub allowed_origins: Vec<String>,
}

impl Default for Cors {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:9444".into(),
            metrics_addr: "127.0.0.1:9909".into(),
            max_conns: 1024,
            read_timeout: "5s".into(),
            write_timeout: "5s".into(),
            idle_timeout: "60s".into(),
            storage: Storage {
                kind: "sled".into(),
                data_dir: "./target/dev-registry".into(),
                fsync: true,
            },
            limits: Limits::default(),
            timeouts: Timeouts::default(),
            sse: Sse::default(),
            cors: Cors::default(),
        }
    }
}
