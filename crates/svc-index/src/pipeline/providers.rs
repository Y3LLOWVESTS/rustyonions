//! RO:WHAT — Providers pipeline: cid -> ranked provider list (MVP).
//! RO:WHY  Validate CID, pull from DHT, filter synthetic stubs, rank, clamp, cache.
//! RO:INVARIANTS Do not synthesize providers; never cache stub entries.

use crate::{error::SvcError, types::ProvidersResponse, AppState};
use std::sync::Arc;

#[inline]
fn is_b3(s: &str) -> bool {
    let s = s.strip_prefix("b3:").unwrap_or("");
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

pub async fn run(
    state: Arc<AppState>,
    cid: &str,
    limit: usize,
) -> Result<ProvidersResponse, SvcError> {
    // 1) Validate input (malformed -> 400)
    if !is_b3(cid) {
        return Err(SvcError::BadRequest("invalid cid".into()));
    }

    // 2) Cache fast-path (already a cleaned object)
    if let Some(cached) = state.cache.get_providers(cid) {
        return Ok(cached);
    }

    // 3) Query DHT (upper-bound to avoid large allocations)
    let lim = limit.clamp(1, 32);
    let mut providers = state.dht.providers_for(cid, lim).await;

    // 4) Remove any synthetic stub providers
    providers.retain(|p| p.id != "local://stub");

    // 5) Rank descending by score
    providers.sort_by(|a, b| b.score.total_cmp(&a.score));

    // 6) Truncate after filtering; mark truncated truthfully
    let truncated = providers.len() > lim;
    if truncated {
        providers.truncate(lim);
    }

    // 7) Build response (no synthesis)
    let resp = ProvidersResponse {
        cid: cid.to_string(),
        providers,
        truncated,
        etag: None,
    };

    // 8) Cache cleaned response
    state.cache.put_providers(cid.to_string(), resp.clone());

    Ok(resp)
}
