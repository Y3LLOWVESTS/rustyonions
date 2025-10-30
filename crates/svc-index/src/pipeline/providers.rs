//! RO:WHAT — Providers pipeline: cid -> ranked provider list (MVP).

use crate::{error::SvcError, types::ProvidersResponse, AppState};
use std::sync::Arc;

#[inline]
fn is_b3(s: &str) -> bool {
    let s = s.strip_prefix("b3:").unwrap_or("");
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9'|b'a'..=b'f'))
}

pub async fn run(
    state: Arc<AppState>,
    cid: &str,
    limit: usize,
) -> Result<ProvidersResponse, SvcError> {
    if !is_b3(cid) {
        return Err(SvcError::BadRequest("invalid cid".into()));
    }
    if let Some(cached) = state.cache.get_providers(cid) {
        return Ok(cached);
    }
    let lim = limit.clamp(1, 32);
    let mut providers = state.dht.providers_for(cid, lim).await;
    providers.sort_by(|a, b| b.score.total_cmp(&a.score));
    let resp = ProvidersResponse {
        cid: cid.to_string(),
        providers,
        truncated: false,
        etag: None,
    };
    state.cache.put_providers(cid.to_string(), resp.clone());
    Ok(resp)
}
