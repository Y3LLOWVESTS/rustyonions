//! RO:WHAT — Tiny TTL cache for resolves/providers.
//! RO:WHY  — Read-optimized service; avoid hot DHT/DB hits.
//! RO:INVARIANTS — bounded by TTL only (simple MVP).

use dashmap::DashMap;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Truthful result of removing one exact provider-cache entry.
///
/// This operation touches only the provider-response cache. It does not
/// remove resolve-cache entries, manifest pointers, store records, or DHT
/// provider records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderCacheInvalidationOutcome {
    Invalidated,
    NotFound,
}

#[derive(Clone)]
pub struct IndexCache {
    ttl: Duration,
    resolve: Arc<DashMap<String, (crate::types::ResolveResponse, Instant)>>,
    providers: Arc<DashMap<String, (crate::types::ProvidersResponse, Instant)>>,
}

impl IndexCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            resolve: Arc::new(DashMap::new()),
            providers: Arc::new(DashMap::new()),
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

    /// Remove one exact provider-response cache entry.
    ///
    /// The key is matched exactly by `DashMap::remove`. Neighboring CIDs,
    /// resolve-cache entries, manifest pointers, and backing store records
    /// are not inspected or modified.
    pub fn invalidate_providers(&self, cid: &str) -> ProviderCacheInvalidationOutcome {
        if self.providers.remove(cid).is_some() {
            ProviderCacheInvalidationOutcome::Invalidated
        } else {
            ProviderCacheInvalidationOutcome::NotFound
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IndexCache, ProviderCacheInvalidationOutcome};
    use crate::types::{ProvidersResponse, ResolveResponse};

    const CID_A: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    const CID_B: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn providers_response(cid: &str) -> ProvidersResponse {
        ProvidersResponse {
            cid: cid.to_owned(),
            providers: Vec::new(),
            truncated: false,
            etag: None,
        }
    }

    fn resolve_response(key: &str) -> ResolveResponse {
        ResolveResponse {
            key: key.to_owned(),
            manifest: Some(key.to_owned()),
            providers: Vec::new(),
            etag: None,
            cached: false,
        }
    }

    #[test]
    fn invalidation_removes_only_the_exact_provider_cache_entry() {
        let cache = IndexCache::new(60);

        cache.put_providers(CID_A.to_owned(), providers_response(CID_A));
        cache.put_providers(CID_B.to_owned(), providers_response(CID_B));

        assert_eq!(
            cache.invalidate_providers(CID_A),
            ProviderCacheInvalidationOutcome::Invalidated
        );

        assert!(
            cache.get_providers(CID_A).is_none(),
            "target provider cache entry must be gone"
        );

        assert_eq!(
            cache
                .get_providers(CID_B)
                .expect("neighbor provider cache entry must survive")
                .cid,
            CID_B
        );
    }

    #[test]
    fn repeated_invalidation_reports_not_found() {
        let cache = IndexCache::new(60);

        cache.put_providers(CID_A.to_owned(), providers_response(CID_A));

        assert_eq!(
            cache.invalidate_providers(CID_A),
            ProviderCacheInvalidationOutcome::Invalidated
        );

        assert_eq!(
            cache.invalidate_providers(CID_A),
            ProviderCacheInvalidationOutcome::NotFound
        );
    }

    #[test]
    fn provider_invalidation_does_not_remove_resolve_cache_entries() {
        let cache = IndexCache::new(60);

        cache.put_providers(CID_A.to_owned(), providers_response(CID_A));
        cache.put_resolve(CID_A.to_owned(), resolve_response(CID_A));

        assert_eq!(
            cache.invalidate_providers(CID_A),
            ProviderCacheInvalidationOutcome::Invalidated
        );

        assert!(cache.get_providers(CID_A).is_none());

        let resolved = cache
            .get_resolve(CID_A)
            .expect("resolve cache must remain untouched");

        assert_eq!(resolved.key, CID_A);
        assert_eq!(resolved.manifest.as_deref(), Some(CID_A));
    }
}
