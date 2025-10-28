//! Happy-path handler smoke test (no sockets).
//! Verifies: provide → find_providers JSON shape + 400 on bad input.

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

// ---- test-global metrics to avoid duplicate Prometheus registration
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
        3, // alpha
        1, // beta
        6, // hop_budget
        Duration::from_millis(300),
        Duration::from_millis(15),
        Duration::from_millis(50),
        lookup_ctx,
    )
}

#[tokio::test]
async fn provide_and_find_basic() {
    let st = make_state();

    // Provide a short-lived record.
    let cid: B3Cid =
        "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".parse().unwrap();

    let body =
        ProvideBody { cid: cid.clone(), node: "local://nodeA".to_string(), ttl_secs: Some(2) };
    let resp = provide(AxumState(st.clone()), axum::Json(body)).await.into_response();
    assert_eq!(resp.status(), StatusCode::OK);

    // Find providers (should see exactly 1).
    let resp = find_providers(AxumState(st), axum::extract::Path(cid)).await.into_response();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.expect("body bytes");
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json");
    assert_eq!(
        v.get("providers").unwrap().as_array().unwrap().len(),
        1,
        "exactly one provider expected"
    );
}

#[tokio::test]
async fn provide_rejects_bad_node_uri() {
    let st = make_state();

    let cid: B3Cid =
        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap();

    // Bad node URI should be rejected with 400.
    let bad = ProvideBody { cid, node: "not a uri".into(), ttl_secs: None };
    let resp = provide(AxumState(st), axum::Json(bad)).await.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
