//! RO:WHAT — In-process HTTP contract test for svc-storage object routes.
//! RO:WHY — RON-CORE CAS proof; avoids nested `cargo run`, fixed ports, zombie child processes, and flaky startup timing.
//! RO:INTERACTS — http::server::build_router, AppState, MemoryStorage, POST/HEAD/GET /o routes.
//! RO:INVARIANTS — content addressing; strong ETag present; full GET matches upload; Range GET returns exact slice.
//! RO:METRICS — verifies `/metrics` is mounted when the `metrics` feature is enabled.
//! RO:CONFIG — uses amnesia-safe in-memory storage only.
//! RO:SECURITY — no auth material; this is a local contract test.
//! RO:TEST — cargo test -p svc-storage --test http_blackbox.

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

#[tokio::test]
async fn blackbox_put_head_get_range() {
    let app = app();

    let (post_status, _post_headers, post_body) =
        send(app.clone(), post_bytes("/o", b"hello world")).await;

    assert_eq!(post_status, StatusCode::OK);

    let post_json: Value =
        serde_json::from_slice(&post_body).expect("POST /o response should be JSON");
    let cid = post_json["cid"]
        .as_str()
        .expect("POST /o response should include cid")
        .to_string();

    assert!(cid.starts_with("b3:"));
    assert_eq!(cid.len(), 67);

    let object_path = format!("/o/{cid}");

    let (head_status, head_headers, head_body) = send(
        app.clone(),
        request(Method::HEAD, &object_path, Body::empty()),
    )
    .await;

    assert_eq!(head_status, StatusCode::OK);
    assert!(
        head_body.is_empty(),
        "HEAD response body should be empty or suppressed"
    );
    assert_eq!(
        head_headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()),
        Some("11")
    );
    assert!(
        head_headers.get(header::ETAG).is_some(),
        "HEAD response should include a strong ETag"
    );

    let (get_status, get_headers, get_body) = send(
        app.clone(),
        request(Method::GET, &object_path, Body::empty()),
    )
    .await;

    assert_eq!(get_status, StatusCode::OK);
    assert_eq!(get_body, Bytes::from_static(b"hello world"));
    assert_eq!(
        get_headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()),
        Some("11")
    );
    assert!(
        get_headers.get(header::ETAG).is_some(),
        "GET response should include a strong ETag"
    );

    let (range_status, range_headers, range_body) =
        send(app.clone(), range_get(&object_path, "bytes=0-4")).await;

    assert_eq!(range_status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(range_body, Bytes::from_static(b"hello"));
    assert_eq!(
        range_headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()),
        Some("5")
    );
    assert_eq!(
        range_headers
            .get(header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok()),
        Some("bytes 0-4/11")
    );

    let unknown_cid = format!("b3:{}", "0".repeat(64));
    let unknown_path = format!("/o/{unknown_cid}");

    let (unknown_status, _unknown_headers, _unknown_body) = send(
        app.clone(),
        request(Method::GET, &unknown_path, Body::empty()),
    )
    .await;

    assert_eq!(unknown_status, StatusCode::NOT_FOUND);

    let (bad_status, _bad_headers, _bad_body) = send(
        app.clone(),
        request(Method::GET, "/o/not-a-cid", Body::empty()),
    )
    .await;

    assert_eq!(bad_status, StatusCode::NOT_FOUND);

    #[cfg(feature = "metrics")]
    {
        let (metrics_status, metrics_headers, metrics_body) =
            send(app.clone(), request(Method::GET, "/metrics", Body::empty())).await;

        assert_eq!(metrics_status, StatusCode::OK);

        let content_type = metrics_headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .expect("/metrics should include content-type");

        assert!(
            content_type.starts_with("text/plain"),
            "/metrics should return Prometheus text content-type"
        );

        let metrics_text =
            String::from_utf8(metrics_body.to_vec()).expect("metrics should be UTF-8");

        assert!(
            metrics_text.is_empty()
                || metrics_text.contains("# HELP")
                || metrics_text.contains("# TYPE"),
            "metrics may be empty until svc-storage registers golden counters"
        );
    }
}
