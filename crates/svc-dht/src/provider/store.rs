//! RO:WHAT — In-memory provider store with TTL
//! RO:WHY — Micronode default; keeps MVP simple
use super::record::ProviderRecord;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Store {
    inner: RwLock<HashMap<String, Vec<ProviderRecord>>>, // cid -> records
    default_ttl: Duration,
}

impl Store {
    pub fn new(default_ttl: Duration) -> Self {
        Self { inner: RwLock::new(HashMap::new()), default_ttl }
    }

    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }

    /// RO:WHAT — Add/refresh a provider record (de-duped by node) for a CID.
    pub fn add(&self, cid: String, node: String, ttl: Option<Duration>) {
        let cid = normalize(&cid);
        let node = normalize(&node);
        let ttl = ttl.unwrap_or(self.default_ttl);
        let rec = ProviderRecord::new(cid.clone(), node, ttl);

        let mut g = self.inner.write();
        let v = g.entry(cid).or_default();
        // de-dup by node
        if let Some(pos) = v.iter().position(|r| r.node == rec.node) {
            v[pos] = rec;
        } else {
            v.push(rec);
        }
    }

    /// RO:WHAT — Read-only view of live providers (no mutation).
    pub fn get_live(&self, cid: &str) -> Vec<String> {
        let cid = normalize(cid);
        let now = Instant::now();
        let g = self.inner.read();
        if let Some(v) = g.get(&cid) {
            v.iter().filter(|r| !r.expired(now)).map(|r| r.node.clone()).collect()
        } else {
            Vec::new()
        }
    }

    /// RO:WHAT — Prune expired records; called by background pruner.
    pub fn purge_expired(&self) -> usize {
        let now = Instant::now();
        let mut g = self.inner.write();
        let mut purged = 0usize;
        for v in g.values_mut() {
            let before = v.len();
            v.retain(|r| !r.expired(now));
            purged += before.saturating_sub(v.len());
        }
        // drop empty CIDs
        g.retain(|_, v| !v.is_empty());
        purged
    }

    /// RO:WHAT — Debug snapshot: all CIDs with nodes and seconds-until-expiry.
    pub fn debug_snapshot(&self) -> Vec<DebugCid> {
        let now = Instant::now();
        let g = self.inner.read();
        let mut out = Vec::new();
        for (cid, recs) in g.iter() {
            let entries = recs
                .iter()
                .map(|r| DebugEntry {
                    node: r.node.clone(),
                    secs_left: r.expires_at.saturating_duration_since(now).as_secs_f64(),
                })
                .collect();
            out.push(DebugCid { cid: cid.clone(), entries });
        }
        out
    }
}

#[derive(serde::Serialize)]
pub struct DebugCid {
    pub cid: String,
    pub entries: Vec<DebugEntry>,
}

#[derive(serde::Serialize)]
pub struct DebugEntry {
    pub node: String,
    pub secs_left: f64,
}

fn normalize(s: &str) -> String {
    s.trim().to_string()
}
