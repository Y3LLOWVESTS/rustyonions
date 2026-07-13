//! In-process binary HTTP transport tests for OAP `OBJ_GET`.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use ron_policy::ModerationPolicy;
use ron_proto::ContentId;
use svc_storage::{
    http::{
        extractors::AppState,
        server::{build_router, build_router_with_moderation},
    },
    oap_object::{build_obj_get_request, encode_frame_wire, verify_obj_stream_wire},
    storage::{DynStorage, MemoryStorage},
};
use tower::ServiceExt;

const OBJECT_BYTES: &[u8] = b"phase-9 live OAP object bytes";

fn cid_for(bytes: &[u8]) -> ContentId {
    format!("b3:{}", blake3::hash(bytes).to_hex())
        .parse()
        .expect("calculated b3 CID must parse")
}

fn app(store: DynStorage) -> Router {
    build_router().with_state(AppState { store })
}

fn app_with_moderation(store: DynStorage, moderation: ModerationPolicy) -> Router {
    build_router_with_moderation(Arc::new(moderation)).with_state(AppState { store })
}

fn oap_request(body: Body) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/oap/obj-get")
        .header(header::CONTENT_TYPE, "application/oap")
        .body(body)
        .expect("OAP request should build")
}

#[tokio::test]
async fn live_oap_route_fetches_and_verifies_object() {
    let cid = cid_for(OBJECT_BYTES);
    let store: DynStorage = Arc::new(MemoryStorage::default());

    store
        .put(cid.as_str(), bytes::Bytes::from_static(OBJECT_BYTES))
        .await
        .expect("test object should store");

    let request_frame =
        build_obj_get_request(cid.clone(), 0xCAFE, 44).expect("OBJ_GET should build");

    let request_wire = encode_frame_wire(request_frame).expect("OBJ_GET should encode");

    let response = app(store)
        .oneshot(oap_request(Body::from(request_wire)))
        .await
        .expect("OAP router request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/oap")
    );

    let response_wire = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("OAP response body should read");

    let verified = verify_obj_stream_wire(&cid, &response_wire).expect("OAP response must verify");

    assert_eq!(verified.as_ref(), OBJECT_BYTES);
}

#[tokio::test]
async fn live_oap_route_reports_not_found() {
    let cid = cid_for(b"missing object");
    let store: DynStorage = Arc::new(MemoryStorage::default());

    let request_wire =
        encode_frame_wire(build_obj_get_request(cid, 1, 2).expect("OBJ_GET should build"))
            .expect("OBJ_GET should encode");

    let response = app(store)
        .oneshot(oap_request(Body::from(request_wire)))
        .await
        .expect("OAP router request should complete");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn live_oap_route_rejects_wrong_storage_bytes() {
    let claimed = cid_for(b"claimed object bytes");
    let store: DynStorage = Arc::new(MemoryStorage::default());

    store
        .put(
            claimed.as_str(),
            bytes::Bytes::from_static(b"different stored bytes"),
        )
        .await
        .expect("test object should store");

    let request_wire =
        encode_frame_wire(build_obj_get_request(claimed, 3, 4).expect("OBJ_GET should build"))
            .expect("OBJ_GET should encode");

    let response = app(store)
        .oneshot(oap_request(Body::from(request_wire)))
        .await
        .expect("OAP router request should complete");

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn live_oap_route_rejects_body_over_frame_limit() {
    let store: DynStorage = Arc::new(MemoryStorage::default());

    let oversized = vec![0_u8; oap::MAX_FRAME_BYTES as usize + 1];

    let response = app(store)
        .oneshot(oap_request(Body::from(oversized)))
        .await
        .expect("OAP router request should complete");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn live_oap_route_honors_injected_moderation_policy() {
    let object = b"runtime-moderated object";
    let cid = cid_for(object);
    let store: DynStorage = Arc::new(MemoryStorage::default());

    store
        .put(cid.as_str(), bytes::Bytes::from_static(object))
        .await
        .expect("test object should store");

    let mut moderation = ModerationPolicy::default();

    assert!(moderation.insert_local_block(
        cid.as_str()
            .parse()
            .expect("canonical ContentId must parse"),
    ));

    let request_wire =
        encode_frame_wire(build_obj_get_request(cid, 9, 10).expect("OBJ_GET should build"))
            .expect("OBJ_GET should encode");

    let response = app_with_moderation(store, moderation)
        .oneshot(oap_request(Body::from(request_wire)))
        .await
        .expect("OAP router request should complete");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
