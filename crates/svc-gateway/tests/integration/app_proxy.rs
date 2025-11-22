#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

use std::net::SocketAddr;
use std::time::Duration;

use axum::{routing::get, Json, Router};
use reqwest::StatusCode;
use serde_json::json;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
use tokio::net::TcpListener;
use tokio::time::sleep;

/// Build a tiny dummy omnigate app exposing `/v1/app/ping`.
fn dummy_omnigate_router() -> Router {
    async fn ping_handler() -> Json<serde_json::Value> {
        Json(json!({
            "ok": true,
            "from": "dummy-omnigate",
        }))
    }

    Router::new().route("/v1/app/ping", get(ping_handler))
}

/// Helper: bind a TCP listener on 127.0.0.1:0 and return (addr, listener).
async fn bind_ephemeral() -> (SocketAddr, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral address");
    let addr = listener.local_addr().expect("local_addr");
    (addr, listener)
}

/// Happy-path: gateway `/app/ping` forwards to dummy omnigate `/v1/app/ping`.
#[tokio::test(flavor = "multi_thread")]
async fn app_proxy_happy_path() {
    // 1) Start dummy omnigate on an ephemeral port.
    let (omni_addr, omni_listener) = bind_ephemeral().await;
    let omni_app = dummy_omnigate_router();

    tokio::spawn(async move {
        axum::serve(omni_listener, omni_app)
            .await
            .expect("serve dummy omnigate");
    });

    // Give the dummy server a brief moment to start accepting.
    sleep(Duration::from_millis(50)).await;

    // 2) Point the gateway at the dummy omnigate via env override.
    //
    // AppState::new() will prefer `SVC_GATEWAY_OMNIGATE_BASE` if set.
    let omni_base = format!("http://{omni_addr}");
    std::env::set_var("SVC_GATEWAY_OMNIGATE_BASE", &omni_base);

    // 3) Build a fresh Config + MetricsHandles + AppState for the gateway.
    let mut cfg = Config::default();
    // Avoid port conflicts by binding the gateway itself on an ephemeral port.
    cfg.server.bind_addr = "127.0.0.1:0".parse().expect("parse bind_addr");
    cfg.server.metrics_addr = "127.0.0.1:0".parse().expect("parse metrics_addr");

    let metrics_handles = metrics::register().expect("register metrics");
    let state = AppState::new(cfg.clone(), metrics_handles);

    // 4) Build the gateway router and bind it on an ephemeral port.
    let router = routes::build_router(&state);

    let (gw_addr, gw_listener) = {
        let listener = TcpListener::bind(cfg.server.bind_addr)
            .await
            .expect("bind gateway");
        let addr = listener.local_addr().expect("gateway local_addr");
        (addr, listener)
    };

    tokio::spawn(async move {
        axum::serve(gw_listener, router)
            .await
            .expect("serve gateway");
    });

    // Small delay to let the gateway start.
    sleep(Duration::from_millis(50)).await;

    // 5) Issue a request to the gateway app plane and assert success.
    let client = reqwest::Client::new();
    let url = format!("http://{gw_addr}/app/ping");
    let resp = client
        .get(&url)
        .send()
        .await
        .expect("gateway /app/ping response");

    assert_eq!(resp.status(), StatusCode::OK, "expected 200 from gateway");

    let body: serde_json::Value = resp.json().await.expect("parse JSON body");
    assert_eq!(body["ok"], json!(true));
    assert_eq!(body["from"], json!("dummy-omnigate"));
}
