//! RO:WHAT — Macronode HTTP ingress (svc-gateway MVP).
//! RO:WHY  — Stand up actual ingress listener + mark readiness correctly.

use std::{net::SocketAddr, str::FromStr};

use axum::{response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::readiness::ReadyProbes;
use std::sync::Arc;

#[derive(Debug, Serialize)]
struct PingBody {
    ok: bool,
    service: &'static str,
    profile: &'static str,
}

async fn ping_handler() -> impl IntoResponse {
    Json(PingBody {
        ok: true,
        service: "svc-gateway",
        profile: "macronode",
    })
}

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

pub fn spawn(probes: Arc<ReadyProbes>) {
    tokio::spawn(async move {
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
}
