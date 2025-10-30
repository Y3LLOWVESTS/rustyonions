//! RO:WHAT — Resolve pipeline: key (name|b3) -> manifest + providers (MVP).
//! RO:WHY  — Encapsulate read-optimized logic with cache & store.

use crate::{
    error::SvcError,
    types::{ProviderEntry, ResolveResponse},
    AppState,
};
use std::sync::Arc;

#[inline]
fn is_b3(s: &str) -> bool {
    let s = s.strip_prefix("b3:").unwrap_or("");
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9'|b'a'..=b'f'))
}

pub async fn run(
    state: Arc<AppState>,
    key: &str,
    fresh: bool,
) -> Result<ResolveResponse, SvcError> {
    if !is_b3(key) && !key.starts_with("name:") {
        return Err(SvcError::BadRequest("invalid key".into()));
    }
    if !fresh {
        if let Some(cached) = state.cache.get_resolve(key) {
            return Ok(cached);
        }
    }

    // Manifest lookup (store is authority for names; b3 maps to itself in MVP)
    let manifest = if is_b3(key) {
        Some(key.to_string())
    } else {
        state.store.get_manifest(key)
    };

    // Provider set (stubbed to DHT client)
    let providers = if let Some(cid) = manifest.as_ref() {
        state.dht.providers_for(cid, 5).await
    } else {
        Vec::<ProviderEntry>::new()
    };

    let resp = ResolveResponse {
        key: key.to_string(),
        manifest,
        providers,
        etag: None,
        cached: false,
    };
    state.cache.put_resolve(key.to_string(), resp.clone());
    Ok(resp)
}
