//! Test support: minimal Axum gateway that accepts any OAP-ish POST and
//! returns controlled JSON bodies / statuses for client-side verification.
//!
//! RO:WHY — Lets us prove end-to-end OAP calls (happy path + error mapping)
//! without coupling tests to real services.
//! RO:INVARIANTS —
//! - Binds ephemeral 127.0.0.1:0 and exposes its chosen port.
//! - Wildcard POST /*path; handlers can branch by request path.
//! - Helpers to reply with fixed JSON bodies that match planes' expectations.

use axum::{
    body::Bytes as AxumBytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

#[derive(Clone, Default)]
pub struct AppState {
    // You can stash toggles/fixtures here if needed later
}

#[derive(Serialize, Deserialize)]
struct BlobResp<'a> {
    // Using serde_bytes encoding semantics on client; here we just send base64 via serde_json.
    blob: &'a [u8],
}

#[derive(Serialize, Deserialize)]
struct PutResp<'a> {
    addr_b3: &'a str,
}

pub struct Running {
    pub addr: SocketAddr,
    _state: Arc<AppState>,
    _task: tokio::task::JoinHandle<()>,
}

pub async fn start() -> Running {
    let state = Arc::new(AppState::default());

    // Wildcard POST endpoint
    let app = Router::new()
        .route("/*path", post(handle_post))
        .with_state(state.clone());

    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();

    let task = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("[mock-gateway] serve error: {err}");
        }
    });

    Running {
        addr,
        _state: state,
        _task: task,
    }
}

async fn handle_post(
    State(_state): State<Arc<AppState>>,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: AxumBytes,
) -> Response {
    // Basic "capability present" check — we don't validate value here.
    let cap_present = headers
        .keys()
        .any(|k| k.as_str().eq_ignore_ascii_case("cap"));

    // Exercise size-cap response if body is absurdly huge (we still accept; client enforces cap itself).
    let _bytes_len = body.len();

    // Route-based canned replies, matching planes documented in carry-over notes.
    match path.as_str() {
        // Storage GET returns {"blob": <bytes>}
        p if p.ends_with("/storage/get") => {
            if !cap_present {
                return (StatusCode::UNAUTHORIZED, "missing cap").into_response();
            }
            let resp = BlobResp { blob: b"hello-oap" };
            (StatusCode::OK, Json(resp)).into_response()
        }
        // Storage PUT returns {"addr_b3":"b3:<64 hex>"}
        p if p.ends_with("/storage/put") => {
            if !cap_present {
                return (StatusCode::UNAUTHORIZED, "missing cap").into_response();
            }
            let resp = PutResp {
                addr_b3: "b3:0000000000000000000000000000000000000000000000000000000000000000",
            };
            (StatusCode::OK, Json(resp)).into_response()
        }
        // Edge GET returns {"blob": <bytes>}
        p if p.ends_with("/edge/get") => {
            if !cap_present {
                return (StatusCode::UNAUTHORIZED, "missing cap").into_response();
            }
            let resp = BlobResp { blob: b"edge-bytes" };
            (StatusCode::OK, Json(resp)).into_response()
        }
        // Index resolve returns {"addr_b3": "..."}
        p if p.ends_with("/index/resolve") => {
            if !cap_present {
                return (StatusCode::UNAUTHORIZED, "missing cap").into_response();
            }
            let resp = PutResp {
                addr_b3: "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            };
            (StatusCode::OK, Json(resp)).into_response()
        }
        // Unknown path → 404 (to test mapping)
        _ => (StatusCode::NOT_FOUND, "no route").into_response(),
    }
}
