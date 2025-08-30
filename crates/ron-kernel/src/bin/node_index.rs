// crates/ron-kernel/src/bin/node_index.rs
#![forbid(unsafe_code)]

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent, Metrics};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone, Default)]
struct IndexState {
    inner: Arc<RwLock<HashMap<String, String>>>, // addr -> dir
}

#[derive(Deserialize)]
struct PutReq {
    addr: String,
    dir: String,
}

#[derive(Serialize)]
#[serde(tag = "ok", content = "data")]
enum ResolveResp {
    #[serde(rename = "true")]
    Found { dir: String },
    #[serde(rename = "false")]
    Err { msg: String },
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    println!("Starting node_index (HTTP + metrics + bus)…");

    // Metrics / admin HTTP
    let metrics = Metrics::new();
    let admin_addr: SocketAddr = "127.0.0.1:9096".parse().unwrap(); // distinct port
    let (admin_handle, bound_admin) = metrics.clone().serve(admin_addr).await;
    println!(
        "Admin endpoints: /metrics /healthz /readyz at http://{}/",
        bound_admin
    );

    // Mark service healthy in readiness map
    let health = metrics.health().clone();
    health.set("index", true);

    // Bus
    let bus: Bus<KernelEvent> = Bus::new(128);
    let mut sub = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = sub.recv().await {
            info!(?ev, "bus event");
        }
    });

    // App state
    let st = IndexState::default();

    // REST API
    let app = Router::new()
        .route("/put", post(put))
        .route("/resolve/:addr", get(resolve))
        .with_state(st.clone());

    // Serve API on its own port
    let api_addr: SocketAddr = "127.0.0.1:8086".parse().unwrap();
    let api = axum::serve(
        tokio::net::TcpListener::bind(api_addr).await.unwrap(),
        app.into_make_service(),
    );
    println!("Index API at http://{}/ (POST /put, GET /resolve/:addr)", api_addr);

    // Health beats keep readiness fresh (optional but nice)
    let hb_bus = bus.clone();
    let hb_health = health.clone();
    tokio::spawn(async move {
        loop {
            let _ = hb_bus.publish(KernelEvent::Health {
                service: "index".into(),
                ok: true,
            });
            hb_health.set("index", true);
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    println!("Press Ctrl-C to shutdown…");
    tokio::select! {
        _ = api => {},
        _ = wait_for_ctrl_c() => {},
    }

    let _ = bus.publish(KernelEvent::Shutdown);
    admin_handle.abort();
    println!("node_index exiting");
}

async fn put(
    State(st): State<IndexState>,
    Json(req): Json<PutReq>,
) -> impl IntoResponse {
    st.inner.write().await.insert(req.addr, req.dir);
    (StatusCode::OK, "ok")
}

async fn resolve(State(st): State<IndexState>, Path(addr): Path<String>) -> impl IntoResponse {
    let map = st.inner.read().await;
    if let Some(dir) = map.get(&addr) {
        (StatusCode::OK, Json(ResolveResp::Found { dir: dir.clone() }))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ResolveResp::Err {
                msg: "not found".into(),
            }),
        )
    }
}
