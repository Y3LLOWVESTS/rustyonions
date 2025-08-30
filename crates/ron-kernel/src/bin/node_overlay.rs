// crates/ron-kernel/src/bin/node_overlay.rs
#![forbid(unsafe_code)]

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::engine::general_purpose;
use base64::Engine as _;
use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent, Metrics};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{net::SocketAddr, sync::Arc};
use tracing::{info};
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[derive(Clone)]
struct AppState {
    metrics: Metrics,
    bus: Bus<KernelEvent>,
}

#[derive(Debug, Deserialize)]
struct OverlayReq {
    /// Payload as base64 (URL-safe not required; standard alphabet expected).
    payload_b64: String,
}

#[derive(Debug, Serialize)]
struct OverlayResp {
    /// Echo length in bytes (after decoding).
    len: usize,
    /// SHA-256 of the decoded payload in hex.
    sha256_hex: String,
    /// Re-encoded payload (base64, standard alphabet)
    payload_b64: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    println!("Starting node_overlay (overlay API + admin endpoints)…");

    // --- Bus + Metrics -------------------------------------------------------
    let bus: Bus<KernelEvent> = Bus::new(1024);
    let metrics = Metrics::new();
    let health = metrics.health().clone();

    // Mark overlay healthy at startup.
    health.set("overlay", true);

    // Serve admin endpoints (metrics/health/ready) on a fixed port for the demo.
    let admin_addr: SocketAddr = "127.0.0.1:9098".parse().expect("valid admin addr");
    let (_admin_task, bound_admin) = metrics.clone().serve(admin_addr).await;
    println!("Admin endpoints at http://{bound_admin}/ → /metrics /healthz /readyz");

    // App state
    let state = Arc::new(AppState { metrics: metrics.clone(), bus: bus.clone() });

    // Overlay API on a separate port.
    let api_addr: SocketAddr = "127.0.0.1:8087".parse().expect("valid api addr");
    let app = Router::new()
        .route("/", get(root_ok))
        .route("/echo", post(overlay_echo))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(api_addr).await.expect("bind overlay api");
    let api_bound = listener.local_addr().expect("read overlay addr");
    println!("Overlay API at http://{api_bound}/ → POST /echo");

    // Print bus events in background.
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            info!(?ev, "overlay: bus event");
        }
    });

    // Run server and wait for Ctrl-C in parallel.
    let server = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("overlay api server error: {e}");
        }
    });

    println!("Press Ctrl-C to shutdown overlay…");
    let _ = wait_for_ctrl_c().await;

    // Signal shutdown & flip health.
    let _ = bus.publish(KernelEvent::Shutdown);
    health.set("overlay", false);

    // Let server wind down (best-effort).
    let _ = server.await;
    println!("node_overlay exiting");
}

/// Simple readiness check for the root.
async fn root_ok() -> impl IntoResponse {
    StatusCode::OK
}

/// POST /echo
///
/// Accepts a JSON body with a `payload_b64` string. Decodes it using the modern
/// engine-based Base64 API, computes SHA-256, and re-encodes back to base64.
/// This replaces deprecated `base64::decode/encode` calls with:
///   - `general_purpose::STANDARD.decode(...)`
///   - `general_purpose::STANDARD.encode(...)`
async fn overlay_echo(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OverlayReq>,
) -> impl IntoResponse {
    // Modern base64 decode (was: base64::decode)
    let bytes = match general_purpose::STANDARD.decode(&req.payload_b64) {
        Ok(b) => b,
        Err(_) => {
            // B64 was not valid in the standard alphabet.
            return (StatusCode::BAD_REQUEST, "invalid base64 payload").into_response();
        }
    };

    // Observe a trivial latency sample for demo purposes
    state.metrics.req_latency.observe(0.002);

    // Compute SHA-256 and hex-encode it
    let sha256_hex = {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let digest = hasher.finalize();
        hex::encode(digest)
    };

    // Modern base64 encode (was: base64::encode)
    let b64 = general_purpose::STANDARD.encode(&bytes);

    // Emit a bus health ping for visibility
    let _ = state.bus.publish(KernelEvent::Health {
        service: "overlay".to_string(),
        ok: true,
    });

    let resp = OverlayResp {
        len: bytes.len(),
        sha256_hex,
        payload_b64: b64,
    };

    (StatusCode::OK, Json(resp)).into_response()
}
