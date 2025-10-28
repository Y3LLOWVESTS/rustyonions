//! RO:WHAT — Admin endpoints + DHT demo endpoints (provide + find_providers via pipeline)
//! RO:WHY — Ops-first; Concerns: GOV/PERF/DX/SEC. Adds CID/node validation and stable errors.
//! RO:INTERACTS — metrics, provider::Store, pipeline::lookup, types::B3Cid.
//! RO:INVARIANTS — deny unknown fields; return 400 on bad input; no lock across .await.
//! RO:TEST — tests/provider_roundtrip.rs

use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    metrics::DhtMetrics,
    pipeline::lookup::{LookupCtx, LookupRequest},
    provider::Store,
    readiness::ReadyGate,
    types::{validate_node_uri, B3Cid},
};
use ron_kernel::HealthState;
use serde::Deserialize;

#[derive(Clone)]
pub struct State {
    pub health: Arc<HealthState>,
    pub ready: Arc<ReadyGate>,
    pub metrics: Arc<DhtMetrics>,
    pub providers: Arc<Store>,

    // Pipeline knobs (from Config)
    pub alpha: usize,
    pub beta: usize,
    pub hop_budget: usize,
    pub default_deadline: Duration,
    pub hedge_stagger: Duration,
    pub min_leg_budget: Duration,

    // Pipeline context (rate limiter etc.)
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
        Self {
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
        }
    }
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

pub async fn metrics() -> impl IntoResponse {
    match crate::metrics::DhtMetrics::encode() {
        Ok(text) => (StatusCode::OK, text),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "encode error".to_string()),
    }
}

/// Demo: POST /dht/provide  {"cid":"b3:...","node":"nodeA","ttl_secs":600}
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
    if !validate_node_uri(&body.node) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "invalid node URI" })))
            .into_response();
    }

    let ttl = body.ttl_secs.map(Duration::from_secs);
    let used_ttl = ttl.unwrap_or_else(|| st.providers.default_ttl());
    st.providers.add(body.cid.into_string(), body.node, Some(used_ttl));
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

/// GET /dht/find_providers/:cid — uses the lookup pipeline (α/β/hedge/deadline)
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

/// Debug: GET /dht/_debug/list — full in-memory snapshot with TTL left
pub async fn debug_list(
    axum::extract::State(st): axum::extract::State<State>,
) -> impl IntoResponse {
    let snap = st.providers.debug_snapshot();
    Json(serde_json::json!(snap))
}
