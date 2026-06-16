//! RO:WHAT — QuickChain Phase-0 range/media preflight tests for svc-storage.
//! RO:WHY — Storage may serve bounded bytes/ranges by b3, but must not become paid-access or chain authority.
//! RO:INTERACTS — http::routes::{get_object,head_object,put_object}, storage::MemoryStorage.
//! RO:INVARIANTS — malformed CIDs reject; valid unknown b3 is 404; invalid ranges are 416; no paid unlock claims.
//! RO:METRICS — none; HTTP contract and static source guard.
//! RO:CONFIG — in-memory app only.
//! RO:SECURITY — prevents cache/range/media paths from implying authorization, receipts, roots, or finality.
//! RO:TEST — cargo test -p svc-storage --test quickchain_preflight_range_media.

use std::{fs, path::PathBuf, sync::Arc};

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

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    fs::read_to_string(crate_dir().join(relative)).unwrap_or_else(|err| {
        panic!("failed to read {relative}: {err}");
    })
}

fn app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    let state = AppState { store };

    build_router().with_state(state)
}

fn request(method: Method, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .expect("request should build")
}

fn post_bytes(uri: &str, body: &'static [u8]) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::from(body))
        .expect("POST request should build")
}

fn range_get(uri: &str, range: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::RANGE, range)
        .body(Body::empty())
        .expect("Range GET request should build")
}

async fn send(router: Router, req: Request<Body>) -> (StatusCode, HeaderMap, Bytes) {
    let response = router
        .oneshot(req)
        .await
        .expect("router request should complete");

    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");

    (status, headers, body)
}

async fn create_object(app: Router, bytes: &'static [u8]) -> String {
    let (status, _headers, body) = send(app, post_bytes("/o", bytes)).await;
    assert_eq!(status, StatusCode::OK);

    let json: Value = serde_json::from_slice(&body).expect("POST response should be JSON");
    json["cid"]
        .as_str()
        .expect("POST response should include cid")
        .to_string()
}

#[tokio::test]
async fn read_path_serves_only_canonical_b3_and_bounded_ranges() {
    let app = app();
    let cid = create_object(app.clone(), b"0123456789abcdef").await;
    let object_path = format!("/o/{cid}");

    let (range_status, range_headers, range_body) =
        send(app.clone(), range_get(&object_path, "bytes=4-7")).await;

    assert_eq!(range_status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(range_body, Bytes::from_static(b"4567"));
    assert_eq!(
        range_headers
            .get(header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok()),
        Some("bytes 4-7/16")
    );
    assert_eq!(
        range_headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()),
        Some("4")
    );

    let (suffix_status, suffix_headers, suffix_body) =
        send(app.clone(), range_get(&object_path, "bytes=-4")).await;

    assert_eq!(suffix_status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(suffix_body, Bytes::from_static(b"cdef"));
    assert_eq!(
        suffix_headers
            .get(header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok()),
        Some("bytes 12-15/16")
    );

    let (bad_range_status, bad_range_headers, _bad_range_body) =
        send(app.clone(), range_get(&object_path, "bytes=100-101")).await;

    assert_eq!(bad_range_status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        bad_range_headers
            .get(header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok()),
        Some("*/16")
    );

    let (malformed_get_status, _headers, _body) = send(
        app.clone(),
        request(Method::GET, "/o/not-a-b3-cid", Body::empty()),
    )
    .await;

    assert_eq!(
        malformed_get_status,
        StatusCode::BAD_REQUEST,
        "malformed b3 must reject instead of being treated like an absent valid object"
    );

    let unknown_cid = format!("b3:{}", "0".repeat(64));
    let unknown_path = format!("/o/{unknown_cid}");
    let (unknown_status, _headers, _body) =
        send(app, request(Method::GET, &unknown_path, Body::empty())).await;

    assert_eq!(
        unknown_status,
        StatusCode::NOT_FOUND,
        "well-formed but absent b3 should remain a normal cache/object miss"
    );
}

#[test]
fn free_object_read_routes_do_not_claim_paid_access_or_quickchain_authority() {
    let combined = [
        "src/http/routes/get_object.rs",
        "src/http/routes/head_object.rs",
        "src/http/routes/put_object.rs",
        "src/http/routes/post_object.rs",
    ]
    .iter()
    .map(|path| read(path))
    .collect::<Vec<_>>()
    .join("\n--- route source ---\n")
    .to_ascii_lowercase();

    for forbidden in [
        "unlock",
        "paid_content",
        "wallet_txid",
        "wallet_receipt",
        "receipt_hash",
        "balance_minor",
        "state_root",
        "receipt_root",
        "checkpoint",
        "validator",
        "bridge",
        "external settlement",
    ] {
        assert!(
            !combined.contains(forbidden),
            "free storage byte/range routes must not claim paid-access or chain authority via `{forbidden}`"
        );
    }

    assert!(
        combined.contains("partial_content"),
        "GET /o/:cid must keep bounded/range media semantics visible in source"
    );
    assert!(
        combined.contains("bad_request"),
        "GET /o/:cid must reject malformed b3 CIDs before lookup"
    );
}
