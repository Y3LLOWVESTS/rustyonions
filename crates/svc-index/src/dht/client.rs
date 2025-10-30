//! RO:WHAT — DHT client stub for provider lookups (to be wired to svc-dht).

use crate::types::ProviderEntry;

#[derive(Clone, Default)]
pub struct DhtClient;

impl DhtClient {
    pub fn new() -> Self {
        Self
    }
    pub async fn providers_for(&self, _cid: &str, limit: usize) -> Vec<ProviderEntry> {
        vec![ProviderEntry {
            id: "local://stub".into(),
            region: Some("local".into()),
            score: 0.5,
        }]
        .into_iter()
        .take(limit)
        .collect()
    }
}
