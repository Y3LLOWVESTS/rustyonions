// crates/gateway/src/routes.rs
#![forbid(unsafe_code)]

use crate::pay_enforce;
use crate::state::AppState;
use crate::utils::basic_headers;

use axum::{
    extract::{Extension, Path},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tracing::{error, info};

/// Build a STATELESS router (Router<()>).
/// We inject AppState later at the server entry via a service wrapper.
pub fn router() -> Router<()> {
    Router::new().route("/o/:addr/*tail", get(serve_object))
}

/// GET /o/:addr/*tail — fetch bytes via svc-overlay.
///
/// NOTE: We use `Extension<AppState>` instead of `State<AppState>` because
/// we inject the state into request extensions in main.rs.
pub async fn serve_object(
    Extension(state): Extension<AppState>,
    Path((addr_in, tail)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    // Normalize: allow "<hex>.<tld>" or "b3:<hex>.<tld>".
    // Clone so we can still log addr_in later (fixes earlier E0382).
    let addr = if addr_in.contains(':') {
        addr_in.clone()
    } else {
        format!("b3:{addr_in}")
    };
    let rel = if tail.is_empty() { "payload.bin" } else { tail.as_str() };

    info!(%addr_in, %addr, %rel, "gateway request");

    // Optional payment guard via Manifest.toml (best-effort).
    if state.enforce_payments {
        if let Ok(Some(manifest)) = state.overlay.get_bytes(&addr, "Manifest.toml") {
            if let Err((_code, rsp)) = pay_enforce::guard_bytes(&manifest) {
                return Ok(rsp);
            }
        }
    }

    // Fetch file through overlay.
    match state.overlay.get_bytes(&addr, rel) {
        Ok(Some(bytes)) => {
            let ctype = guess_ct(rel);
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
        "txt" | "toml" => "text/plain; charset=utf-8",
        "pdf" => "application/pdf",
        "wasm" | "bin" => "application/octet-stream",
        _ => "application/octet-stream",
    }
}
