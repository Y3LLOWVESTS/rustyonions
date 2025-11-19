//! RO:WHAT — Readiness probes and `/readyz` handler for Macronode.
//! RO:WHY  — Truthful readiness for orchestration (K8s, systemd, CI).
//! RO:INVARIANTS —
//!   - Required gates: listeners_bound && cfg_loaded.
//!   - Optional probes: metrics_bound, deps_ok (storage/index/etc).
//!   - Dev override env: `MACRONODE_DEV_READY=1` forces 200 for local benches.

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug)]
pub struct ReadyProbes {
    listeners_bound: AtomicBool,
    cfg_loaded: AtomicBool,
    metrics_bound: AtomicBool,
    deps_ok: AtomicBool,
}

impl ReadyProbes {
    /// Construct probes with a conservative, truthful baseline.
    pub fn new() -> Self {
        Self {
            listeners_bound: AtomicBool::new(false),
            cfg_loaded: AtomicBool::new(false),
            metrics_bound: AtomicBool::new(false),
            // For now deps_ok defaults to true (no external storage/overlay yet).
            deps_ok: AtomicBool::new(true),
        }
    }

    // --- Setters ---

    pub fn set_listeners_bound(&self, v: bool) {
        self.listeners_bound.store(v, Ordering::Release);
    }

    pub fn set_cfg_loaded(&self, v: bool) {
        self.cfg_loaded.store(v, Ordering::Release);
    }

    pub fn set_metrics_bound(&self, v: bool) {
        self.metrics_bound.store(v, Ordering::Release);
    }

    pub fn set_deps_ok(&self, v: bool) {
        self.deps_ok.store(v, Ordering::Release);
    }

    // --- Snapshot ---

    pub fn snapshot(&self) -> ReadySnapshot {
        ReadySnapshot {
            listeners_bound: self.listeners_bound.load(Ordering::Acquire),
            cfg_loaded: self.cfg_loaded.load(Ordering::Acquire),
            metrics_bound: self.metrics_bound.load(Ordering::Acquire),
            deps_ok: self.deps_ok.load(Ordering::Acquire),
        }
    }
}

impl Default for ReadyProbes {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadySnapshot {
    pub listeners_bound: bool,
    pub cfg_loaded: bool,
    pub metrics_bound: bool,
    pub deps_ok: bool,
}

impl ReadySnapshot {
    /// REQUIRED gates for 200 OK; adjust here if we tighten semantics.
    pub fn required_ready(&self) -> bool {
        self.listeners_bound && self.cfg_loaded
    }
}

#[derive(Serialize)]
struct ReadyDeps<'a> {
    config: &'a str,
    network: &'a str,
    storage: &'a str,
}

#[derive(Serialize)]
struct ReadyBody<'a> {
    ready: bool,
    deps: ReadyDeps<'a>,
    mode: &'a str,
}

/// Axum-compatible handler for `/readyz`.
pub async fn handler(probes: Arc<ReadyProbes>) -> impl IntoResponse {
    // Dev override for local smokes/benches.
    if matches!(
        std::env::var("MACRONODE_DEV_READY").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("on") | Ok("ON")
    ) {
        let snap = probes.snapshot();
        let deps = ReadyDeps {
            config: if snap.cfg_loaded { "loaded" } else { "pending" },
            network: if snap.listeners_bound {
                "ok"
            } else {
                "pending"
            },
            storage: if snap.deps_ok { "ok" } else { "pending" },
        };
        let body = ReadyBody {
            ready: true,
            deps,
            mode: "dev-forced",
        };
        return (StatusCode::OK, Json(body)).into_response();
    }

    let snap = probes.snapshot();
    let ok = snap.required_ready();
    let status = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let deps = ReadyDeps {
        config: if snap.cfg_loaded { "loaded" } else { "pending" },
        network: if snap.listeners_bound {
            "ok"
        } else {
            "pending"
        },
        storage: if snap.deps_ok { "ok" } else { "pending" },
    };

    let mut headers = HeaderMap::new();
    if !ok {
        // Simple Retry-After; we can tune later or make configurable.
        headers.insert("Retry-After", HeaderValue::from_static("5"));
    }

    let body = ReadyBody {
        ready: ok,
        deps,
        mode: "truthful",
    };

    (status, headers, Json(body)).into_response()
}
