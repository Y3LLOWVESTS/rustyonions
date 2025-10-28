//! RO:WHAT — ProviderRecord v1 (MVP: node string + expiry)
//! RO:WHY — Minimal schema to exercise provide/find locally

use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct ProviderRecord {
    pub cid: String,
    pub node: String,
    pub expires_at: Instant,
}

impl ProviderRecord {
    pub fn new(cid: String, node: String, ttl: Duration) -> Self {
        Self { cid, node, expires_at: Instant::now() + ttl }
    }
    pub fn expired(&self, now: Instant) -> bool {
        now >= self.expires_at
    }
}
