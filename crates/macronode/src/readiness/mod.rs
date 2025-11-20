//! RO:WHAT — Readiness probes and `/readyz` handler for Macronode.
//! RO:WHY  — Truthful readiness for orchestration (K8s/systemd/CI).
//! RO:INVARIANTS —
//!   - Required gates: listeners_bound && cfg_loaded && gateway_bound && deps_ok.
//!   - Optional probes: metrics_bound.
//!   - Dev override: MACRONODE_DEV_READY=1 forces ready=true.

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
    gateway_bound: AtomicBool,
}

impl ReadyProbes {
    pub fn new() -> Self {
        Self {
            listeners_bound: AtomicBool::new(false),
            cfg_loaded: AtomicBool::new(false),
            metrics_bound: AtomicBool::new(false),
            deps_ok: AtomicBool::new(false),
            gateway_bound: AtomicBool::new(false),
        }
    }

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

    pub fn set_gateway_bound(&self, v: bool) {
        self.gateway_bound.store(v, Ordering::Release);
    }

    pub fn snapshot(&self) -> ReadySnapshot {
        ReadySnapshot {
            listeners_bound: self.listeners_bound.load(Ordering::Acquire),
            cfg_loaded: self.cfg_loaded.load(Ordering::Acquire),
            metrics_bound: self.metrics_bound.load(Ordering::Acquire),
            deps_ok: self.deps_ok.load(Ordering::Acquire),
            gateway_bound: self.gateway_bound.load(Ordering::Acquire),
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
    pub gateway_bound: bool,
}

impl ReadySnapshot {
    pub fn required_ready(&self) -> bool {
        self.listeners_bound && self.cfg_loaded && self.deps_ok && self.gateway_bound
    }
}

#[derive(Serialize)]
struct ReadyDeps<'a> {
    config: &'a str,
    network: &'a str,
    gateway: &'a str,
    storage: &'a str,
}

#[derive(Serialize)]
struct ReadyBody<'a> {
    ready: bool,
    deps: ReadyDeps<'a>,
    mode: &'a str,
}

pub async fn handler(probes: Arc<ReadyProbes>) -> impl IntoResponse {
    // dev override
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
            gateway: if snap.gateway_bound { "ok" } else { "pending" },
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

    let deps = ReadyDeps {
        config: if snap.cfg_loaded { "loaded" } else { "pending" },
        network: if snap.listeners_bound {
            "ok"
        } else {
            "pending"
        },
        gateway: if snap.gateway_bound { "ok" } else { "pending" },
        storage: if snap.deps_ok { "ok" } else { "pending" },
    };

    let mut headers = HeaderMap::new();
    if !ok {
        headers.insert("Retry-After", HeaderValue::from_static("5"));
    }

    let body = ReadyBody {
        ready: ok,
        deps,
        mode: "truthful",
    };

    let status = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, headers, Json(body)).into_response()
}
