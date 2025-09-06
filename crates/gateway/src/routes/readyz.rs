// crates/gateway/src/routes/readyz.rs
#![forbid(unsafe_code)]

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::{env, path::PathBuf, time::Duration};
use tokio::{net::UnixStream, time::timeout};

/// Liveness: if the process is up, return 200.
pub async fn healthz() -> Response {
    (StatusCode::OK, "ok").into_response()
}

#[derive(Serialize)]
struct ReadyReport {
    ok: bool,
    overlay_ok: bool,
    index_ok: Option<bool>,
    storage_ok: Option<bool>,
    overlay_sock: Option<String>,
    index_sock: Option<String>,
    storage_sock: Option<String>,
}

/// Readiness succeeds only if overlay is reachable; if RON_INDEX_SOCK/RON_STORAGE_SOCK are
/// configured, they must be reachable as well.
pub async fn readyz() -> Response {
    let overlay_sock = env::var("RON_OVERLAY_SOCK").ok().map(PathBuf::from);
    let index_sock = env::var("RON_INDEX_SOCK").ok().map(PathBuf::from);
    let storage_sock = env::var("RON_STORAGE_SOCK").ok().map(PathBuf::from);

    let mut overlay_ok = false;
    let mut index_ok = None;
    let mut storage_ok = None;

    if let Some(ref p) = overlay_sock {
        overlay_ok = connect_ok(p).await;
    }
    if let Some(ref p) = index_sock {
        index_ok = Some(connect_ok(p).await);
    }
    if let Some(ref p) = storage_sock {
        storage_ok = Some(connect_ok(p).await);
    }

    let ok = overlay_ok && index_ok.unwrap_or(true) && storage_ok.unwrap_or(true);

    let report = ReadyReport {
        ok,
        overlay_ok,
        index_ok,
        storage_ok,
        overlay_sock: overlay_sock.as_ref().map(|p| p.display().to_string()),
        index_sock: index_sock.as_ref().map(|p| p.display().to_string()),
        storage_sock: storage_sock.as_ref().map(|p| p.display().to_string()),
    };

    if ok {
        (StatusCode::OK, Json(report)).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(report)).into_response()
    }
}

async fn connect_ok(path: &PathBuf) -> bool {
    timeout(Duration::from_millis(300), UnixStream::connect(path))
        .await
        .is_ok()
}
