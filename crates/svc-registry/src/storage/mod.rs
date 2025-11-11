//! RO:WHAT — Storage facade for svc-registry (trait + in-memory backend).
//! RO:WHY  — Abstracts the read/write plane and the SSE event source.
//! RO:INTERACTS — http::{routes,sse}, observability::metrics, readiness gate.
//! RO:INVARIANTS — Monotonic head.version; subscribe is non-blocking.

pub mod inmem;

use chrono::{DateTime, Utc};
use tokio_stream::wrappers::BroadcastStream;

/// Public head shape returned to clients.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Head {
    pub version: u64,
    pub payload_b3: String,
    pub committed_at: Option<DateTime<Utc>>,
}

/// Internal SSE events exposed by the store.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegistryEvent {
    // Sent on successful commit with the resulting Head.
    Commit { head: Head },
}

#[async_trait::async_trait]
pub trait RegistryStore: Send + Sync {
    /// RO:WHAT — Fast read of the current head.
    async fn head(&self) -> Head;

    /// RO:WHAT — Commit a new payload; monotonically bumps version and returns new head.
    /// RO:INVARIANTS — version strictly increases; committed_at set to now.
    async fn commit(&self, payload_b3: String) -> anyhow::Result<Head>;

    /// RO:WHAT — Subscribe to store events (commit stream).
    /// RO:WHY  — Feeds SSE; non-blocking broadcast with bounded buffers.
    fn subscribe(&self) -> BroadcastStream<RegistryEvent>;
}
