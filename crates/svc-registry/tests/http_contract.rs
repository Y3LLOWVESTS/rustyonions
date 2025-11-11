use std::sync::Arc;

use axum::{body, http::Request, Router};
use ron_kernel::HealthState;
use svc_registry::{
    build_info,
    config::model::Config,
    http::routes::registry_routes_with_cfg,
    observability::{
        endpoints::{admin_router, AdminState},
        metrics::RegistryMetrics,
    },
    storage::{inmem::InMemoryStore, RegistryStore},
};
use tower::util::ServiceExt; // for .oneshot

#[tokio::test]
async fn readiness_flips_200_when_gates_true() {
    // Build admin plane and flip gates like main.rs does.
    let metrics = RegistryMetrics::default();
    let health = Arc::new(HealthState::default());
    health.set("services_ok", true);
    health.set("queues_ok", true);

    let admin: Router = admin_router(AdminState {
        health,
        build: build_info::build_info(),
        metrics,
    });

    // GET /readyz -> 200 with our flips
    let res = admin
        .clone()
        .oneshot(Request::get("/readyz").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(res.status().is_success());

    // (Optional) also sanity-check /version and /healthz are reachable.
    let v = admin
        .clone()
        .oneshot(Request::get("/version").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(v.status().is_success());

    let h = admin
        .oneshot(Request::get("/healthz").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(h.status().is_success());
}

#[tokio::test]
async fn commit_bumps_head_and_returns_200() {
    let metrics = RegistryMetrics::default();
    let store = Arc::new(InMemoryStore::new());

    // Minimal config (routes only read the pieces they need)
    let cfg = Config::default();

    // Build API router
    let api: Router = registry_routes_with_cfg(metrics, store.clone(), &cfg);

    // Capture initial head
    let h0 = store.head().await;
    assert_eq!(h0.version, 0);

    // POST /registry/commit
    let req = Request::post("/registry/commit")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(r#"{"payload_b3":"b3:test"}"#))
        .unwrap();

    let res = api.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    // Head should bump to 1
    let h1 = store.head().await;
    assert_eq!(h1.version, 1);

    // And /registry/head returns the bumped version if we call through the router:
    let res = api
        .oneshot(Request::get("/registry/head").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(res.status().is_success());
    let body = body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains(r#""version":1"#));
}
