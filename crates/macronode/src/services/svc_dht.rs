//! RO:WHAT — Embed the canonical svc-dht HTTP/provider runtime in macronode.
//!
//! RO:WHY — Macronode pruning must withdraw its real local provider record;
//! a host-shell worker cannot truthfully provide that behavior.
//!
//! RO:INTERACTS — svc_dht::{Config, rpc::http, ProviderStore, bootstrap},
//! ReadyProbes, ShutdownToken, and the macronode service supervisor.
//!
//! RO:INVARIANTS —
//!   - DHT semantics and routes remain owned by svc-dht.
//!   - The configured typed CrabNodeId is used by local withdrawal.
//!   - `dht_bound` becomes true only after a real listener bind.
//!   - No caller can select another node for local-provider withdrawal.
//!   - Provider records remain amnesia-friendly in-memory state.
//!   - No wallet, ledger, reward, or settlement mutation occurs here.
//!
//! RO:CONFIG —
//!   - `RON_DHT_ADDR` overrides the svc-dht configured admin bind.
//!   - Remaining DHT settings come from `svc_dht::Config::from_env()`.
//!
//! RO:TEST — embedded_runtime_router_withdraws_configured_node.

#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc, time::Duration};

use ron_kernel::HealthState;
use svc_dht::{
    bootstrap,
    config::Config as DhtConfig,
    metrics::DhtMetrics,
    pipeline::lookup::LookupCtx,
    provider::ttl::spawn_pruner,
    readiness::ReadyGate,
    rpc::http::{build_router, State as DhtHttpState},
    ProviderStore,
};
use tokio::{net::TcpListener, time::sleep};
use tracing::{error, info};

use crate::{
    readiness::ReadyProbes,
    services::ports,
    supervisor::{ManagedTask, ShutdownToken},
    types::RuntimeStatus,
};

struct EmbeddedDhtRuntime {
    state: DhtHttpState,
    health: Arc<HealthState>,
    ready: Arc<ReadyGate>,
    metrics: Arc<DhtMetrics>,
    providers: Arc<ProviderStore>,
}

/// Choose between an explicitly configured svc-dht bind and macronode's
/// collision-free DHT default.
fn configured_or_macronode_default(
    configured: SocketAddr,
    dht_admin_bind_explicit: bool,
) -> SocketAddr {
    if dht_admin_bind_explicit {
        configured
    } else {
        ports::default_dht_addr()
    }
}

/// Resolve the embedded DHT listener.
///
/// Precedence:
///
/// 1. `RON_DHT_ADDR`
/// 2. Explicit `DHT_ADMIN_BIND` loaded by svc-dht
/// 3. Macronode's canonical DHT default (`127.0.0.1:5302`)
fn resolve_bind_addr(configured: SocketAddr) -> SocketAddr {
    let fallback =
        configured_or_macronode_default(configured, std::env::var_os("DHT_ADMIN_BIND").is_some());

    let Ok(raw) = std::env::var("RON_DHT_ADDR") else {
        return fallback;
    };

    match raw.trim().parse::<SocketAddr>() {
        Ok(addr) => {
            info!("svc-dht embedded: using RON_DHT_ADDR={addr}");
            addr
        }
        Err(err) => {
            error!(
                raw = ?raw,
                fallback = %fallback,
                %err,
                "svc-dht embedded: invalid RON_DHT_ADDR; using fallback bind"
            );
            fallback
        }
    }
}

fn build_runtime(cfg: &DhtConfig) -> anyhow::Result<EmbeddedDhtRuntime> {
    let health = Arc::new(HealthState::default());
    let ready = Arc::new(ReadyGate::new());
    let metrics = Arc::new(DhtMetrics::new()?);
    let providers = Arc::new(ProviderStore::new(Duration::from_secs(600)));
    let lookup_ctx = Arc::new(LookupCtx::new(providers.clone(), 64));

    let state = DhtHttpState::new_with_node_id(
        cfg.node_id,
        health.clone(),
        ready.clone(),
        metrics.clone(),
        providers.clone(),
        cfg.alpha,
        cfg.beta,
        cfg.hop_budget,
        Duration::from_millis(300),
        Duration::from_millis(25),
        Duration::from_millis(50),
        lookup_ctx,
    );

    Ok(EmbeddedDhtRuntime {
        state,
        health,
        ready,
        metrics,
        providers,
    })
}

/// Spawn the real embedded svc-dht runtime.
pub fn spawn(
    probes: Arc<ReadyProbes>,
    shutdown: ShutdownToken,
    runtime: Arc<RuntimeStatus>,
) -> ManagedTask {
    let handle =
        tokio::spawn(async move {
            let mut cfg = match DhtConfig::from_env() {
                Ok(cfg) => cfg,
                Err(err) => {
                    error!(%err, "svc-dht embedded: configuration rejected");
                    probes.set_dht_bound(false);
                    return;
                }
            };

            cfg.admin_bind = resolve_bind_addr(cfg.admin_bind);

            let embedded_runtime = match build_runtime(&cfg) {
                Ok(embedded_runtime) => embedded_runtime,
                Err(err) => {
                    error!(%err, "svc-dht embedded: runtime construction failed");
                    probes.set_dht_bound(false);
                    return;
                }
            };

            let listener = match TcpListener::bind(cfg.admin_bind).await {
                Ok(listener) => listener,
                Err(err) => {
                    error!(
                        bind = %cfg.admin_bind,
                        %err,
                        "svc-dht embedded: listener bind failed"
                    );
                    probes.set_dht_bound(false);
                    return;
                }
            };

            let bound_addr = match listener.local_addr() {
                Ok(addr) => addr,
                Err(err) => {
                    error!(%err, "svc-dht embedded: could not read bound address");
                    probes.set_dht_bound(false);
                    return;
                }
            };

            let EmbeddedDhtRuntime {
                state,
                health,
                ready,
                metrics,
                providers,
            } = embedded_runtime;

            let bootstrap =
                match bootstrap::spawn_bootstrap_supervisor(cfg.clone(), health, ready, metrics)
                    .await
                {
                    Ok(supervisor) => supervisor,
                    Err(err) => {
                        error!(%err, "svc-dht embedded: bootstrap supervisor failed");
                        probes.set_dht_bound(false);
                        return;
                    }
                };

            // Register the exact provider store and configured local node
            // used by the embedded router.
            runtime.register_prune_provider_store(providers.clone(), cfg.node_id);

            let provider_pruner = spawn_pruner(providers);
            let app = build_router(state);

            probes.set_dht_bound(true);

            info!(
                %bound_addr,
                node = %cfg.node_uri(),
                "svc-dht embedded: listening with canonical provider router"
            );

            let shutdown_wait = async move {
                while !shutdown.is_triggered() {
                    sleep(Duration::from_millis(50)).await;
                }
            };

            let serve_result = axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_wait)
                .await;

            provider_pruner.abort();
            bootstrap.shutdown().await;
            runtime.clear_prune_provider_store();
            probes.set_dht_bound(false);

            match serve_result {
                Ok(()) => info!("svc-dht embedded: server exited cleanly"),
                Err(err) => error!(%err, "svc-dht embedded: server error"),
            }
        });

    ManagedTask::new("svc-dht", handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use svc_dht::types::CrabNodeId;
    use tower::ServiceExt;

    const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    const NODE_A_URI: &str =
        "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";

    const NODE_B_URI: &str =
        "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";

    #[test]
    fn embedded_bind_fallback_preserves_macronode_port_map() {
        let svc_dht_default: SocketAddr = "127.0.0.1:5301".parse().expect("svc-dht test address");
        let explicit: SocketAddr = "127.0.0.1:5399".parse().expect("explicit test address");

        assert_eq!(
            configured_or_macronode_default(svc_dht_default, false),
            ports::default_dht_addr(),
            "embedded DHT must use macronode's collision-free default"
        );

        assert_eq!(
            configured_or_macronode_default(explicit, true),
            explicit,
            "an explicit DHT_ADMIN_BIND must remain authoritative"
        );
    }

    #[tokio::test]
    async fn embedded_runtime_router_withdraws_configured_node() {
        let cfg = DhtConfig {
            node_id: CrabNodeId::from_uri(NODE_A_URI)
                .expect("configured test node must be canonical"),
            ..DhtConfig::default()
        };

        let runtime = build_runtime(&cfg).expect("embedded DHT runtime");

        runtime
            .providers
            .add(
                CID.to_owned(),
                NODE_A_URI.to_owned(),
                Some(Duration::from_secs(60)),
            )
            .expect("configured provider record");

        runtime
            .providers
            .add(
                CID.to_owned(),
                NODE_B_URI.to_owned(),
                Some(Duration::from_secs(60)),
            )
            .expect("neighbor provider record");

        let providers = runtime.providers.clone();
        let app = build_router(runtime.state);

        let request = Request::builder()
            .method("POST")
            .uri("/dht/withdraw_local_provider")
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"cid":"{CID}"}}"#)))
            .expect("withdrawal request");

        let response = app.oneshot(request).await.expect("router response");

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("response body");

        let value: serde_json::Value = serde_json::from_slice(&body).expect("response JSON");

        assert_eq!(value["status"], "withdrawn");
        assert_eq!(value["scope"], "local_provider_store");
        assert_eq!(value["node"], NODE_A_URI);

        assert_eq!(
            providers.get_live(CID),
            vec![NODE_B_URI.to_owned()],
            "embedded runtime must preserve the neighboring provider"
        );
    }
}
