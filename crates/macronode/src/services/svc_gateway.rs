// crates/macronode/src/services/svc_gateway.rs

//! RO:WHAT — Macronode HTTP ingress (svc-gateway MVP).
//! RO:WHY  — Stand up actual ingress listener + mark readiness correctly.
//! RO:INVARIANTS —
//!   - Binds to 127.0.0.1:8090 by default (override via RON_GATEWAY_ADDR).
//!   - Sets `gateway_bound=true` on successful bind to feed `/readyz` + status.
//!   - No locks held across `.await`.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc};

use axum::{response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::observability::metrics::observe_facet_ok;
use crate::readiness::ReadyProbes;
use crate::services::ports;
use crate::supervisor::ManagedTask;

#[derive(Debug, Serialize)]
struct PingBody {
    ok: bool,
    service: &'static str,
    profile: &'static str,
}

async fn ping_handler() -> impl IntoResponse {
    // Count this as one successful gateway app request so svc-admin can
    // surface `gateway.app` in the facet metrics panel.
    observe_facet_ok("gateway.app");

    Json(PingBody {
        ok: true,
        service: "svc-gateway",
        profile: "macronode",
    })
}

/// Resolve the bind address for the gateway plane.
///
/// Env override:
///   - `RON_GATEWAY_ADDR=IP:PORT`
fn resolve_bind_addr() -> SocketAddr {
    if let Ok(raw) = std::env::var("RON_GATEWAY_ADDR") {
        match raw.trim().parse::<SocketAddr>() {
            Ok(addr) => {
                info!("svc-gateway: using RON_GATEWAY_ADDR={addr}");
                return addr;
            }
            Err(err) => {
                error!(
                    "svc-gateway: invalid RON_GATEWAY_ADDR={raw:?}, falling back to {}: {err}",
                    ports::DEFAULT_GATEWAY_ADDR_STR
                );
            }
        }
    }

    ports::default_gateway_addr()
}

/// Spawn the gateway HTTP ingress server.
pub fn spawn(probes: Arc<ReadyProbes>) -> ManagedTask {
    let handle = tokio::spawn(async move {
        let addr = resolve_bind_addr();

        let listener = match TcpListener::bind(addr).await {
            Ok(listener) => {
                probes.set_gateway_bound(true);
                info!("svc-gateway: listening on {addr}");
                listener
            }
            Err(err) => {
                error!("svc-gateway: failed to bind to {addr}: {err}");
                return;
            }
        };

        let app = Router::new().route("/ingress/ping", get(ping_handler));
        let make_svc = app.into_make_service();

        if let Err(err) = axum::serve(listener, make_svc).await {
            error!("svc-gateway: server error: {err}");
        } else {
            info!("svc-gateway: server exited cleanly");
        }
    });

    ManagedTask::new("svc-gateway", handle)
}
