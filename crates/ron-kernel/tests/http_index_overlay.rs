#![forbid(unsafe_code)]

use std::{net::SocketAddr, time::Duration};

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use ron_kernel::Metrics;
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, task::JoinHandle, time::sleep};

#[derive(Clone)]
struct TestState {
    metrics: std::sync::Arc<Metrics>,
    map: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>>,
}

#[derive(Deserialize)]
struct PutReq {
    addr: String,
    dir: String,
}

#[derive(Serialize, Deserialize)]
struct EchoResp {
    echo: String,
}

async fn overlay_echo(
    axum::extract::State(st): axum::extract::State<TestState>,
    Json(req): Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    let _t = st.metrics.request_latency_seconds.start_timer();
    // Simulate a tiny bit of work so the histogram records something > 0
    sleep(Duration::from_millis(2)).await;

    let payload = req
        .get("payload")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    (StatusCode::OK, Json(EchoResp { echo: payload }))
}

async fn index_put(
    axum::extract::State(st): axum::extract::State<TestState>,
    Json(req): Json<PutReq>,
) -> impl axum::response::IntoResponse {
    let _t = st.metrics.request_latency_seconds.start_timer();
    st.map.write().await.insert(req.addr, req.dir);
    (
        StatusCode::OK,
        Json(serde_json::json!({ "ok": true, "data": "ok" })),
    )
}

async fn index_resolve(
    axum::extract::State(st): axum::extract::State<TestState>,
    axum::extract::Path(addr): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    let _t = st.metrics.request_latency_seconds.start_timer();
    let g = st.map.read().await;
    if let Some(dir) = g.get(&addr) {
        (
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "data": { "addr": addr, "dir": dir } })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "ok": false, "error": "not found" })),
        )
    }
}

async fn serve_on_ephemeral(app: Router) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let h = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, h)
}

#[tokio::test]
async fn overlay_echo_roundtrip() {
    let m = std::sync::Arc::new(Metrics::new());
    m.health().set("test_overlay", true);

    let st = TestState {
        metrics: m.clone(),
        map: std::sync::Arc::new(tokio::sync::RwLock::new(Default::default())),
    };
    let app = Router::new().route("/echo", post(overlay_echo)).with_state(st);

    let (addr, task) = serve_on_ephemeral(app).await;

    // drive
    let client = reqwest::Client::new();
    let r = client
        .post(format!("http://{addr}/echo"))
        .json(&serde_json::json!({ "payload": "ping" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let v: EchoResp = r.json().await.unwrap();
    assert_eq!(v.echo, "ping");

    // histogram should be > 0
    assert!(m.request_latency_seconds.get_sample_count() >= 1);

    task.abort();
}

#[tokio::test]
async fn index_put_resolve_roundtrip() {
    let m = std::sync::Arc::new(Metrics::new());
    m.health().set("test_index", true);

    let st = TestState {
        metrics: m.clone(),
        map: std::sync::Arc::new(tokio::sync::RwLock::new(Default::default())),
    };
    let app = Router::new()
        .route("/put", post(index_put))
        .route("/resolve/:addr", get(index_resolve))
        .with_state(st);

    let (addr, task) = serve_on_ephemeral(app).await;

    // PUT a few entries
    let client = reqwest::Client::new();
    for i in 1..=3 {
        let r = client
            .post(format!("http://{addr}/put"))
            .json(&serde_json::json!({ "addr": format!("A{i}"), "dir": format!("B{i}") }))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
    }

    // RESOLVE one of them
    let r = reqwest::get(format!("http://{addr}/resolve/A2")).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let j: serde_json::Value = r.json().await.unwrap();
    assert_eq!(j["ok"], true);
    assert_eq!(j["data"]["addr"], "A2");
    assert_eq!(j["data"]["dir"], "B2");

    // We exercised histogram at least 4 times (put x3 + resolve x1)
    assert!(m.request_latency_seconds.get_sample_count() >= 4);

    task.abort();
}
