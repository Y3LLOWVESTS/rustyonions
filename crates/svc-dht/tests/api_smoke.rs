//! Happy-path handler smoke test (no sockets).
//! Verifies: provide → find_providers JSON shape + 400 on bad crab node identity.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::extract::State as AxumState;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use ron_kernel::HealthState;
use svc_dht::metrics::DhtMetrics;
use svc_dht::pipeline::lookup::LookupCtx;
use svc_dht::provider::Store;
use svc_dht::readiness::ReadyGate;
use svc_dht::rpc::http::{find_providers, provide, ProvideBody, State};
use svc_dht::types::B3Cid;

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";

static METRICS: OnceLock<Arc<DhtMetrics>> = OnceLock::new();

fn metrics() -> Arc<DhtMetrics> {
    METRICS.get_or_init(|| Arc::new(DhtMetrics::new().expect("metrics"))).clone()
}

fn make_state() -> State {
    let health = Arc::new(HealthState::default());
    let ready = Arc::new(ReadyGate::new());
    ready.set_ready();

    let providers = Arc::new(Store::new(Duration::from_secs(60)));
    let lookup_ctx = Arc::new(LookupCtx::new(providers.clone(), 16));

    State::new(
        health,
        ready,
        metrics(),
        providers,
        3,
        1,
        6,
        Duration::from_millis(300),
        Duration::from_millis(15),
        Duration::from_millis(50),
        lookup_ctx,
    )
}

#[tokio::test]
async fn provide_and_find_basic() {
    let st = make_state();

    let cid: B3Cid =
        "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".parse().unwrap();

    let body = ProvideBody { cid: cid.clone(), node: NODE_A_URI.to_string(), ttl_secs: Some(2) };
    let resp = provide(AxumState(st.clone()), axum::Json(body)).await.into_response();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = find_providers(AxumState(st), axum::extract::Path(cid)).await.into_response();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.expect("body bytes");
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json");

    let providers = v.get("providers").unwrap().as_array().unwrap();
    assert_eq!(providers.len(), 1, "exactly one provider expected");
    assert_eq!(providers[0], NODE_A_URI);
}

#[tokio::test]
async fn provide_rejects_bad_node_uri() {
    let st = make_state();

    let cid: B3Cid =
        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap();

    let bad = ProvideBody { cid, node: "local://nodeA".into(), ttl_secs: None };
    let resp = provide(AxumState(st), axum::Json(bad)).await.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.expect("body bytes");
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json");
    assert_eq!(v["error"], "invalid crab node URI");
}
