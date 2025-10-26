//! RO:WHAT
//!   Admin plane: router(), ReadyProbe gates, /healthz, /readyz, /version, /metrics.
//! RO:WHY
//!   Matches bootstrap’s expected API (router(probe, ver), ReadyProbe::set(..)).
//! RO:INVARIANTS
//!   - Truthful readiness: 200 only when all gates are satisfied.
//!   - Cheap atomics; /readyz lists missing gates when not ready.

use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub mod metrics;
pub use metrics::handle_metrics;

pub mod version;
pub use version::{current_build_info, BuildInfo};

/// Readiness gates container (Arc so we can clone into tasks).
#[derive(Clone)]
pub struct ReadyProbe(Arc<ReadyInner>);

struct ReadyInner {
    listeners_bound: AtomicBool,
    metrics_bound: AtomicBool,
    cfg_loaded: AtomicBool,
    queues_ok: AtomicBool,
    shed_rate_ok: AtomicBool,
    fd_headroom: AtomicBool,
}

/// Mutable view used by `ReadyProbe::set` to emulate your original closure API.
#[derive(Default)]
pub struct ReadyState {
    pub listeners_bound: bool,
    pub metrics_bound: bool,
    pub cfg_loaded: bool,
    pub queues_ok: bool,
    pub shed_rate_ok: bool,
    pub fd_headroom: bool,
}

impl Default for ReadyProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadyProbe {
    pub fn new() -> Self {
        Self(Arc::new(ReadyInner {
            listeners_bound: AtomicBool::new(false),
            metrics_bound: AtomicBool::new(false),
            cfg_loaded: AtomicBool::new(false),
            queues_ok: AtomicBool::new(true),
            shed_rate_ok: AtomicBool::new(true),
            fd_headroom: AtomicBool::new(true),
        }))
    }

    #[inline]
    pub fn set_listeners_bound(&self, v: bool) {
        self.0.listeners_bound.store(v, Ordering::Relaxed);
    }
    #[inline]
    pub fn set_metrics_bound(&self, v: bool) {
        self.0.metrics_bound.store(v, Ordering::Relaxed);
    }
    #[inline]
    pub fn set_cfg_loaded(&self, v: bool) {
        self.0.cfg_loaded.store(v, Ordering::Relaxed);
    }
    #[inline]
    pub fn set_queues_ok(&self, v: bool) {
        self.0.queues_ok.store(v, Ordering::Relaxed);
    }
    #[inline]
    pub fn set_shed_rate_ok(&self, v: bool) {
        self.0.shed_rate_ok.store(v, Ordering::Relaxed);
    }
    #[inline]
    pub fn set_fd_headroom(&self, v: bool) {
        self.0.fd_headroom.store(v, Ordering::Relaxed);
    }

    /// API compatibility shim for prior `probe.set(|s| s.<gate> = ..).await` usage.
    /// This is `async` to match call sites; it performs stores immediately.
    pub async fn set<F>(&self, f: F)
    where
        F: FnOnce(&mut ReadyState),
    {
        // snapshot
        let mut st = ReadyState {
            listeners_bound: self.0.listeners_bound.load(Ordering::Relaxed),
            metrics_bound: self.0.metrics_bound.load(Ordering::Relaxed),
            cfg_loaded: self.0.cfg_loaded.load(Ordering::Relaxed),
            queues_ok: self.0.queues_ok.load(Ordering::Relaxed),
            shed_rate_ok: self.0.shed_rate_ok.load(Ordering::Relaxed),
            fd_headroom: self.0.fd_headroom.load(Ordering::Relaxed),
        };
        // mutate
        f(&mut st);
        // store
        self.0
            .listeners_bound
            .store(st.listeners_bound, Ordering::Relaxed);
        self.0
            .metrics_bound
            .store(st.metrics_bound, Ordering::Relaxed);
        self.0.cfg_loaded.store(st.cfg_loaded, Ordering::Relaxed);
        self.0.queues_ok.store(st.queues_ok, Ordering::Relaxed);
        self.0
            .shed_rate_ok
            .store(st.shed_rate_ok, Ordering::Relaxed);
        self.0.fd_headroom.store(st.fd_headroom, Ordering::Relaxed);
    }

    fn snapshot(&self) -> Gates {
        Gates {
            listeners_bound: self.0.listeners_bound.load(Ordering::Relaxed),
            metrics_bound: self.0.metrics_bound.load(Ordering::Relaxed),
            cfg_loaded: self.0.cfg_loaded.load(Ordering::Relaxed),
            queues_ok: self.0.queues_ok.load(Ordering::Relaxed),
            shed_rate_ok: self.0.shed_rate_ok.load(Ordering::Relaxed),
            fd_headroom: self.0.fd_headroom.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone)]
pub struct AdminState {
    pub probe: ReadyProbe,
    pub build: BuildInfo,
}

#[derive(Serialize)]
struct Health {
    alive: bool,
}

async fn handle_healthz() -> Json<Health> {
    Json(Health { alive: true })
}

#[derive(Serialize)]
struct ReadyBody<'a> {
    ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing: Option<&'a [&'a str]>,
}

#[derive(Default)]
struct Gates {
    listeners_bound: bool,
    metrics_bound: bool,
    cfg_loaded: bool,
    queues_ok: bool,
    shed_rate_ok: bool,
    fd_headroom: bool,
}

impl Gates {
    fn all_ready(&self) -> bool {
        self.listeners_bound
            && self.metrics_bound
            && self.cfg_loaded
            && self.queues_ok
            && self.shed_rate_ok
            && self.fd_headroom
    }
    fn missing(&self) -> Vec<&'static str> {
        let mut v = Vec::with_capacity(6);
        if !self.listeners_bound {
            v.push("listeners_bound");
        }
        if !self.metrics_bound {
            v.push("metrics_bound");
        }
        if !self.cfg_loaded {
            v.push("cfg_loaded");
        }
        if !self.queues_ok {
            v.push("queues_ok");
        }
        if !self.shed_rate_ok {
            v.push("shed_rate_ok");
        }
        if !self.fd_headroom {
            v.push("fd_headroom");
        }
        v
    }
}

async fn handle_readyz(State(state): State<AdminState>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let snap = state.probe.snapshot();
    if snap.all_ready() {
        (
            StatusCode::OK,
            Json(ReadyBody {
                ready: true,
                missing: None,
            }),
        )
            .into_response()
    } else {
        let miss = snap.missing();
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyBody {
                ready: false,
                missing: Some(&miss),
            }),
        )
            .into_response()
    }
}

/// Build the admin router with state and handlers (what bootstrap expects).
pub fn router(probe: ReadyProbe, build: BuildInfo) -> Router {
    let state = AdminState { probe, build };
    Router::new()
        .route("/healthz", get(handle_healthz))
        .route("/readyz", get(handle_readyz))
        .route("/version", get(version::handle_version))
        .route("/metrics", get(metrics::handle_metrics))
        .with_state(state)
}

/// Helper to run the admin plane on an address.
pub async fn serve_admin(
    bind: SocketAddr,
    probe: ReadyProbe,
    build: BuildInfo,
) -> anyhow::Result<()> {
    let app = router(probe, build);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(addr=?bind, "admin server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
