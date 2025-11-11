//! Adapter: live-fill miss handler (fetch-on-miss) — stub.

use super::cas::{CasStore, Digest};

/// Miss policy for live fill (placeholder).
#[derive(Debug, Clone, Copy)]
pub enum LiveFillPolicy {
    /// Do not attempt live-fill on misses.
    Off,
    /// Allow live-fill with bounded concurrency (details TBD).
    On,
}

impl Default for LiveFillPolicy {
    fn default() -> Self {
        LiveFillPolicy::Off
    }
}

/// Live-fill engine stub.
#[derive(Debug, Default)]
pub struct LiveFill;

impl LiveFill {
    /// Attempt to fill a digest; always returns `None` for now.
    pub async fn fill<C: CasStore>(&self, _cas: &C, _d: &Digest) -> Option<Vec<u8>> {
        None
    }
}
