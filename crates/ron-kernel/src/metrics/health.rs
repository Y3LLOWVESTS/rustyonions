use std::collections::BTreeMap;
use std::sync::Arc;

use axum::{response::IntoResponse, Json};
use parking_lot::RwLock;

use crate::internal::types::ServiceName;

#[derive(Clone)]
pub struct HealthState {
    inner: Arc<RwLock<BTreeMap<ServiceName, bool>>>,
}

impl HealthState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    pub fn set(&self, service: ServiceName, ok: bool) {
        let mut w = self.inner.write();
        w.insert(service, ok);
    }

    pub fn snapshot(&self) -> BTreeMap<ServiceName, bool> {
        self.inner.read().clone()
    }

    pub fn all_ready(&self) -> bool {
        // Not ready until at least one service has reported AND all are healthy.
        let r = self.inner.read();
        !r.is_empty() && r.values().all(|v| *v)
    }
}

pub async fn healthz_handler(state: HealthState) -> impl IntoResponse {
    if state.all_ready() {
        let body = serde_json::to_value(state.snapshot()).unwrap();
        (axum::http::StatusCode::OK, Json(body))
    } else {
        let body = state
            .snapshot()
            .into_iter()
            .filter(|(_, ok)| !*ok)
            .map(|(k, _)| k)
            .collect::<Vec<_>>();
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "unhealthy": body })),
        )
    }
}

impl Default for HealthState {
    fn default() -> Self {
        Self::new()
    }
}
