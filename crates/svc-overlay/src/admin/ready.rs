//! /readyz — truthful readiness gate. JSON schema kept stable.

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct ReadyProbe(Arc<RwLock<ReadyState>>);

#[derive(Default, Clone)]
pub struct ReadyState {
    pub listeners_bound: bool,
    pub metrics_bound: bool,
    pub cfg_loaded: bool,
    pub queues_ok: bool,
    pub shed_rate_ok: bool,
    pub fd_headroom: bool,
    pub pq_ready: Option<bool>,
    pub tor_bootstrap: Option<bool>,
}

impl ReadyProbe {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn set<F: FnOnce(&mut ReadyState)>(&self, f: F) {
        let mut g = self.0.write().await;
        f(&mut g);
    }

    pub async fn snapshot(&self) -> ReadyState {
        self.0.read().await.clone()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadyResp {
    ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    degraded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing: Option<Vec<&'static str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after: Option<u64>,
}

#[inline]
fn need(flag: bool, name: &'static str, out: &mut Vec<&'static str>) {
    if !flag {
        out.push(name);
    }
}

/// Axum handler for `/readyz`.
pub async fn readyz(State(probe): State<ReadyProbe>) -> impl IntoResponse {
    let s = probe.snapshot().await;
    let mut missing = Vec::new();

    need(s.listeners_bound, "listeners_bound", &mut missing);
    need(s.metrics_bound, "metrics_bound", &mut missing);
    need(s.cfg_loaded, "cfg_loaded", &mut missing);
    need(s.queues_ok, "queues_ok", &mut missing);
    need(s.shed_rate_ok, "shed_rate_ok", &mut missing);
    need(s.fd_headroom, "fd_headroom", &mut missing);
    if let Some(false) = s.pq_ready {
        missing.push("pq_ready");
    }
    if let Some(false) = s.tor_bootstrap {
        missing.push("tor_bootstrap");
    }

    if missing.is_empty() {
        (
            axum::http::StatusCode::OK,
            Json(ReadyResp {
                ready: true,
                degraded: None,
                missing: None,
                retry_after: None,
            }),
        )
    } else {
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResp {
                ready: false,
                degraded: Some(true),
                missing: Some(missing),
                retry_after: Some(5),
            }),
        )
    }
}
