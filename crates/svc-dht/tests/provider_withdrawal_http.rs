//! RO:WHAT — Local provider-withdrawal HTTP handler tests.
//!
//! RO:WHY — Object prune coordination needs a bounded DHT operation that
//! withdraws only the service's configured node identity.
//!
//! RO:INVARIANTS — request supplies only a canonical B3 CID; configured
//! CrabNodeId is authoritative; other providers remain; repeat returns 404;
//! response claims local provider-store scope only.
//!
//! RO:TEST — cargo test -p svc-dht --test provider_withdrawal_http.

use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use axum::{extract::State as AxumState, http::StatusCode, response::IntoResponse, Json};
use ron_kernel::HealthState;
use svc_dht::{
    metrics::DhtMetrics,
    pipeline::lookup::LookupCtx,
    provider::Store,
    readiness::ReadyGate,
    rpc::http::{withdraw_local_provider, State, WithdrawLocalProviderBody},
    types::{B3Cid, CrabNodeId},
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";

const NODE_B_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";

static METRICS: OnceLock<Arc<DhtMetrics>> = OnceLock::new();

fn metrics() -> Arc<DhtMetrics> {
    METRICS.get_or_init(|| Arc::new(DhtMetrics::new().expect("metrics"))).clone()
}

fn node(uri: &str) -> CrabNodeId {
    CrabNodeId::from_uri(uri).expect("test node URI must be canonical")
}

fn cid() -> B3Cid {
    CID.parse().expect("test CID must be canonical")
}

fn make_state(configured_node: CrabNodeId) -> State {
    let health = Arc::new(HealthState::default());
    let ready = Arc::new(ReadyGate::new());
    ready.set_ready();

    let providers = Arc::new(Store::new(Duration::from_secs(60)));
    let lookup_ctx = Arc::new(LookupCtx::new(providers.clone(), 16));

    State::new_with_node_id(
        configured_node,
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
async fn handler_withdraws_only_the_configured_local_node() {
    let st = make_state(node(NODE_A_URI));

    st.providers
        .add(CID.to_owned(), NODE_A_URI.to_owned(), Some(Duration::from_secs(60)))
        .expect("configured-node provider record should be accepted");

    st.providers
        .add(CID.to_owned(), NODE_B_URI.to_owned(), Some(Duration::from_secs(60)))
        .expect("neighbor provider record should be accepted");

    let response = withdraw_local_provider(
        AxumState(st.clone()),
        Json(WithdrawLocalProviderBody { cid: cid() }),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);

    let body =
        axum::body::to_bytes(response.into_body(), 1024 * 1024).await.expect("response body");

    let value: serde_json::Value = serde_json::from_slice(&body).expect("response JSON");

    assert_eq!(value["ok"], true);
    assert_eq!(value["status"], "withdrawn");
    assert_eq!(value["scope"], "local_provider_store");
    assert_eq!(value["cid"], CID);
    assert_eq!(value["node"], NODE_A_URI);

    assert_eq!(
        st.providers.get_live(CID),
        vec![NODE_B_URI.to_owned()],
        "the neighboring provider must remain"
    );
}

#[tokio::test]
async fn repeated_handler_withdrawal_returns_not_found() {
    let st = make_state(node(NODE_A_URI));

    st.providers
        .add(CID.to_owned(), NODE_A_URI.to_owned(), Some(Duration::from_secs(60)))
        .expect("configured-node provider record should be accepted");

    let first = withdraw_local_provider(
        AxumState(st.clone()),
        Json(WithdrawLocalProviderBody { cid: cid() }),
    )
    .await
    .into_response();

    assert_eq!(first.status(), StatusCode::OK);

    let second =
        withdraw_local_provider(AxumState(st), Json(WithdrawLocalProviderBody { cid: cid() }))
            .await
            .into_response();

    assert_eq!(second.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(second.into_body(), 1024 * 1024).await.expect("response body");

    let value: serde_json::Value = serde_json::from_slice(&body).expect("response JSON");

    assert_eq!(value["ok"], false);
    assert_eq!(value["status"], "not_found");
    assert_eq!(value["scope"], "local_provider_store");
}

#[test]
fn withdrawal_body_rejects_caller_supplied_node_identity() {
    let err = serde_json::from_value::<WithdrawLocalProviderBody>(serde_json::json!({
        "cid": CID,
        "node": NODE_B_URI
    }))
    .expect_err("caller-supplied node identity must reject");

    assert!(err.to_string().contains("unknown field"), "unexpected error: {err}");
}
