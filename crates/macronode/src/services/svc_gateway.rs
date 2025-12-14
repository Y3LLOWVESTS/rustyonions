//! RO:WHAT — Macronode HTTP ingress (svc-gateway MVP).
//! RO:WHY  — Stand up actual ingress listener + mark readiness correctly.
//! RO:INVARIANTS —
//!   - Binds to 127.0.0.1:8090 by default (override via RON_GATEWAY_ADDR).
//!   - Sets `gateway_bound=true` on successful bind to feed `/readyz` + status.
//!   - No locks held across `.await`.

use std::sync::Arc;
use std::{net::SocketAddr, str::FromStr};

use axum::{response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::observability::metrics::observe_facet_ok;
use crate::readiness::ReadyProbes;
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
    //
    // Appears in macronode's /metrics as:
    //   ron_facet_requests_total{facet="gateway.app",result="ok"} N
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
    const DEFAULT_ADDR: &str = "127.0.0.1:8090";

    if let Ok(raw) = std::env::var("RON_GATEWAY_ADDR") {
        match SocketAddr::from_str(raw.trim()) {
            Ok(addr) => {
                info!("svc-gateway: using RON_GATEWAY_ADDR={addr}");
                return addr;
            }
            Err(err) => {
                error!(
                    "svc-gateway: invalid RON_GATEWAY_ADDR={raw:?}, \
                     falling back to {DEFAULT_ADDR}: {err}"
                );
            }
        }
    }

    SocketAddr::from_str(DEFAULT_ADDR).expect("DEFAULT_ADDR must be a valid SocketAddr")
}

/// Spawn the gateway HTTP ingress server.
///
/// Returns a `ManagedTask` wrapping the JoinHandle so the supervisor can
/// log when this service exits. Behavior is otherwise identical to the
/// previous fire-and-forget slice.
pub fn spawn(probes: Arc<ReadyProbes>) -> ManagedTask {
    let handle = tokio::spawn(async move {
        let addr = resolve_bind_addr();

        let listener = match TcpListener::bind(addr).await {
            Ok(listener) => {
                info!("svc-gateway: listening on {addr}");
                probes.set_gateway_bound(true);
                listener
            }
            Err(err) => {
                error!("svc-gateway: failed to bind to {addr}: {err}");
                return;
            }
        };

        let app = Router::new().route("/ingress/ping", get(ping_handler));

        if let Err(err) = axum::serve(listener, app).await {
            error!("svc-gateway: server error: {err}");
        } else {
            info!("svc-gateway: server exited cleanly");
        }
    });

    ManagedTask::new("svc-gateway", handle)
}
