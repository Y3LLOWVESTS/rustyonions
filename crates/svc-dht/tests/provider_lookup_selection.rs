//! RO:WHAT — Proves the real lookup handler uses status-aware provider candidates.
//! RO:WHY — Phase 12 discovery must not return unavailable providers or bypass ordering.
//! RO:INTERACTS — pipeline::lookup, provider::Store, rpc::http::find_providers.
//! RO:INVARIANTS — responsive first; unavailable excluded; result count bounded by alpha.
//! RO:SECURITY — returned identities remain canonical crab://node URIs only.
//! RO:TEST — cargo test -p svc-dht --test provider_lookup_selection.

use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use axum::{
    extract::{Path, State as AxumState},
    http::StatusCode,
    response::IntoResponse,
};
use ron_kernel::HealthState;
use svc_dht::{
    metrics::DhtMetrics,
    pipeline::lookup::LookupCtx,
    provider::{ProviderStatusHint, ProviderStatusUpdateOutcome, Store},
    readiness::ReadyGate,
    rpc::http::{find_providers, State},
    types::{B3Cid, CrabNodeId},
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";
const NODE_B_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";
const NODE_C_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000c3";
const NODE_D_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000d4";
const NODE_E_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000e5";

static METRICS: OnceLock<Arc<DhtMetrics>> = OnceLock::new();

fn metrics() -> Arc<DhtMetrics> {
    METRICS.get_or_init(|| Arc::new(DhtMetrics::new().expect("DHT metrics"))).clone()
}

fn node(uri: &str) -> CrabNodeId {
    CrabNodeId::from_uri(uri).expect("test node URI must be canonical")
}

fn make_state(providers: Arc<Store>, alpha: usize) -> State {
    let health = Arc::new(HealthState::default());
    let ready = Arc::new(ReadyGate::new());
    ready.set_ready();

    let lookup_ctx = Arc::new(LookupCtx::new(providers.clone(), 16));

    State::new(
        health,
        ready,
        metrics(),
        providers,
        alpha,
        1,
        6,
        Duration::from_millis(300),
        Duration::from_millis(15),
        Duration::from_millis(50),
        lookup_ctx,
    )
}

fn add_provider(store: &Store, uri: &str) {
    store
        .add(CID.to_owned(), uri.to_owned(), Some(Duration::from_secs(60)))
        .expect("canonical provider record");
}

#[tokio::test]
async fn find_providers_uses_status_order_and_alpha_bound() {
    let providers = Arc::new(Store::new(Duration::from_secs(60)));

    // Insert in deliberately non-preferred order.
    for uri in [NODE_E_URI, NODE_C_URI, NODE_A_URI, NODE_D_URI, NODE_B_URI] {
        add_provider(&providers, uri);
    }

    assert_eq!(
        providers.record_local_status(CID, node(NODE_B_URI), ProviderStatusHint::Responsive,),
        ProviderStatusUpdateOutcome::Updated
    );

    assert_eq!(
        providers.record_local_status(CID, node(NODE_D_URI), ProviderStatusHint::Responsive,),
        ProviderStatusUpdateOutcome::Updated
    );

    assert_eq!(
        providers.record_local_status(CID, node(NODE_A_URI), ProviderStatusHint::Degraded,),
        ProviderStatusUpdateOutcome::Updated
    );

    assert_eq!(
        providers.record_local_status(CID, node(NODE_E_URI), ProviderStatusHint::Unavailable,),
        ProviderStatusUpdateOutcome::Updated
    );

    // Alpha=3 means only the first three eligible candidates return.
    let state = make_state(providers, 3);
    let cid: B3Cid = CID.parse().expect("canonical CID");

    let response = find_providers(AxumState(state), Path(cid)).await.into_response();

    assert_eq!(response.status(), StatusCode::OK);

    let body =
        axum::body::to_bytes(response.into_body(), 1024 * 1024).await.expect("response body");

    let value: serde_json::Value = serde_json::from_slice(&body).expect("response JSON");

    let returned = value["providers"]
        .as_array()
        .expect("provider array")
        .iter()
        .map(|value| value.as_str().expect("provider must be a string"))
        .collect::<Vec<_>>();

    assert_eq!(returned, vec![NODE_B_URI, NODE_D_URI, NODE_C_URI,]);

    assert!(
        !returned.contains(&NODE_A_URI),
        "degraded fourth candidate must be excluded by alpha bound"
    );

    assert!(!returned.contains(&NODE_E_URI), "unavailable provider must never be returned");
}

#[tokio::test]
async fn find_providers_fails_when_only_unavailable_records_remain() {
    let providers = Arc::new(Store::new(Duration::from_secs(60)));

    add_provider(&providers, NODE_A_URI);

    assert_eq!(
        providers.record_local_status(CID, node(NODE_A_URI), ProviderStatusHint::Unavailable,),
        ProviderStatusUpdateOutcome::Updated
    );

    let state = make_state(providers, 3);
    let cid: B3Cid = CID.parse().expect("canonical CID");

    let response = find_providers(AxumState(state), Path(cid)).await.into_response();

    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);

    let body =
        axum::body::to_bytes(response.into_body(), 1024 * 1024).await.expect("response body");

    let value: serde_json::Value = serde_json::from_slice(&body).expect("response JSON");

    assert_eq!(value["error"], "lookup failed or timed out");
}
