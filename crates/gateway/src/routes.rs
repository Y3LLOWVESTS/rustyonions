#![forbid(unsafe_code)]

use crate::pay_enforce;
use crate::state::AppState;
use crate::utils::basic_headers;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tracing::error;

/// Build router with overlay-backed serving.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/o/:addr/*tail", get(serve_object))
        .with_state(state)
}

/// GET /o/:addr/*tail — fetch bytes via svc-overlay (no direct FS/DB).
async fn serve_object(
    State(state): State<AppState>,
    Path((addr, tail)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    let rel = if tail.is_empty() {
        "payload.bin"
    } else {
        tail.as_str()
    };

    // 1) Optional payment guard from Manifest.toml (best-effort).
    if state.enforce_payments {
        if let Ok(Some(manifest)) = state.overlay.get_bytes(&addr, "Manifest.toml") {
            if let Err((_code, rsp)) = pay_enforce::guard_bytes(&manifest) {
                // We can’t return a body via Err(StatusCode) (axum will drop it),
                // so emit the full response (with 402 status) here.
                return Ok(rsp);
            }
        }
    }

    // 2) Fetch actual file bytes through overlay.
    match state.overlay.get_bytes(&addr, rel) {
        Ok(Some(bytes)) => {
            let ctype = guess_ct(rel);
            // utils::basic_headers(content_type, etag_b3, content_encoding)
            let mut headers: HeaderMap = basic_headers(ctype, Some(&addr), None);
            headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
            Ok((StatusCode::OK, headers, bytes).into_response())
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!(error=?e, %addr, %rel, "overlay get error");
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

fn guess_ct(rel: &str) -> &'static str {
    match rel.rsplit('.').next().unwrap_or_default() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "txt" => "text/plain; charset=utf-8",
        "toml" => "text/plain; charset=utf-8",
        "pdf" => "application/pdf",
        "wasm" => "application/wasm",
        "bin" => "application/octet-stream",
        _ => "application/octet-stream",
    }
}
