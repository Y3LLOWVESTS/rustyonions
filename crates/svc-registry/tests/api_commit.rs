use std::sync::Arc;

use axum::{body, http::Request, Router};
use svc_registry::{
    config::model::Config,
    http::routes::registry_routes_with_cfg,
    observability::metrics::RegistryMetrics,
    storage::inmem::InMemoryStore,
};
use tower::util::ServiceExt; // for .oneshot

#[tokio::test]
async fn commit_endpoint_roundtrip_and_body_shape() {
    let metrics = RegistryMetrics::default();
    let store = Arc::new(InMemoryStore::new());
    let cfg = Config::default();
    let api: Router = registry_routes_with_cfg(metrics, store.clone(), &cfg);

    // Commit once
    let req = Request::post("/registry/commit")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(r#"{"payload_b3":"b3:roundtrip"}"#))
        .unwrap();

    let res = api.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let body = body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains(r#""version":1"#));
    assert!(s.contains(r#""payload_b3":"b3:roundtrip""#));

    // GET head via router to ensure JSON shape is stable
    let res = api
        .oneshot(
            Request::get("/registry/head")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_success());
    let body = body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains(r#""version":1"#));
}
