//! RO:WHAT — Phase 7E svc-admin status privacy projection tests.
//! RO:WHY — svc-admin must preserve privacy posture from micronode/macronode status while
//!          not inventing raw socket/public route fields.
//! RO:INTERACTS — nodes::status::{RawStatus, from_raw}, dto::node::AdminStatusView.
//! RO:INVARIANTS — privacy booleans/status strings survive normalization.

use std::path::PathBuf;
use std::time::Duration;

use svc_admin::config::NodeCfg;
use svc_admin::nodes::status::{from_raw, RawPlaneStatus, RawStatus};

fn node_cfg() -> NodeCfg {
    NodeCfg {
        base_url: "http://127.0.0.1:5310".to_string(),
        display_name: Some("Micronode".to_string()),
        environment: "dev".to_string(),
        insecure_http: true,
        forced_profile: Some("micronode".to_string()),
        macaroon_path: Option::<PathBuf>::None,
        default_timeout: Some(Duration::from_secs(2)),
    }
}

#[test]
fn status_normalization_preserves_privacy_route_posture() {
    let raw = RawStatus {
        profile: Some("micronode".to_string()),
        node_role: Some("user_node".to_string()),
        node_profile: Some("micronode".to_string()),
        version: "0.1.0-test".to_string(),
        uptime_seconds: Some(42),
        capabilities: Some(vec!["user_node_status_v1".to_string()]),
        amnesia_mode: Some(true),
        privacy_mode: Some(true),
        public_inbound_enabled: Some(false),
        headless_mode: Some(false),
        admin_ui_enabled: Some(false),
        admin_ui_bind: None,
        operator_ui_profile: None,
        admin_ui_runtime_required: Some(false),
        verification_enabled: Some(true),
        content_serving_enabled: Some(false),
        economic_replay_enabled: Some(true),
        service_quorum_enabled: Some(false),
        wallet_execution_participant: Some(false),
        ledger_replay_enabled: Some(false),
        user_ip_publication: Some("forbidden".to_string()),
        peer_ip_display: Some("forbidden".to_string()),
        admin_bind_publication: Some(false),
        service_socket_publication: Some("not_public".to_string()),
        transport_routes_public: Some(false),
        raw_socket_publication: Some(false),
        planes: vec![RawPlaneStatus {
            name: "micronode".to_string(),
            health: "healthy".to_string(),
            ready: true,
            restart_count: 0,
        }],
    };

    let view = from_raw("micro", &node_cfg(), raw);

    assert_eq!(view.node_role.as_deref(), Some("user_node"));
    assert_eq!(view.node_profile.as_deref(), Some("micronode"));
    assert_eq!(view.privacy_mode, Some(true));
    assert_eq!(view.public_inbound_enabled, Some(false));
    assert_eq!(view.user_ip_publication.as_deref(), Some("forbidden"));
    assert_eq!(view.peer_ip_display.as_deref(), Some("forbidden"));
    assert_eq!(view.admin_bind_publication, Some(false));
    assert_eq!(
        view.service_socket_publication.as_deref(),
        Some("not_public")
    );
    assert_eq!(view.transport_routes_public, Some(false));
    assert_eq!(view.raw_socket_publication, Some(false));
}
