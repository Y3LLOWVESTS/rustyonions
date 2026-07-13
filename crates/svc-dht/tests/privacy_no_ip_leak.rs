//! RO:WHAT — Phase 7B no-IP-leak tests for svc-dht public JSON surfaces.
//! RO:WHY — Provider discovery may expose crab://node identities, but must not expose raw IPs,
//!   socket addresses, or transport-specific provider routes.
//! RO:INTERACTS — privacy::review_public_json, rpc::http provide/find/debug handlers.
//! RO:INVARIANTS — public DHT DTOs remain crab://node-only at the provider boundary.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::extract::State as AxumState;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ron_kernel::HealthState;
use serde_json::{json, Value};
use svc_dht::metrics::DhtMetrics;
use svc_dht::pipeline::lookup::LookupCtx;
use svc_dht::privacy::review_public_json;
use svc_dht::provider::Store;
use svc_dht::readiness::ReadyGate;
use svc_dht::rpc::http::{debug_list, find_providers, provide, ProvideBody, State};
use svc_dht::types::B3Cid;

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
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

async fn response_json(resp: Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.expect("body bytes");
    serde_json::from_slice(&bytes).expect("json response")
}

#[tokio::test]
async fn find_and_debug_outputs_do_not_expose_ip_or_socket_fields() {
    let st = make_state();
    let cid: B3Cid = CID.parse().expect("valid b3 cid");

    let body = ProvideBody { cid: cid.clone(), node: NODE_A_URI.to_string(), ttl_secs: Some(60) };
    let provide_resp = provide(AxumState(st.clone()), axum::Json(body)).await.into_response();
    assert_eq!(provide_resp.status(), StatusCode::OK);

    let find_resp =
        find_providers(AxumState(st.clone()), axum::extract::Path(cid)).await.into_response();
    assert_eq!(find_resp.status(), StatusCode::OK);
    let find_json = response_json(find_resp).await;

    assert_eq!(find_json["providers"][0], NODE_A_URI);
    let find_leaks = review_public_json("find_providers", &find_json);
    assert!(find_leaks.is_empty(), "find_providers leaked private routing data: {find_leaks:?}");

    let debug_resp = debug_list(AxumState(st)).await.into_response();
    assert_eq!(debug_resp.status(), StatusCode::OK);
    let debug_json = response_json(debug_resp).await;

    let debug_leaks = review_public_json("debug_list", &debug_json);
    assert!(debug_leaks.is_empty(), "debug_list leaked private routing data: {debug_leaks:?}");
}

#[test]
fn privacy_reviewer_accepts_crab_node_identity_json() {
    let value = json!({
        "cid": CID,
        "providers": [NODE_A_URI],
        "node": NODE_A_URI,
        "privacy_route_id": "route_0001",
        "request_id": "req_0001"
    });

    let findings = review_public_json("good_public_dto", &value);
    assert!(findings.is_empty(), "crab://node DTO should be clean: {findings:?}");
}

#[test]
fn privacy_reviewer_flags_ip_fields_and_raw_transport_routes() {
    let value = json!({
        "source_ip": "192.168.1.10",
        "viewer_ip": "10.0.0.2",
        "socket_addr": "127.0.0.1:5301",
        "providers": [
            "tcp://192.168.1.10:7000",
            "http://10.0.0.2:5301",
            "local://nodeA",
            "relay://nodeB",
            "onion://nodeC",
            "service://nodeD"
        ]
    });

    let findings = review_public_json("bad_public_dto", &value);

    assert!(
        findings.iter().any(|finding| finding.reason == "forbidden_ip_field_name"),
        "expected forbidden field-name finding, got {findings:?}"
    );
    assert!(
        findings.iter().any(|finding| finding.reason == "forbidden_route_or_ip_literal"),
        "expected forbidden route/IP finding, got {findings:?}"
    );
}
