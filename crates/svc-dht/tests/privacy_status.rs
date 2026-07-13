//! RO:WHAT — Phase 7D privacy status DTO tests for svc-dht.
//! RO:WHY — The DHT status surface must explicitly report that user/residential IP
//!   publication, raw socket publication, and transport route publication are disabled.
//! RO:INTERACTS — rpc::http::privacy_status, privacy::review_public_json.
//! RO:INVARIANTS — public privacy status is crab://node-only and leaks no IP/socket route data.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::extract::State as AxumState;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ron_kernel::HealthState;
use serde_json::Value;
use svc_dht::metrics::DhtMetrics;
use svc_dht::pipeline::lookup::LookupCtx;
use svc_dht::privacy::review_public_json;
use svc_dht::provider::Store;
use svc_dht::readiness::ReadyGate;
use svc_dht::rpc::http::{privacy_status, State};
use svc_dht::types::CrabNodeId;

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
    let node_id = CrabNodeId::from_uri(NODE_A_URI).expect("valid crab node id");

    State::new_with_node_id(
        node_id,
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
async fn privacy_status_reports_fail_closed_public_route_posture() {
    let st = make_state();

    let resp = privacy_status(AxumState(st)).await.into_response();
    assert_eq!(resp.status(), StatusCode::OK);

    let json = response_json(resp).await;

    assert_eq!(json["node"], NODE_A_URI);
    assert_eq!(json["privacy_mode"], true);
    assert_eq!(json["privacy_profile"], "service_node_dht_public_boundary");
    assert_eq!(json["public_inbound_enabled"], false);
    assert_eq!(json["user_ip_publication"], "forbidden");
    assert_eq!(json["residential_ip_publication"], "forbidden");
    assert_eq!(json["lan_ip_publication"], "forbidden");
    assert_eq!(json["admin_bind_publication"], false);
    assert_eq!(json["provider_identity_kind"], "crab_node");
    assert_eq!(json["provider_records_use_raw_socket"], false);
    assert_eq!(json["transport_routes_public"], false);
    assert_eq!(json["raw_socket_publication"], false);
    assert_eq!(json["peer_socket_publication"], false);
    assert_eq!(json["service_routes_public"], false);
    assert_eq!(json["dto_ip_fields_allowed"], false);
    assert_eq!(json["route_identity_scheme"], "crab://node");
}

#[tokio::test]
async fn privacy_status_dto_has_no_ip_or_transport_route_leaks() {
    let st = make_state();

    let resp = privacy_status(AxumState(st)).await.into_response();
    assert_eq!(resp.status(), StatusCode::OK);

    let json = response_json(resp).await;
    let findings = review_public_json("privacy_status", &json);

    assert!(findings.is_empty(), "privacy_status leaked private route data: {findings:?}");
}
