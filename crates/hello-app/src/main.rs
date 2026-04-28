// crates/hello-app/src/main.rs
//
// WHAT:
//   Tiny Axum workload for the “Managed HTTP App” demo.
// WHY:
//   Proves proxying + reload + crash (later: supervision) without touching core.
// INVARIANTS:
//   - No locks held across .await
//   - Deterministic admin endpoints for demo automation
//   - Default bind matches micronode dev facet upstream (127.0.0.1:5401)

#![forbid(unsafe_code)]

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use parking_lot::RwLock;
use serde::Serialize;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    counter: Arc<AtomicU64>,
    reload_gen: Arc<AtomicU64>,
    banner: Arc<RwLock<String>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            counter: Arc::new(AtomicU64::new(0)),
            reload_gen: Arc::new(AtomicU64::new(0)),
            banner: Arc::new(RwLock::new("hello-app: initial banner".to_string())),
        }
    }
}

#[derive(Debug, Serialize)]
struct CounterResp {
    counter: u64,
    reload_gen: u64,
    banner: String,
    unix_ms: u128,
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let bind: SocketAddr = std::env::var("HELLO_APP_BIND")
        .unwrap_or_else(|_| "127.0.0.1:5401".to_string())
        .parse()
        .expect("HELLO_APP_BIND must be a SocketAddr");

    let st = AppState::new();

    let app = Router::new()
        .route("/", get(root))
        .route("/api/counter", get(api_counter))
        .route("/admin/reload", post(admin_reload))
        .route("/admin/crash", post(admin_crash))
        .with_state(st)
        .layer(TraceLayer::new_for_http());

    info!("hello-app starting on http://{bind}");
    let listener = tokio::net::TcpListener::bind(bind).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

async fn root(State(st): State<AppState>) -> impl IntoResponse {
    let counter = st.counter.load(Ordering::Relaxed);
    let reload_gen = st.reload_gen.load(Ordering::Relaxed);
    let banner = st.banner.read().clone();

    let mut headers = HeaderMap::new();
    headers.insert("cache-control", "no-store".parse().unwrap());
    headers.insert(
        "x-hello-reload-gen",
        reload_gen.to_string().parse().unwrap(),
    );

    // NOTE: this is a Rust `format!` string, so any JS braces must be escaped:
    //   {  -> {{
    //   }  -> }}
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>hello-app</title>
  <style>
    body {{ font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Arial; margin: 32px; }}
    .pill {{ display:inline-block; padding:2px 8px; border-radius:999px; background:#eee; }}
    .row {{ display:flex; gap:16px; flex-wrap:wrap; margin-top:12px; }}
    .card {{ padding:12px 14px; border:1px solid #ddd; border-radius:12px; min-width:240px; }}
    code {{ background:#f6f6f6; padding:2px 6px; border-radius:8px; }}
    button {{ padding:10px 14px; border-radius:12px; border:1px solid #ddd; background:#fafafa; cursor:pointer; }}
    button:hover {{ background:#f0f0f0; }}
  </style>
</head>
<body>
  <h1>hello-app <span class="pill">workload</span></h1>
  <p><strong>Banner:</strong> {banner}</p>

  <div class="row">
    <div class="card"><div>Counter</div><h2>{counter}</h2></div>
    <div class="card"><div>Reload Gen</div><h2>{reload_gen}</h2></div>
  </div>

  <h3>Endpoints</h3>
  <ul>
    <li><code>GET /api/counter</code> increments counter and returns JSON</li>
    <li><code>POST /admin/reload</code> bumps reload gen + changes banner</li>
    <li><code>POST /admin/crash</code> exits (for supervision demo)</li>
  </ul>

  <div class="row">
    <button onclick="fetch('/api/counter').then(r=>r.json()).then(j=>{{console.log(j); location.reload();}})">Hit /api/counter</button>
    <button onclick="fetch('/admin/reload',{{method:'POST'}}).then(()=>location.reload())">POST /admin/reload</button>
  </div>
</body>
</html>"#
    );

    (headers, Html(html))
}

async fn api_counter(State(st): State<AppState>) -> impl IntoResponse {
    let next = st.counter.fetch_add(1, Ordering::Relaxed) + 1;
    let reload_gen = st.reload_gen.load(Ordering::Relaxed);
    let banner = st.banner.read().clone();

    let mut headers = HeaderMap::new();
    headers.insert("cache-control", "no-store".parse().unwrap());
    headers.insert(
        "x-hello-reload-gen",
        reload_gen.to_string().parse().unwrap(),
    );

    (
        headers,
        Json(CounterResp {
            counter: next,
            reload_gen,
            banner,
            unix_ms: unix_ms(),
        }),
    )
}

async fn admin_reload(State(st): State<AppState>) -> impl IntoResponse {
    let gen = st.reload_gen.fetch_add(1, Ordering::Relaxed) + 1;

    // Short lock, no await.
    {
        let mut b = st.banner.write();
        *b = format!("hello-app: reloaded gen={gen} @ {}ms", unix_ms());
    }

    let mut headers = HeaderMap::new();
    headers.insert("cache-control", "no-store".parse().unwrap());
    headers.insert("x-hello-reload-gen", gen.to_string().parse().unwrap());

    (
        StatusCode::OK,
        headers,
        Json(serde_json::json!({ "ok": true, "reload_gen": gen })),
    )
}

async fn admin_crash() -> Response {
    // Deterministic “crash” for supervision demo.
    warn!("hello-app crash requested: exiting with code 42");
    std::process::exit(42);

    // Unreachable, but keeps the return type explicit and future-proof.
    #[allow(unreachable_code)]
    (StatusCode::INTERNAL_SERVER_ERROR, "unreachable").into_response()
}
