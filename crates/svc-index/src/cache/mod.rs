//! RO:WHAT — Tiny TTL cache for resolves/providers.
//! RO:WHY  — Read-optimized service; avoid hot DHT/DB hits.
//! RO:INVARIANTS — bounded by TTL only (simple MVP).

use dashmap::DashMap;
use std::time::{Duration, Instant};

pub struct IndexCache {
    ttl: Duration,
    resolve: DashMap<String, (crate::types::ResolveResponse, Instant)>,
    providers: DashMap<String, (crate::types::ProvidersResponse, Instant)>,
}

impl IndexCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            resolve: DashMap::new(),
            providers: DashMap::new(),
        }
    }

    pub fn get_resolve(&self, key: &str) -> Option<crate::types::ResolveResponse> {
        self.resolve.get(key).and_then(|v| {
            let (val, ins) = v.value();
            if ins.elapsed() <= self.ttl {
                Some(val.clone())
            } else {
                None
            }
        })
    }
    pub fn put_resolve(&self, key: String, val: crate::types::ResolveResponse) {
        self.resolve.insert(key, (val, Instant::now()));
    }

    pub fn get_providers(&self, cid: &str) -> Option<crate::types::ProvidersResponse> {
        self.providers.get(cid).and_then(|v| {
            let (val, ins) = v.value();
            if ins.elapsed() <= self.ttl {
                Some(val.clone())
            } else {
                None
            }
        })
    }
    pub fn put_providers(&self, cid: String, val: crate::types::ProvidersResponse) {
        self.providers.insert(cid, (val, Instant::now()));
    }
}
