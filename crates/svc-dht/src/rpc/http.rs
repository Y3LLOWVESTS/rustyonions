//! RO:WHAT — Admin endpoints + DHT demo endpoints using canonical crab://node providers.
//! RO:WHY — Ops-first DHT surface; Phase 7 prevents provider records and service identity
//!   DTOs from exposing raw IPs or transport-specific schemes.
//! RO:INTERACTS — metrics, provider::Store, pipeline::lookup, types::{B3Cid, CrabNodeId}.
//! RO:INVARIANTS — deny unknown fields; return 400 on bad input; no lock across .await;
//!   provider identities are crab://node/<64-lowercase-hex>.
//! RO:TEST — tests/api_smoke.rs, tests/crab_node_identity.rs, tests/privacy_no_ip_leak.rs,
//!   tests/node_identity_config.rs, tests/privacy_status.rs.

use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    metrics::DhtMetrics,
    pipeline::lookup::{LookupCtx, LookupRequest},
    provider::{ProviderStoreError, ProviderWithdrawalOutcome, Store},
    readiness::ReadyGate,
    types::{B3Cid, CrabNodeId},
};
use ron_kernel::HealthState;
use serde::Deserialize;

#[derive(Clone)]
pub struct State {
    pub health: Arc<HealthState>,
    pub ready: Arc<ReadyGate>,
    pub metrics: Arc<DhtMetrics>,
    pub providers: Arc<Store>,
    pub node_id: CrabNodeId,

    pub alpha: usize,
    pub beta: usize,
    pub hop_budget: usize,
    pub default_deadline: Duration,
    pub hedge_stagger: Duration,
    pub min_leg_budget: Duration,

    pub lookup_ctx: Arc<LookupCtx>,
}

impl State {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        health: Arc<HealthState>,
        ready: Arc<ReadyGate>,
        metrics: Arc<DhtMetrics>,
        providers: Arc<Store>,
        alpha: usize,
        beta: usize,
        hop_budget: usize,
        default_deadline: Duration,
        hedge_stagger: Duration,
        min_leg_budget: Duration,
        lookup_ctx: Arc<LookupCtx>,
    ) -> Self {
        Self::new_with_node_id(
            CrabNodeId::from_bytes([0x01; 32]),
            health,
            ready,
            metrics,
            providers,
            alpha,
            beta,
            hop_budget,
            default_deadline,
            hedge_stagger,
            min_leg_budget,
            lookup_ctx,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_node_id(
        node_id: CrabNodeId,
        health: Arc<HealthState>,
        ready: Arc<ReadyGate>,
        metrics: Arc<DhtMetrics>,
        providers: Arc<Store>,
        alpha: usize,
        beta: usize,
        hop_budget: usize,
        default_deadline: Duration,
        hedge_stagger: Duration,
        min_leg_budget: Duration,
        lookup_ctx: Arc<LookupCtx>,
    ) -> Self {
        Self {
            health,
            ready,
            metrics,
            providers,
            node_id,
            alpha,
            beta,
            hop_budget,
            default_deadline,
            hedge_stagger,
            min_leg_budget,
            lookup_ctx,
        }
    }
}

/// Build the complete svc-dht HTTP surface around an already-created state.
///
/// This is shared by the standalone svc-dht binary and macronode embedding.
/// Route construction does not create a second provider store or substitute
/// another node identity.
pub fn build_router(state: State) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/version", get(version))
        .route("/dht/node_identity", get(node_identity))
        .route("/dht/privacy_status", get(privacy_status))
        .route("/metrics", get(metrics))
        .route("/dht/find_providers/:cid", get(find_providers))
        .route("/dht/provide", post(provide))
        .route("/dht/withdraw_local_provider", post(withdraw_local_provider))
        .route("/dht/_debug/list", get(debug_list))
        .with_state(state)
}

pub async fn healthz(axum::extract::State(st): axum::extract::State<State>) -> impl IntoResponse {
    if st.health.all_ready() || st.ready.is_ready() {
        (StatusCode::OK, "ok").into_response()
    } else {
        (StatusCode::OK, "starting").into_response()
    }
}

pub async fn readyz(axum::extract::State(st): axum::extract::State<State>) -> impl IntoResponse {
    if st.ready.is_ready() {
        (StatusCode::OK, "ready").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, [("Retry-After", "1")], "booting").into_response()
    }
}

pub async fn version() -> impl IntoResponse {
    let sha = option_env!("BUILD_GIT_SHA").unwrap_or("unknown");
    let ts = option_env!("BUILD_TS").unwrap_or("unknown");
    Json(serde_json::json!({ "git": sha, "built": ts }))
}

pub async fn node_identity(
    axum::extract::State(st): axum::extract::State<State>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "node": st.node_id.to_uri(),
        "node_id_hex": st.node_id.to_node_hex(),
        "identity_scheme": "crab://node",
        "provider_identity_kind": "crab_node",
        "public_inbound": false
    }))
}

pub async fn privacy_status(
    axum::extract::State(st): axum::extract::State<State>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "node": st.node_id.to_uri(),
        "privacy_mode": true,
        "privacy_profile": "service_node_dht_public_boundary",
        "public_inbound_enabled": false,
        "user_ip_publication": "forbidden",
        "residential_ip_publication": "forbidden",
        "lan_ip_publication": "forbidden",
        "admin_bind_publication": false,
        "provider_identity_kind": "crab_node",
        "provider_records_use_raw_socket": false,
        "transport_routes_public": false,
        "raw_socket_publication": false,
        "peer_socket_publication": false,
        "service_routes_public": false,
        "dto_ip_fields_allowed": false,
        "route_identity_scheme": "crab://node"
    }))
}

pub async fn metrics() -> impl IntoResponse {
    match crate::metrics::DhtMetrics::encode() {
        Ok(text) => (StatusCode::OK, text),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "encode error".to_string()),
    }
}

/// Demo: POST /dht/provide
///
/// ```json
/// {"cid":"b3:<64-hex>","node":"crab://node/<64-hex>","ttl_secs":600}
/// ```
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvideBody {
    pub cid: B3Cid,
    pub node: String,
    #[serde(default)]
    pub ttl_secs: Option<u64>,
}

pub async fn provide(
    axum::extract::State(st): axum::extract::State<State>,
    Json(body): Json<ProvideBody>,
) -> impl IntoResponse {
    let ttl = body.ttl_secs.map(Duration::from_secs);
    let used_ttl = ttl.unwrap_or_else(|| st.providers.default_ttl());

    match st.providers.add(body.cid.into_string(), body.node, Some(used_ttl)) {
        Ok(()) => {
            st.metrics.provides_total.inc();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "ttl_secs_used": used_ttl.as_secs()
                })),
            )
                .into_response()
        }
        Err(ProviderStoreError::InvalidNode(reason)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid crab node URI",
                "reason": reason.as_str()
            })),
        )
            .into_response(),
        Err(ProviderStoreError::InvalidCid(reason)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid b3 cid",
                "reason": reason
            })),
        )
            .into_response(),
    }
}

/// POST /dht/withdraw_local_provider
///
/// Removes only the configured local node's provider record for one CID.
/// Callers cannot select or submit a provider identity.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WithdrawLocalProviderBody {
    pub cid: B3Cid,
}

pub async fn withdraw_local_provider(
    axum::extract::State(st): axum::extract::State<State>,
    Json(body): Json<WithdrawLocalProviderBody>,
) -> impl IntoResponse {
    let cid = body.cid.into_string();
    let node = st.node_id;
    let node_uri = node.to_uri();

    match st.providers.withdraw_id(&cid, node) {
        ProviderWithdrawalOutcome::Withdrawn => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "status": "withdrawn",
                "scope": "local_provider_store",
                "cid": cid,
                "node": node_uri
            })),
        )
            .into_response(),
        ProviderWithdrawalOutcome::NotFound => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "ok": false,
                "status": "not_found",
                "scope": "local_provider_store",
                "cid": cid,
                "node": node_uri
            })),
        )
            .into_response(),
    }
}

/// GET /dht/find_providers/:cid — uses the lookup pipeline.
pub async fn find_providers(
    axum::extract::State(st): axum::extract::State<State>,
    Path(cid): Path<B3Cid>,
) -> impl IntoResponse {
    let t0 = Instant::now();

    let req = LookupRequest {
        cid: cid.to_string(),
        alpha: st.alpha,
        beta: st.beta,
        hop_budget: st.hop_budget,
        deadline: st.default_deadline,
        hedge_stagger: st.hedge_stagger,
        min_leg_budget: st.min_leg_budget,
    };

    match st.lookup_ctx.run(req).await {
        Ok(res) => {
            st.metrics.observe_lookup(t0.elapsed(), res.hops);
            Json(serde_json::json!({
                "cid": cid.to_string(),
                "providers": res.providers,
                "hops": res.hops,
                "elapsed_ms": res.elapsed.as_millis(),
            }))
            .into_response()
        }
        Err(e) => {
            st.metrics.observe_lookup(t0.elapsed(), 0);
            (StatusCode::GATEWAY_TIMEOUT, Json(serde_json::json!({ "error": e.to_string() })))
                .into_response()
        }
    }
}

/// Debug: GET /dht/_debug/list — full in-memory snapshot with TTL left.
pub async fn debug_list(
    axum::extract::State(st): axum::extract::State<State>,
) -> impl IntoResponse {
    let snap = st.providers.debug_snapshot();
    Json(serde_json::json!(snap))
}
