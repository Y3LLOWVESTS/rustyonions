// crates/macronode/src/services/svc_index.rs

//! RO:WHAT — Macronode wiring for svc-index (real HTTP server).
//! RO:WHY  — Stand up svc-index inside Macronode using its own Config/AppState/router.
//! RO:INVARIANTS —
//!   - Uses svc-index crate as the source of truth (Config/AppState/build_router).
//!   - Binds to the same address logic as svc-index/bin (INDEX_BIND or cfg.bind or 127.0.0.1:5304).
//!   - Does *not* yet participate in graceful shutdown; process exit stops the server.
//!   - Readiness for macronode remains governed by ReadyProbes; deps_ok is still set in spawn_all().

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::readiness::ReadyProbes;

// Re-exported API from svc-index crate.
// lib.rs exposes:
//   pub use config::Config;
//   pub use router::build_router;
//   pub use state::AppState;
use svc_index::{
    build_router as build_index_router, AppState as IndexAppState, Config as IndexConfig,
};

/// Spawn the embedded svc-index HTTP server.
///
/// Today we only take `ReadyProbes` so we can hook this into macronode’s
/// readiness story later if we want (e.g., mark deps_ok after warmup).
/// Shutdown is still coarse: when macronode exits, this task ends.
///
/// NOTE: This intentionally mirrors `crates/svc-index/src/main.rs`’s flow:
///   config → state → bootstrap → router.with_state → TcpListener → axum::serve.
pub fn spawn(_probes: Arc<ReadyProbes>) {
    tokio::spawn(async move {
        // 1) Load svc-index config (env + defaults).
        let cfg = match IndexConfig::load() {
            Ok(cfg) => cfg,
            Err(err) => {
                error!(?err, "svc-index (embedded): failed to load config");
                return;
            }
        };

        // 2) Build shared Arc<AppState>.
        let state: Arc<IndexAppState> = match IndexAppState::new(cfg.clone()).await {
            Ok(s) => Arc::new(s),
            Err(err) => {
                error!(?err, "svc-index (embedded): failed to build AppState");
                return;
            }
        };

        // 3) Bootstrap (index warmup / caches / readiness gates).
        let state = IndexAppState::bootstrap(state).await;

        // 4) Build router and inject state at the end (Axum 0.7 pattern).
        let app: Router = build_index_router().with_state(state.clone());

        // 5) Bind address: INDEX_BIND env > cfg.bind > 127.0.0.1:5304.
        let bind_str = std::env::var("INDEX_BIND").unwrap_or_else(|_| cfg.bind.clone());
        let bind: SocketAddr = bind_str
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 5304)));

        let listener: TcpListener = match TcpListener::bind(bind).await {
            Ok(l) => {
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

        // 6) Serve. For now we *don’t* wire macronode’s ShutdownToken in; when
        // the process exits, the server stops. That keeps behavior simple and
        // matches other embedded services at this slice.
        //
        // We use Router -> IntoMakeService pattern to avoid the generic
        // axum::serve<Router<Arc<AppState>>> bound issues.
        let make_svc = app.into_make_service();

        if let Err(err) = axum::serve(listener, make_svc).await {
            error!(?err, "svc-index (embedded): server error");
        } else {
            info!("svc-index (embedded): server exited cleanly");
        }
    });
}
