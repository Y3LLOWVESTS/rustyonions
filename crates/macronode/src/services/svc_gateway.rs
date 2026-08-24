//! RO:WHAT — Compose the canonical full svc-gateway router as CrabNode client ingress.
//! RO:WHY — CN-3 retires the ping-only wrapper and requires the real gateway → Omnigate path.
//! RO:INTERACTS — svc_gateway config/state/routes, Omnigate app health, ReadyProbes, spawn_all.
//! RO:INVARIANTS — no route copies; gateway readiness flips only after a real app-plane roundtrip.
//! RO:METRICS — uses svc-gateway's canonical metrics registration and HTTP middleware.
//! RO:CONFIG — SVC_GATEWAY_* is canonical; RON_GATEWAY_ADDR remains a migration alias.
//! RO:SECURITY — gateway remains proxy-only; no wallet/ledger/Passport authority is introduced.
//! RO:TEST — crabnode_cn3_ingress.rs and svc-gateway app/product proxy tests.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc, time::Duration};

use reqwest::Client;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
use tokio::{net::TcpListener, time::sleep};
use tracing::{error, info};

use crate::{
    errors::{Error, Result},
    readiness::ReadyProbes,
    services::ports,
    supervisor::ManagedTask,
};

const PRODUCT_PATH_PROBE_ATTEMPTS: usize = 50;
const PRODUCT_PATH_PROBE_DELAY: Duration = Duration::from_millis(20);
const PRODUCT_PATH_REQUEST_TIMEOUT: Duration = Duration::from_millis(500);

pub async fn spawn(probes: Arc<ReadyProbes>) -> Result<ManagedTask> {
    probes.set_gateway_bound(false);

    let mut cfg = Config::load().map_err(Error::Other)?;

    apply_crabnode_owned_overrides(&mut cfg)?;

    let bind_addr = cfg.server.bind_addr.parse::<SocketAddr>().map_err(|err| {
        Error::config(format!(
            "invalid canonical svc-gateway bind {}: {err}",
            cfg.server.bind_addr
        ))
    })?;

    let metrics_handles = metrics::unregistered().map_err(|err| {
        Error::Other(anyhow::anyhow!(
            "svc-gateway local metric construction failed: {err}"
        ))
    })?;

    let state = AppState::new(cfg.clone(), metrics_handles);

    let router = routes::build_router(&state);

    let listener = TcpListener::bind(bind_addr).await?;

    let actual_bind = listener.local_addr()?;

    if actual_bind != bind_addr {
        return Err(Error::config(format!(
            "svc-gateway canonical bind mismatch: expected {bind_addr}, got {actual_bind}"
        )));
    }

    let exit_probes = probes.clone();

    let handle = tokio::spawn(async move {
        info!(
            %actual_bind,
            "CrabNode canonical svc-gateway listening"
        );

        let result = axum::serve(listener, router).await;

        exit_probes.set_gateway_bound(false);

        match result {
            Ok(()) => {
                info!("canonical svc-gateway exited cleanly");
            }
            Err(err) => {
                error!(
                    error = %err,
                    "canonical svc-gateway server failed"
                );
            }
        }
    });

    let probe_url = format!("http://{actual_bind}/app/healthz");

    let client = Client::builder()
        .timeout(PRODUCT_PATH_REQUEST_TIMEOUT)
        .build()
        .map_err(|err| {
            handle.abort();

            Error::Other(anyhow::anyhow!(
                "cannot build CN-3 product-path probe client: {err}"
            ))
        })?;

    for _ in 0..PRODUCT_PATH_PROBE_ATTEMPTS {
        if gateway_omnigate_path_ok(&client, &probe_url).await {
            probes.set_gateway_bound(true);

            info!(
                %actual_bind,
                probe = %probe_url,
                "CN-3 gateway → Omnigate product path verified"
            );

            return Ok(ManagedTask::new("svc-gateway", handle));
        }

        if handle.is_finished() {
            break;
        }

        sleep(PRODUCT_PATH_PROBE_DELAY).await;
    }

    handle.abort();
    probes.set_gateway_bound(false);

    Err(Error::Other(anyhow::anyhow!(
        "CN-3 gateway → Omnigate capability probe failed at {probe_url}"
    )))
}

fn apply_crabnode_owned_overrides(cfg: &mut Config) -> Result<()> {
    if std::env::var_os("SVC_GATEWAY_BIND_ADDR").is_none() {
        match std::env::var("RON_GATEWAY_ADDR") {
            Ok(raw) if !raw.trim().is_empty() => {
                let addr = raw.trim().parse::<SocketAddr>().map_err(|err| {
                    Error::config(format!("invalid RON_GATEWAY_ADDR `{raw}`: {err}"))
                })?;

                cfg.server.bind_addr = addr.to_string();
            }
            _ => {
                cfg.server.bind_addr = ports::DEFAULT_GATEWAY_ADDR_STR.to_string();
            }
        }
    }

    if std::env::var_os("SVC_GATEWAY_OMNIGATE_BASE_URL").is_none() {
        let omnigate_addr = match std::env::var("OMNIGATE_BIND") {
            Ok(raw) if !raw.trim().is_empty() => {
                raw.trim().parse::<SocketAddr>().map_err(|err| {
                    Error::config(format!(
                        "invalid OMNIGATE_BIND `{raw}` for gateway upstream: {err}"
                    ))
                })?
            }
            _ => ports::DEFAULT_OMNIGATE_ADDR_STR
                .parse::<SocketAddr>()
                .map_err(|err| {
                    Error::config(format!("invalid canonical Omnigate address: {err}"))
                })?,
        };

        cfg.upstreams.omnigate_base_url = format!("http://{omnigate_addr}");
    }

    if std::env::var_os("SVC_GATEWAY_STORAGE_BASE_URL").is_none() {
        if let Ok(raw) = std::env::var("RON_STORAGE_ADDR") {
            if !raw.trim().is_empty() {
                let storage_addr = raw.trim().parse::<SocketAddr>().map_err(|err| {
                    Error::config(format!(
                        "invalid RON_STORAGE_ADDR `{raw}` for gateway upstream: {err}"
                    ))
                })?;

                cfg.upstreams.storage_base_url = format!("http://{storage_addr}");
            }
        }
    }

    Ok(())
}

async fn gateway_omnigate_path_ok(client: &Client, url: &str) -> bool {
    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(_) => return false,
    };

    if !response.status().is_success() {
        return false;
    }

    let body = match response.text().await {
        Ok(body) => body,
        Err(_) => return false,
    };

    body.contains("\"ok\":true") && body.contains("app plane mounted")
}
