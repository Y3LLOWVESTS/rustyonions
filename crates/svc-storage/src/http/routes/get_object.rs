use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};

use crate::http::extractors::AppState;

fn parse_range_bytes(range_header: &str, total_len: u64) -> Option<(u64, u64)> {
    // Support the simple form used by our smoke test: "bytes=START-END"
    // Also accept "bytes=START-" and "bytes=-SUFFIX".
    let s = range_header.trim();
    if !s.starts_with("bytes=") {
        return None;
    }
    let spec = &s[6..];
    // Only support a single range
    if let Some((a, b)) = spec.split_once('-') {
        match (a.trim(), b.trim()) {
            // bytes=START-END
            (a, b) if !a.is_empty() && !b.is_empty() => {
                let start: u64 = a.parse().ok()?;
                let end: u64 = b.parse().ok()?;
                if start <= end && end < total_len {
                    Some((start, end))
                } else {
                    None
                }
            }
            // bytes=START-
            (a, b) if !a.is_empty() && b.is_empty() => {
                let start: u64 = a.parse().ok()?;
                if start < total_len {
                    Some((start, total_len.saturating_sub(1)))
                } else {
                    None
                }
            }
            // bytes=-SUFFIX  (last N bytes)
            (a, b) if a.is_empty() && !b.is_empty() => {
                let suffix: u64 = b.parse().ok()?;
                if suffix == 0 {
                    None
                } else if suffix >= total_len {
                    Some((0, total_len.saturating_sub(1)))
                } else {
                    Some((total_len - suffix, total_len - 1))
                }
            }
            _ => None,
        }
    } else {
        None
    }
}

pub async fn handler(
    State(app): State<AppState>,
    Path(cid): Path<String>,
    headers_in: HeaderMap,
) -> Response {
    // HEAD meta first (for both full and range)
    let meta = match app.store.head(&cid).await {
        Ok(m) => m,
        Err(_) => return (StatusCode::NOT_FOUND, ()).into_response(),
    };

    // Optional Range: parse manually so we don't depend on headers crate API quirks
    if let Some(hv) = headers_in.get(axum::http::header::RANGE) {
        if let Ok(hs) = hv.to_str() {
            if let Some((start, end_inclusive)) = parse_range_bytes(hs, meta.len) {
                match app.store.get_range(&cid, start, end_inclusive).await {
                    Ok((chunk, _total_len)) => {
                        let mut headers = HeaderMap::new();
                        headers.insert(
                            axum::http::header::ETAG,
                            HeaderValue::from_str(&meta.etag).unwrap(),
                        );
                        headers.insert(
                            axum::http::header::CONTENT_LENGTH,
                            HeaderValue::from_str(&chunk.len().to_string()).unwrap(),
                        );
                        headers.insert(
                            axum::http::header::CONTENT_RANGE,
                            HeaderValue::from_str(&format!(
                                "bytes {start}-{end_inclusive}/{}",
                                meta.len
                            ))
                            .unwrap(),
                        );
                        return (StatusCode::PARTIAL_CONTENT, headers, chunk).into_response();
                    }
                    Err(_) => return (StatusCode::NOT_FOUND, ()).into_response(),
                }
            } else {
                return (StatusCode::RANGE_NOT_SATISFIABLE, ()).into_response();
            }
        }
    }

    // Full body
    match app.store.get_full(&cid).await {
        Ok(bytes) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                axum::http::header::ETAG,
                HeaderValue::from_str(&meta.etag).unwrap(),
            );
            headers.insert(
                axum::http::header::CONTENT_LENGTH,
                HeaderValue::from_str(&meta.len.to_string()).unwrap(),
            );
            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, ()).into_response(),
    }
}
