// crates/macronode/src/services/svc_index.rs

//! RO:WHAT — Macronode wiring for svc-index (embedded Axum HTTP server).
//! RO:WHY  — Stand up svc-index inside macronode using svc-index as the source of truth (DX/GOV).
//! RO:INTERACTS — svc_index::{Config,AppState,build_router}, crate::readiness::ReadyProbes, crate::supervisor::ManagedTask
//! RO:INVARIANTS — no locks held across .await; readiness flips only after listener bind; default bind must be stable and non-colliding
//! RO:METRICS/LOGS — logs bind/version; flips readiness probe index_bound
//! RO:CONFIG — INDEX_BIND env > cfg.bind > ports::DEFAULT_INDEX_BIND_STR
//! RO:SECURITY — local-only default bind; no secrets handled here
//! RO:TEST — exercised by dev stack boot; svc-index /readyz,/metrics exposed on its own bind (embedded)

#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use crate::{readiness::ReadyProbes, services::ports, supervisor::ManagedTask};

use svc_index::{
    build_router as build_index_router, AppState as IndexAppState, Config as IndexConfig,
};

fn default_index_bind() -> SocketAddr {
    ports::default_index_addr()
}

fn resolve_bind(cfg: &IndexConfig) -> SocketAddr {
    let env = std::env::var("INDEX_BIND")
        .ok()
        .map(|s| s.trim().to_string());
    let cfg_bind = cfg.bind.trim().to_string();

    let chosen = env
        .filter(|s| !s.is_empty())
        .or_else(|| (!cfg_bind.is_empty()).then_some(cfg_bind))
        .unwrap_or_else(|| ports::DEFAULT_INDEX_BIND_STR.to_string());

    match chosen.parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(err) => {
            let fallback = default_index_bind();
            warn!(
                raw = %chosen,
                ?err,
                %fallback,
                "svc-index (embedded): invalid bind string; falling back to default"
            );
            fallback
        }
    }
}

pub fn spawn(probes: Arc<ReadyProbes>) -> ManagedTask {
    let handle = tokio::spawn(async move {
        let cfg = match IndexConfig::load() {
            Ok(cfg) => cfg,
            Err(err) => {
                error!(?err, "svc-index (embedded): failed to load config");
                return;
            }
        };

        let state: Arc<IndexAppState> = match IndexAppState::new(cfg.clone()).await {
            Ok(s) => Arc::new(s),
            Err(err) => {
                error!(?err, "svc-index (embedded): failed to build AppState");
                return;
            }
        };

        let state = IndexAppState::bootstrap(state).await;

        let app: Router = build_index_router().with_state(state.clone());

        let bind: SocketAddr = resolve_bind(&cfg);

        let listener: TcpListener = match TcpListener::bind(bind).await {
            Ok(l) => {
                probes.set_index_bound(true);
                info!(
                    version = env!("CARGO_PKG_VERSION"),
                    %bind,
                    "svc-index (embedded) starting"
                );
                l
            }
            Err(err) => {
                error!(?err, %bind, "svc-index (embedded): failed to bind");
                return;
            }
        };

        let make_svc = app.into_make_service();

        if let Err(err) = axum::serve(listener, make_svc).await {
            error!(?err, "svc-index (embedded): server error");
        } else {
            info!("svc-index (embedded): server exited cleanly");
        }
    });

    ManagedTask::new("svc-index", handle)
}
