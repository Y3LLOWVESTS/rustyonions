use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::http::extractors::AppState;

/// b3:<64 lowercase hex>
#[inline]
fn is_valid_cid(cid: &str) -> bool {
    if cid.len() != 3 + 64 {
        return false;
    }
    if !cid.starts_with("b3:") {
        return false;
    }
    cid[3..]
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[inline]
fn ensure_quoted_etag(etag: &str) -> String {
    if etag.len() >= 2 && etag.starts_with('"') && etag.ends_with('"') {
        etag.to_string()
    } else {
        format!("\"{etag}\"")
    }
}

pub async fn handler(State(app): State<AppState>, Path(cid): Path<String>) -> impl IntoResponse {
    // Malformed CID → 400
    if !is_valid_cid(&cid) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    match app.store.head(&cid).await {
        Ok(meta) => {
            let mut headers = HeaderMap::new();

            // Strong ETag (quoted hex)
            let quoted = ensure_quoted_etag(&meta.etag);
            headers.insert(header::ETAG, HeaderValue::from_str(&quoted).unwrap());

            // Exact length
            headers.insert(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&meta.len.to_string()).unwrap(),
            );

            (StatusCode::OK, headers).into_response()
        }
        // Well-formed but unknown → 404
        Err(_) => (StatusCode::NOT_FOUND, ()).into_response(),
    }
}
