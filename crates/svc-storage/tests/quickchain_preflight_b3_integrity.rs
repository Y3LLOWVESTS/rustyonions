//! RO:WHAT — Runtime b3 integrity tests for svc-storage QuickChain preflight.
//! RO:WHY — QuickChain depends on content-addressed bytes, but b3 is content truth only.
//! RO:INTERACTS — http::server::build_router, MemoryStorage, PUT/GET/HEAD /o.
//! RO:INVARIANTS — ingest derives b3 from bytes; ETag matches content hash; caller cannot pick a fake CID.
//! RO:METRICS — none.
//! RO:CONFIG — in-process amnesia-safe memory store only.
//! RO:SECURITY — proves b3 is byte integrity, not payment/finality authority.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_b3_integrity.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body, Bytes},
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use svc_storage::{
    http::{extractors::AppState, server::build_router},
    storage::{MemoryStorage, Storage},
};
use tower::ServiceExt;

const OBJECT_BYTES: &[u8] = b"quickchain storage b3 integrity proof";

fn app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    build_router().with_state(AppState { store })
}

fn expected_cid(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn post_o(bytes: &'static [u8]) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/o")
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::from(bytes))
        .expect("POST /o request should build")
}

fn get(path: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(path)
        .body(Body::empty())
        .expect("GET request should build")
}

fn head(path: &str) -> Request<Body> {
    Request::builder()
        .method(Method::HEAD)
        .uri(path)
        .body(Body::empty())
        .expect("HEAD request should build")
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, HeaderMap, Bytes) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");

    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");

    (status, headers, body)
}

#[tokio::test]
async fn object_ingest_derives_canonical_b3_from_bytes() {
    let app = app();
    let expected = expected_cid(OBJECT_BYTES);

    let (status, _headers, body) = send(app.clone(), post_o(OBJECT_BYTES)).await;
    assert_eq!(status, StatusCode::OK);

    let json: Value = serde_json::from_slice(&body).expect("PUT response should be JSON");
    let cid = json["cid"].as_str().expect("response should include cid");

    assert_eq!(cid, expected);
    assert_eq!(cid.len(), 67);
    assert!(cid.starts_with("b3:"));
    assert!(cid[3..]
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));

    let object_path = format!("/o/{cid}");
    let (get_status, get_headers, get_body) = send(app.clone(), get(&object_path)).await;
    assert_eq!(get_status, StatusCode::OK);
    assert_eq!(get_body, Bytes::from_static(OBJECT_BYTES));

    let expected_etag = format!("\"{}\"", blake3::hash(OBJECT_BYTES).to_hex());
    assert_eq!(
        get_headers
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok()),
        Some(expected_etag.as_str())
    );
}

#[tokio::test]
async fn caller_cannot_retrieve_bytes_under_fake_or_noncanonical_cid() {
    let app = app();
    let real_cid = expected_cid(OBJECT_BYTES);

    let (post_status, _headers, _body) = send(app.clone(), post_o(OBJECT_BYTES)).await;
    assert_eq!(post_status, StatusCode::OK);

    let fake_cid = format!("b3:{}", "0".repeat(64));
    assert_ne!(fake_cid, real_cid);

    for path in [
        format!("/o/{fake_cid}"),
        format!("/o/{}", real_cid.to_uppercase()),
        "/o/not-a-b3-cid".to_string(),
    ] {
        let (get_status, _headers, _body) = send(app.clone(), get(&path)).await;
        assert!(
            matches!(get_status, StatusCode::NOT_FOUND | StatusCode::BAD_REQUEST),
            "nonexistent or noncanonical CID should not return bytes: {path} -> {get_status}"
        );
    }

    let real_path = format!("/o/{real_cid}");
    let expected_len = OBJECT_BYTES.len().to_string();
    let (head_status, head_headers, _head_body) = send(app, head(&real_path)).await;
    assert_eq!(head_status, StatusCode::OK);
    assert_eq!(
        head_headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()),
        Some(expected_len.as_str())
    );
}
