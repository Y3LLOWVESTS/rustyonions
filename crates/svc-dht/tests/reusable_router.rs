//! RO:WHAT — Reusable svc-dht router wiring test.
//!
//! RO:WHY — Embedded runtimes must use the same tested DHT surface as the
//! standalone binary instead of reconstructing provider routes.
//!
//! RO:INVARIANTS — the supplied State owns provider records and node identity;
//! local withdrawal targets only that configured node.
//!
//! RO:TEST — cargo test -p svc-dht --test reusable_router.

use std::{sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use ron_kernel::HealthState;
use svc_dht::{
    metrics::DhtMetrics,
    pipeline::lookup::LookupCtx,
    provider::Store,
    readiness::ReadyGate,
    rpc::http::{build_router, State},
    types::CrabNodeId,
};
use tower::ServiceExt;

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";

const NODE_B_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";

fn node(uri: &str) -> CrabNodeId {
    CrabNodeId::from_uri(uri).expect("test node URI must be canonical")
}

#[tokio::test]
async fn reusable_router_withdraws_only_the_supplied_states_node() {
    let health = Arc::new(HealthState::default());
    let ready = Arc::new(ReadyGate::new());
    ready.set_ready();

    let metrics = Arc::new(DhtMetrics::new().expect("DHT metrics"));
    let providers = Arc::new(Store::new(Duration::from_secs(60)));
    let lookup_ctx = Arc::new(LookupCtx::new(providers.clone(), 16));

    providers
        .add(CID.to_owned(), NODE_A_URI.to_owned(), Some(Duration::from_secs(60)))
        .expect("configured-node provider record should be accepted");

    providers
        .add(CID.to_owned(), NODE_B_URI.to_owned(), Some(Duration::from_secs(60)))
        .expect("neighbor provider record should be accepted");

    let state = State::new_with_node_id(
        node(NODE_A_URI),
        health,
        ready,
        metrics,
        providers.clone(),
        3,
        1,
        6,
        Duration::from_millis(300),
        Duration::from_millis(25),
        Duration::from_millis(50),
        lookup_ctx,
    );

    let request = Request::builder()
        .method("POST")
        .uri("/dht/withdraw_local_provider")
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"cid":"{CID}"}}"#)))
        .expect("withdrawal request");

    let response = build_router(state).oneshot(request).await.expect("router response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.expect("response body");

    let value: serde_json::Value = serde_json::from_slice(&body).expect("response JSON");

    assert_eq!(value["status"], "withdrawn");
    assert_eq!(value["scope"], "local_provider_store");
    assert_eq!(value["node"], NODE_A_URI);

    assert_eq!(
        providers.get_live(CID),
        vec![NODE_B_URI.to_owned()],
        "router must preserve the neighboring provider"
    );
}
