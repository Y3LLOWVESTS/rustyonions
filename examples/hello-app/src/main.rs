//! examples/hello-app/src/main.rs
//!
//! RO:WHAT — Tiny Axum workload for the Micronode proxy facet demo.
//! RO:WHY  — Proves end-to-end: proxying, metrics movement, reload, crash/restart.
//! RO:DX   — Intentionally boring + deterministic.
//!
//! Endpoints (upstream):
//!   GET  /                -> HTML (banner + counters)
//!   GET  /api/counter      -> increments counter, returns JSON
//!   POST /admin/reload     -> bumps reload counter + changes banner
//!   POST /admin/crash      -> exits the process (deterministic)
//!   GET  /healthz          -> ok

#![forbid(unsafe_code)]

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::signal;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    counter: Arc<AtomicU64>,
    reloads: Arc<AtomicU64>,
}

#[derive(Debug, Serialize)]
struct CounterResp {
    counter: u64,
    reloads: u64,
    banner: String,
    pid: u32,
}

#[derive(Debug, Deserialize)]
struct ReloadReq {
    /// Optional custom banner string.
    banner: Option<String>,
}

fn banner_for(reloads: u64, override_banner: Option<&str>) -> String {
    if let Some(b) = override_banner {
        return b.to_string();
    }
    // Deterministic toggle so it’s easy to see the change without clocks.
    if reloads % 2 == 0 {
        "HELLO-APP (banner A)".to_string()
    } else {
        "HELLO-APP (banner B)".to_string()
    }
}

async fn healthz() -> &'static str {
    "ok"
}

async fn root(State(st): State<AppState>) -> impl IntoResponse {
    let counter = st.counter.load(Ordering::Relaxed);
    let reloads = st.reloads.load(Ordering::Relaxed);
    let banner = banner_for(reloads, None);

    let pid = std::process::id();

    let html = format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width,initial-scale=1" />
    <title>hello-app</title>
    <style>
      body {{ font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial; padding: 24px; }}
      .pill {{ display:inline-block; padding: 2px 10px; border:1px solid #ddd; border-radius: 999px; margin-right: 8px; }}
      button {{ padding: 10px 14px; border-radius: 10px; border: 1px solid #ddd; cursor: pointer; }}
      pre {{ background:#111; color:#eee; padding: 12px; border-radius: 12px; overflow:auto; }}
    </style>
  </head>
  <body>
    <div class="pill">pid: {pid}</div>
    <div class="pill">reloads: {reloads}</div>
    <div class="pill">counter: {counter}</div>

    <h1>{banner}</h1>

    <p>This is the upstream workload. When proxied through micronode, you should see traffic + restarts in metrics.</p>

    <div style="display:flex; gap: 10px; margin: 16px 0;">
      <button onclick="fetch('/api/counter').then(r=>r.json()).then(j=>{{
        document.getElementById('out').textContent = JSON.stringify(j, null, 2);
      }})">GET /api/counter</button>

      <button onclick="fetch('/admin/reload', {{method:'POST', headers:{{'content-type':'application/json'}}, body:'{{}}'}}).then(r=>r.json()).then(j=>{{
        document.getElementById('out').textContent = JSON.stringify(j, null, 2);
        location.reload();
      }})">POST /admin/reload</button>

      <button onclick="fetch('/admin/crash', {{method:'POST'}}).then(r=>r.text()).then(t=>{{
        document.getElementById('out').textContent = t;
      }})">POST /admin/crash</button>
    </div>

    <pre id="out">click a button...</pre>
  </body>
</html>"#
    );

    Html(html)
}

async fn api_counter(State(st): State<AppState>) -> impl IntoResponse {
    let counter = st.counter.fetch_add(1, Ordering::Relaxed) + 1;
    let reloads = st.reloads.load(Ordering::Relaxed);
    let banner = banner_for(reloads, None);

    Json(CounterResp {
        counter,
        reloads,
        banner,
        pid: std::process::id(),
    })
}

async fn admin_reload(State(st): State<AppState>, Json(req): Json<ReloadReq>) -> impl IntoResponse {
    let reloads = st.reloads.fetch_add(1, Ordering::Relaxed) + 1;
    let banner = banner_for(reloads, req.banner.as_deref());

    Json(CounterResp {
        counter: st.counter.load(Ordering::Relaxed),
        reloads,
        banner,
        pid: std::process::id(),
    })
}

async fn admin_crash() -> impl IntoResponse {
    // Return a response, then exit shortly after so the proxy/demo can observe downtime.
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        warn!("hello-app exiting intentionally (admin/crash)");
        std::process::exit(42);
    });

    (StatusCode::OK, "crashing now\n")
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let bind: SocketAddr = std::env::var("HELLO_APP_BIND")
        .unwrap_or_else(|_| "127.0.0.1:5401".to_string())
        .parse()
        .expect("HELLO_APP_BIND SocketAddr");

    let st = AppState {
        counter: Arc::new(AtomicU64::new(0)),
        reloads: Arc::new(AtomicU64::new(0)),
    };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/", get(root))
        .route("/api/counter", get(api_counter))
        .route("/admin/reload", post(admin_reload))
        .route("/admin/crash", post(admin_crash))
        .with_state(st);

    info!("hello-app starting on http://{bind}");
    let listener = tokio::net::TcpListener::bind(bind).await.expect("bind");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = signal::ctrl_c().await;
        })
        .await
        .ok();
}
