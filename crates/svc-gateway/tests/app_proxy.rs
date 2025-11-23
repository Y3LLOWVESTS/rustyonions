//! app_proxy.rs — integration test for `/app/*` → omnigate app plane.
//!
//! RO:WHAT  Spin up a dummy omnigate, then a real svc-gateway, and assert that
//!          `/app/ping` returns whatever omnigate returns.
//! RO:WHY   Proves env wiring (`SVC_GATEWAY_OMNIGATE_BASE_URL`) and proxy plumbing.

use std::net::SocketAddr;
use std::time::Duration;

use axum::{http::StatusCode, routing::get, Json, Router};
use serde_json::Value;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
use tokio::net::TcpListener;

/// Start a dummy omnigate app-plane server that answers `/v1/app/ping` with `{ "ok": true }`.
async fn start_dummy_omnigate() -> SocketAddr {
    async fn ping_handler() -> Json<Value> {
        Json(serde_json::json!({ "ok": true }))
    }

    let router = Router::new().route("/v1/app/ping", get(ping_handler));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy omnigate");
    let addr = listener.local_addr().expect("omnigate local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy omnigate serve");
    });

    addr
}

/// Happy-path roundtrip: `/app/ping` should be forwarded to omnigate and
/// return whatever omnigate replies.
///
/// This uses env vars to wire the base URL:
/// - `SVC_GATEWAY_OMNIGATE_BASE_URL`
/// - `SVC_GATEWAY_BIND_ADDR`
#[tokio::test]
async fn app_proxy_happy_path() {
    // 1) Start dummy omnigate.
    let omnigate_addr = start_dummy_omnigate().await;
    let omnigate_base = format!("http://{}", omnigate_addr);

    // 2) Configure gateway via env.
    std::env::set_var("SVC_GATEWAY_OMNIGATE_BASE_URL", &omnigate_base);
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    // 3) Build gateway state and router.
    let cfg = Config::load().expect("load config with env overrides");
    let metrics_handles = metrics::register().expect("register metrics");
    let state = AppState::new(cfg.clone(), metrics_handles);

    let router = routes::build_router(&state);

    // 4) Bind gateway listener (letting OS choose port).
    let listener = TcpListener::bind(&cfg.server.bind_addr)
        .await
        .expect("bind gateway");
    let gateway_addr = listener.local_addr().expect("gateway local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("gateway serve");
    });

    // Give both servers a moment to boot.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 5) Call /app/ping on the gateway and assert we get omnigate's response.
    let url = format!("http://{}/app/ping", gateway_addr);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .expect("gateway /app/ping response");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "expected 200 from gateway, got {}",
        resp.status()
    );

    let body: Value = resp.json().await.expect("parse JSON body");
    assert_eq!(body, serde_json::json!({ "ok": true }));
}
