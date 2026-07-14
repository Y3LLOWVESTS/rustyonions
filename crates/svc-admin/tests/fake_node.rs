// crates/svc-admin/tests/fake_node.rs
//
// WHAT: Integration tests for NodeRegistry + NodeClient against a fake node.
// WHY: Validates that svc-admin can successfully fan out to a node's
//      /api/v1/status endpoint and normalize it into AdminStatusView.
// INTERACTS: crate::config::{NodeCfg, NodesCfg}, crate::nodes::registry::NodeRegistry,
//            Axum router (test-only), reqwest (via NodeClient).
// INVARIANTS:
//   - Tests must not rely on hard-coded ports; they bind 127.0.0.1:0.
//   - Fake node schema matches the minimal RawStatus shape expected by
//     NodeClient (profile, version, planes[*]).
//   - Tests are self-contained and do not require a running macronode.

use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::time::Duration;
use svc_admin::config::{NodeCfg, NodesCfg};
use svc_admin::nodes::registry::NodeRegistry;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Shape of the fake node's `/api/v1/status` response.
///
/// This intentionally matches the fields consumed by NodeClient's RawStatus.
#[derive(Debug, Serialize)]
struct FakePlane {
    name: String,
    health: String,
    ready: bool,
    restart_count: u64,
}

#[derive(Debug, Serialize)]
struct FakeStatus {
    profile: String,
    node_role: String,
    node_profile: String,
    version: String,
    capabilities: Vec<String>,
    amnesia_mode: bool,
    privacy_mode: bool,
    public_inbound_enabled: bool,
    headless_mode: bool,
    admin_ui_enabled: bool,
    admin_ui_bind: String,
    operator_ui_profile: String,
    admin_ui_runtime_required: bool,
    verification_enabled: bool,
    content_serving_enabled: bool,
    economic_replay_enabled: bool,
    service_quorum_enabled: bool,
    wallet_execution_participant: bool,
    ledger_replay_enabled: bool,
    user_ip_publication: String,
    planes: Vec<FakePlane>,
}

/// Axum handler for `/api/v1/status` on the fake node.
async fn fake_status() -> Json<FakeStatus> {
    Json(FakeStatus {
        profile: "macronode".to_string(),
        node_role: "service_node".to_string(),
        node_profile: "macronode".to_string(),
        version: "1.2.3-test".to_string(),
        capabilities: vec!["service_node_status_v1".to_string()],
        amnesia_mode: false,
        privacy_mode: false,
        public_inbound_enabled: true,
        headless_mode: true,
        admin_ui_enabled: false,
        admin_ui_bind: "127.0.0.1:5300".to_string(),
        operator_ui_profile: "service_node_local".to_string(),
        admin_ui_runtime_required: false,
        verification_enabled: false,
        content_serving_enabled: true,
        economic_replay_enabled: false,
        service_quorum_enabled: false,
        wallet_execution_participant: false,
        ledger_replay_enabled: false,
        user_ip_publication: "not_applicable_service_node".to_string(),
        planes: vec![FakePlane {
            name: "gateway".to_string(),
            health: "ok".to_string(),
            ready: true,
            restart_count: 1,
        }],
    })
}

/// Spawn a fake node admin-plane HTTP server on 127.0.0.1:0.
///
/// Returns the bound SocketAddr and a JoinHandle for the server task.
async fn spawn_fake_node() -> (SocketAddr, JoinHandle<()>) {
    let app = Router::new().route("/api/v1/status", get(fake_status));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fake node");
    let addr = listener.local_addr().expect("get fake node local address");

    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("fake node server error: {err}");
        }
    });

    (addr, handle)
}

#[tokio::test]
async fn node_registry_returns_real_status_from_fake_node() {
    let (addr, _handle) = spawn_fake_node().await;

    // Build a NodesCfg with a single fake node pointing at our ephemeral addr.
    let mut nodes = NodesCfg::new();
    nodes.insert(
        "fake-node".into(),
        NodeCfg {
            base_url: format!("http://{}", addr),
            display_name: Some("Fake Node".into()),
            environment: "test".into(),
            insecure_http: true,

            // New fields added in Config v2:
            forced_profile: Some("macronode".into()),
            macaroon_path: None,
            default_timeout: Some(Duration::from_secs(2)),
        },
    );

    let registry = NodeRegistry::new(&nodes);

    let status = registry
        .get_status("fake-node")
        .await
        .expect("node status should exist");

    assert_eq!(status.id, "fake-node");
    assert_eq!(status.display_name, "Fake Node");
    assert_eq!(status.profile.as_deref(), Some("macronode"));
    assert_eq!(status.node_role.as_deref(), Some("service_node"));
    assert_eq!(status.node_profile.as_deref(), Some("macronode"));
    assert_eq!(status.version.as_deref(), Some("1.2.3-test"));
    assert_eq!(
        status
            .capabilities
            .as_ref()
            .expect("capabilities propagated"),
        &vec!["service_node_status_v1".to_string()]
    );
    // Older/partial status producers may omit the Phase 20 operator blocks.
    // svc-admin must preserve compatibility and report them as unknown rather
    // than manufacturing success or authority.
    assert_eq!(status.ready, None);
    assert!(status.oap.is_none());
    assert!(status.provider.is_none());
    assert!(status.policy.is_none());
    assert!(status.reward_binding.is_none());
    assert!(status.service_evidence.is_none());

    assert_eq!(status.amnesia_mode, Some(false));
    assert_eq!(status.privacy_mode, Some(false));
    assert_eq!(status.public_inbound_enabled, Some(true));
    assert_eq!(status.headless_mode, Some(true));
    assert_eq!(status.admin_ui_enabled, Some(false));
    assert_eq!(status.admin_ui_bind.as_deref(), Some("127.0.0.1:5300"));
    assert_eq!(
        status.operator_ui_profile.as_deref(),
        Some("service_node_local")
    );
    assert_eq!(status.admin_ui_runtime_required, Some(false));
    assert_eq!(status.verification_enabled, Some(false));
    assert_eq!(status.content_serving_enabled, Some(true));
    assert_eq!(status.economic_replay_enabled, Some(false));
    assert_eq!(status.service_quorum_enabled, Some(false));
    assert_eq!(status.wallet_execution_participant, Some(false));
    assert_eq!(status.ledger_replay_enabled, Some(false));
    assert_eq!(
        status.user_ip_publication.as_deref(),
        Some("not_applicable_service_node")
    );

    assert_eq!(status.planes.len(), 1);
    let plane = &status.planes[0];
    assert_eq!(plane.name, "gateway");
    assert_eq!(plane.health, "ok");
    assert!(plane.ready);
    assert_eq!(plane.restart_count, 1);
}
