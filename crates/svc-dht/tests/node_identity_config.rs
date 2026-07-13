//! RO:WHAT — Phase 7C service node identity config/DTO tests.
//! RO:WHY — svc-dht must expose only canonical crab://node identity, never raw routes/IPs.
//! RO:INTERACTS — Config::from_env, rpc::http::node_identity, privacy::review_public_json.
//! RO:INVARIANTS — bad DHT_NODE_URI fails closed before service startup.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use axum::extract::State as AxumState;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ron_kernel::HealthState;
use serde_json::Value;
use svc_dht::config::Config;
use svc_dht::metrics::DhtMetrics;
use svc_dht::pipeline::lookup::LookupCtx;
use svc_dht::privacy::review_public_json;
use svc_dht::provider::Store;
use svc_dht::readiness::ReadyGate;
use svc_dht::rpc::http::{node_identity, State};
use svc_dht::types::CrabNodeId;

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";
const NODE_A_HEX: &str = "00000000000000000000000000000000000000000000000000000000000000a1";

static ENV_LOCK: Mutex<()> = Mutex::new(());
static METRICS: OnceLock<Arc<DhtMetrics>> = OnceLock::new();

const CONFIG_ENV_KEYS: &[&str] = &[
    "DHT_ADMIN_BIND",
    "DHT_NODE_URI",
    "RON_DHT_NODE_URI",
    "DHT_ALPHA",
    "DHT_BETA",
    "DHT_K",
    "DHT_HOP_BUDGET",
    "DHT_DIAL_TIMEOUT_MS",
    "DHT_IDLE_TIMEOUT_MS",
    "DHT_SEEDS",
    "RON_AMNESIA",
];

struct EnvGuard(Vec<(&'static str, Option<String>)>);

impl EnvGuard {
    fn clear() -> Self {
        let saved =
            CONFIG_ENV_KEYS.iter().map(|key| (*key, std::env::var(key).ok())).collect::<Vec<_>>();

        for key in CONFIG_ENV_KEYS {
            std::env::remove_var(key);
        }

        Self(saved)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn metrics() -> Arc<DhtMetrics> {
    METRICS.get_or_init(|| Arc::new(DhtMetrics::new().expect("metrics"))).clone()
}

fn make_state(node_id: CrabNodeId) -> State {
    let health = Arc::new(HealthState::default());
    let ready = Arc::new(ReadyGate::new());
    ready.set_ready();

    let providers = Arc::new(Store::new(Duration::from_secs(60)));
    let lookup_ctx = Arc::new(LookupCtx::new(providers.clone(), 16));

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

#[test]
fn config_from_env_accepts_canonical_dht_node_uri() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::clear();

    std::env::set_var("DHT_NODE_URI", NODE_A_URI);

    let cfg = Config::from_env().expect("canonical DHT_NODE_URI should be accepted");

    assert_eq!(cfg.node_uri(), NODE_A_URI);
    assert_eq!(cfg.node_id.to_node_hex(), NODE_A_HEX);
}

#[test]
fn config_from_env_rejects_legacy_or_transport_node_uri() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::clear();

    std::env::set_var("DHT_NODE_URI", "tcp://192.168.1.10:7000");

    let err = Config::from_env().expect_err("transport-specific DHT_NODE_URI must fail closed");

    assert!(err.to_string().contains("missing_crab_node_prefix"), "unexpected error: {err}");
}

#[tokio::test]
async fn node_identity_dto_exposes_only_crab_node_identity() {
    let node_id = CrabNodeId::from_uri(NODE_A_URI).expect("valid crab node id");
    let st = make_state(node_id);

    let resp = node_identity(AxumState(st)).await.into_response();
    assert_eq!(resp.status(), StatusCode::OK);

    let json = response_json(resp).await;

    assert_eq!(json["node"], NODE_A_URI);
    assert_eq!(json["node_id_hex"], NODE_A_HEX);
    assert_eq!(json["identity_scheme"], "crab://node");
    assert_eq!(json["provider_identity_kind"], "crab_node");
    assert_eq!(json["public_inbound"], false);

    let findings = review_public_json("node_identity", &json);
    assert!(findings.is_empty(), "node_identity leaked private route data: {findings:?}");
}
