//! RO:WHAT — Proves bounded moderation-refusal metrics for every object-read surface.
//! RO:WHY — Phase 10 needs refusal evidence without exposing content or requester identity.
//! RO:INTERACTS — svc-storage router, OAP OBJ_GET, legacy GET/HEAD, ron-policy.
//! RO:INVARIANTS — only moderation refusals count; allowed/not-found/invalid requests do not.
//! RO:METRICS — storage_moderation_refusals_total{route,reason}.
//! RO:CONFIG — runs with the existing svc-storage `metrics` feature.
//! RO:SECURITY — asserts no CID, path, IP, identity, or policy-path labels.
//! RO:TEST — cargo test -p svc-storage --features metrics --test moderation_observability.

#![cfg(feature = "metrics")]

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
    Router,
};
use ron_policy::{B3Id, ModerationPolicy};
use ron_proto::ContentId;
use svc_storage::{
    http::{extractors::AppState, server::build_router_with_moderation},
    oap_object::{build_obj_get_request, encode_frame_wire},
    storage::{DynStorage, MemoryStorage},
};
use tower::ServiceExt;

const BLOCKED_CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const MISSING_CID: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

const ALLOWED_BYTES: &[u8] = b"allowed moderation metrics object";

const METRIC_PREFIX: &str = "storage_moderation_refusals_total{";

async fn app() -> Router {
    let store: DynStorage = Arc::new(MemoryStorage::default());

    let allowed_cid = format!("b3:{}", blake3::hash(ALLOWED_BYTES).to_hex());

    store
        .put(&allowed_cid, bytes::Bytes::from_static(ALLOWED_BYTES))
        .await
        .expect("allowed object should store");

    let mut moderation = ModerationPolicy::default();

    let blocked: B3Id = BLOCKED_CID.parse().expect("blocked CID must parse");

    assert!(moderation.insert_local_block(blocked));

    build_router_with_moderation(Arc::new(moderation)).with_state(AppState { store })
}

fn legacy_request(method: Method, cid: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(format!("/o/{cid}"))
        .body(Body::empty())
        .expect("legacy request should build")
}

fn oap_request(cid: &str) -> Request<Body> {
    let content_id: ContentId = cid.parse().expect("OAP CID must parse");

    let frame = build_obj_get_request(content_id, 17, 29).expect("OBJ_GET should build");

    let wire = encode_frame_wire(frame).expect("OBJ_GET should encode");

    Request::builder()
        .method(Method::POST)
        .uri("/oap/obj-get")
        .header("content-type", "application/oap")
        .body(Body::from(wire))
        .expect("OAP request should build")
}

async fn status(app: Router, request: Request<Body>) -> StatusCode {
    app.oneshot(request)
        .await
        .expect("router request should complete")
        .status()
}

async fn metrics_text(app: Router) -> String {
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/metrics")
                .body(Body::empty())
                .expect("metrics request should build"),
        )
        .await
        .expect("metrics request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("metrics body should read");

    String::from_utf8(body.to_vec()).expect("metrics body should be UTF-8")
}

fn metric_value(metrics: &str, route: &str, reason: &str) -> f64 {
    metrics
        .lines()
        .filter(|line| line.starts_with(METRIC_PREFIX))
        .find_map(|line| {
            let (labels, value) = line.rsplit_once(' ')?;

            if labels.contains(&format!("route=\"{route}\""))
                && labels.contains(&format!("reason=\"{reason}\""))
            {
                value.parse::<f64>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0.0)
}

fn assert_metric_delta(before: &str, after: &str, route: &str, reason: &str, delta: f64) {
    assert_eq!(
        metric_value(after, route, reason) - metric_value(before, route, reason),
        delta,
        "unexpected counter delta for route={route} reason={reason}"
    );
}

#[tokio::test]
async fn all_read_surfaces_count_only_bounded_moderation_refusals() {
    let app = app().await;
    let before = metrics_text(app.clone()).await;

    let allowed_cid = format!("b3:{}", blake3::hash(ALLOWED_BYTES).to_hex());

    // Allowed traffic on every read surface must not count.
    assert_eq!(
        status(app.clone(), legacy_request(Method::GET, &allowed_cid),).await,
        StatusCode::OK
    );

    assert_eq!(
        status(app.clone(), legacy_request(Method::HEAD, &allowed_cid),).await,
        StatusCode::OK
    );

    assert_eq!(
        status(app.clone(), oap_request(&allowed_cid),).await,
        StatusCode::OK
    );

    // Not-found and malformed identifiers are not moderation refusals.
    assert_eq!(
        status(app.clone(), legacy_request(Method::GET, MISSING_CID),).await,
        StatusCode::NOT_FOUND
    );

    assert_eq!(
        status(app.clone(), legacy_request(Method::GET, "not-a-cid"),).await,
        StatusCode::BAD_REQUEST
    );

    // One refusal through every active object-read surface.
    assert_eq!(
        status(app.clone(), legacy_request(Method::GET, BLOCKED_CID),).await,
        StatusCode::FORBIDDEN
    );

    assert_eq!(
        status(app.clone(), legacy_request(Method::HEAD, BLOCKED_CID),).await,
        StatusCode::FORBIDDEN
    );

    assert_eq!(
        status(app.clone(), oap_request(BLOCKED_CID),).await,
        StatusCode::FORBIDDEN
    );

    let after = metrics_text(app).await;

    assert_metric_delta(&before, &after, "legacy_get", "local_block", 1.0);

    assert_metric_delta(&before, &after, "legacy_head", "local_block", 1.0);

    assert_metric_delta(&before, &after, "oap_obj_get", "local_block", 1.0);

    let allowed_routes = [
        "route=\"legacy_get\"",
        "route=\"legacy_head\"",
        "route=\"oap_obj_get\"",
    ];

    let allowed_reasons = [
        "reason=\"global_deny\"",
        "reason=\"owner_tombstone\"",
        "reason=\"local_block\"",
        "reason=\"quarantined\"",
    ];

    for line in after.lines().filter(|line| line.starts_with(METRIC_PREFIX)) {
        let labels = line
            .split_once('{')
            .and_then(|(_, rest)| rest.split_once('}'))
            .map(|(labels, _)| labels)
            .expect("moderation metric should contain labels");

        assert_eq!(labels.split(',').count(), 2, "unexpected labels: {line}");

        assert!(
            allowed_routes.iter().any(|label| line.contains(label)),
            "unbounded route label: {line}"
        );

        assert!(
            allowed_reasons.iter().any(|label| line.contains(label)),
            "unbounded reason label: {line}"
        );

        assert!(!line.contains("b3:"), "CID leaked into metric: {line}");

        assert!(!line.contains("/o/"), "path leaked into metric: {line}");

        assert!(!line.contains("127.0.0.1"), "IP leaked into metric: {line}");
    }

    assert!(!after.contains("reason=\"no_rule\""));
    assert!(!after.contains("reason=\"local_allow\""));
}
