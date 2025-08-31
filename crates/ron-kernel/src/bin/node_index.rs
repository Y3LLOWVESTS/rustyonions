#![forbid(unsafe_code)]

use std::{collections::HashMap, error::Error, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use prometheus::HistogramTimer;
use ron_kernel::{wait_for_ctrl_c, Metrics};
use serde::Serialize;
use tokio::{net::TcpListener, sync::RwLock, time::sleep};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct IndexState {
    inner: Arc<RwLock<HashMap<String, String>>>, // addr -> dir
    metrics: Arc<Metrics>,
}

#[derive(Debug, Serialize)]
struct ApiResp<T: Serialize> {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .with_target(false)
        .compact()
        .init();

    // === Metrics / Admin server (index uses 9097 to avoid conflicts) ===
    let admin_addr: SocketAddr = "127.0.0.1:9097".parse()?;
    let metrics0 = Metrics::new();
    let (_admin_task, bound) = metrics0.clone().serve(admin_addr).await?;
    info!("Admin endpoints: /metrics /healthz /readyz at http://{bound}/");

    let metrics = Arc::new(metrics0);
    metrics.health().set("node_index", true);

    // === App server (index API on 8086) ===
    let app_addr: SocketAddr = "127.0.0.1:8086".parse()?;
    let listener = TcpListener::bind(app_addr).await?;

    let state = IndexState {
        inner: Arc::new(RwLock::new(HashMap::new())),
        metrics: metrics.clone(),
    };

    let app = Router::new()
        .route("/put", post(put))
        .route("/resolve/:addr", get(resolve))
        .with_state(state);

    info!(
        "node_index listening on http://{}/ (POST /put, GET /resolve/:addr)",
        app_addr
    );

    // Graceful shutdown on Ctrl-C
    let shutdown = async {
        let _ = wait_for_ctrl_c().await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    metrics.health().set("node_index", false);
    Ok(())
}

#[derive(serde::Deserialize)]
struct PutReq {
    addr: String,
    dir: String,
}

async fn put(State(state): State<IndexState>, Json(req): Json<PutReq>) -> impl IntoResponse {
    let _t: HistogramTimer = state.metrics.request_latency_seconds.start_timer();

    // Simulate small work so histogram isn't zero
    sleep(Duration::from_millis(2)).await;

    state.inner.write().await.insert(req.addr, req.dir);
    let resp: ApiResp<&'static str> = ApiResp {
        ok: true,
        data: Some("ok"),
        error: None,
    };
    (StatusCode::OK, Json(resp)).into_response()
}

async fn resolve(State(state): State<IndexState>, Path(addr): Path<String>) -> impl IntoResponse {
    let _t: HistogramTimer = state.metrics.request_latency_seconds.start_timer();

    // Simulate small work
    sleep(Duration::from_millis(2)).await;

    let map = state.inner.read().await;
    if let Some(dir) = map.get(&addr) {
        let resp: ApiResp<_> = ApiResp {
            ok: true,
            data: Some(serde_json::json!({ "addr": addr, "dir": dir })),
            error: None,
        };
        (StatusCode::OK, Json(resp)).into_response()
    } else {
        let resp: ApiResp<()> = ApiResp {
            ok: false,
            data: None,
            error: Some("not found".into()),
        };
        (StatusCode::NOT_FOUND, Json(resp)).into_response()
    }
}
