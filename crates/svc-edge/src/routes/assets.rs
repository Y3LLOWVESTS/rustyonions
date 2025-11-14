//! Edge surface routes.
//!
//! RO:WHAT
//! - `/echo` — counts request body bytes; returns `{ ok, len }`.
//! - `/echo/slow/:ms` — sleeps `ms` milliseconds, then echoes length.
//! - `GET /edge/assets/*path` — serve file from assets dir with ETag/Range.
//! - `GET /cas/:algo/:digest` — minimal CAS fetch (blake3 only), ETag/Range.
//!
//! RO:WHY
//! - Echo routes exercise admission guards.
//! - Assets/CAS scaffold the beta surface (packs/CAS later).
//!
//! RO:HTTP
//! - Strong ETag = BLAKE3(content).
//! - Honor `If-None-Match` → 304 when tag matches.
//! - Support a single range `bytes=start-` → 206; else 200.
//! - Always emit `Accept-Ranges: bytes`.
//!
//! RO:SECURITY
//! - Test-only surface; no auth. Keep payloads small.
//! - Path normalization protects against traversal.
//!
//! RO:METRICS
//! - Record `edge_requests_total` and `edge_request_latency_seconds` per route.

use std::{fs, io, path::PathBuf, time::{Duration, Instant}};

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{
        header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG, IF_NONE_MATCH, RANGE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use blake3::Hasher;
use mime_guess::mime;
use percent_encoding::percent_decode_str;
use serde::Serialize;
use tracing::warn;

use crate::{metrics, AppState};

// ---------- Echo test endpoints ----------

#[derive(Serialize)]
struct EchoResp {
    ok: bool,
    len: usize,
}

/// POST /echo
pub async fn echo(body: Bytes) -> impl IntoResponse {
    let t0 = Instant::now();
    let out = Json(EchoResp { ok: true, len: body.len() });
    metrics::record_request("echo", "POST", 200, t0.elapsed());
    out
}

/// POST /echo/slow/:ms
pub async fn echo_slow(Path(ms): Path<u64>, body: Bytes) -> impl IntoResponse {
    let t0 = Instant::now();
    if ms > 0 {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }
    let out = Json(EchoResp { ok: true, len: body.len() });
    metrics::record_request("echo_slow", "POST", 200, t0.elapsed());
    out
}

// ---------- Assets (filesystem scaffold) ----------

/// GET /edge/assets/*path
pub async fn get_asset(
    State(_state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
) -> Response {
    let t0 = Instant::now();

    // Resolve root from env (temp) — will come from Config later if not already.
    let root = std::env::var("SVC_EDGE_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());

    let resp = match resolve_and_read(&root, &path) {
        Ok(file) => reply_with_etag_range(file, &headers),
        Err(e) => map_fs_err(e),
    };

    metrics::record_request("edge_assets", "GET", resp.status().as_u16(), t0.elapsed());
    resp
}

// ---------- CAS (simple blake3 filesystem scaffold) ----------

/// GET /cas/:algo/:digest
///
/// Only supports algo=blake3 for now. Looks under assets/cas/blake3/{digest}
pub async fn get_cas(
    State(_state): State<AppState>,
    Path((algo, digest)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let t0 = Instant::now();

    let resp = if algo != "blake3" {
        (StatusCode::BAD_REQUEST, "unsupported algo").into_response()
    } else {
        let root = std::env::var("SVC_EDGE_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());
        let mut p = PathBuf::from(root);
        p.push("cas");
        p.push("blake3");
        p.push(sanitize_component(&digest));

        match read_file(&p) {
            Ok(file) => reply_with_etag_range(file, &headers),
            Err(e) => map_fs_err(e),
        }
    };

    metrics::record_request("cas_get", "GET", resp.status().as_u16(), t0.elapsed());
    resp
}

// ---------- Helpers ----------

struct LoadedFile {
    bytes: Vec<u8>,
    etag: String, // blake3 hex
    mime: String,
}

fn resolve_and_read(root: &str, raw_path: &str) -> io::Result<LoadedFile> {
    // Decode URL components and prevent path traversal.
    let decoded = percent_decode_str(raw_path).decode_utf8_lossy();
    let safe = decoded.trim_start_matches('/');
    let safe = sanitize_path(safe);

    let mut p = PathBuf::from(root);
    p.push(safe);

    read_file(&p)
}

fn read_file(p: &PathBuf) -> io::Result<LoadedFile> {
    let data = fs::read(p)?;
    let mut hasher = Hasher::new();
    hasher.update(&data);
    let etag = hasher.finalize().to_hex().to_string();

    // Very simple content-type detection.
    let mime = mime_guess::from_path(p)
        .first_or(mime::APPLICATION_OCTET_STREAM)
        .essence_str()
        .to_string();

    Ok(LoadedFile { bytes: data, etag, mime })
}

fn reply_with_etag_range(file: LoadedFile, headers: &HeaderMap) -> Response {
    // If-None-Match
    if let Some(tag) = headers.get(IF_NONE_MATCH).and_then(|v| v.to_str().ok()) {
        if tag.trim_matches('"') == file.etag {
            return (
                StatusCode::NOT_MODIFIED,
                [(ETAG, quoted(&file.etag)), (ACCEPT_RANGES, HeaderValue::from_static("bytes"))],
            )
                .into_response();
        }
    }

    // Range: only support "bytes=start-"
    let total = file.bytes.len() as u64;
    if let Some(range) = headers.get(RANGE).and_then(|v| v.to_str().ok()) {
        if let Some(start) = parse_range_start(range) {
            if start >= total {
                // 416 Range Not Satisfiable
                let cr = format!("bytes */{}", total);
                return (
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    [
                        (ACCEPT_RANGES, HeaderValue::from_static("bytes")),
                        (CONTENT_RANGE, HeaderValue::from_str(&cr).unwrap()),
                        (ETAG, quoted(&file.etag)),
                    ],
                )
                    .into_response();
            }
            let slice = &file.bytes[start as usize..];
            let len = slice.len() as u64;
            let cr = format!("bytes {}-{}/{}", start, start + len - 1, total);
            return (
                StatusCode::PARTIAL_CONTENT,
                [
                    (ACCEPT_RANGES, HeaderValue::from_static("bytes")),
                    (CONTENT_TYPE, HeaderValue::from_str(&file.mime).unwrap()),
                    (CONTENT_LENGTH, HeaderValue::from_str(&len.to_string()).unwrap()),
                    (CONTENT_RANGE, HeaderValue::from_str(&cr).unwrap()),
                    (ETAG, quoted(&file.etag)),
                ],
                slice.to_vec(),
            )
                .into_response();
        }
    }

    // 200 OK full
    (
        StatusCode::OK,
        [
            (ACCEPT_RANGES, HeaderValue::from_static("bytes")),
            (CONTENT_TYPE, HeaderValue::from_str(&file.mime).unwrap()),
            (
                CONTENT_LENGTH,
                HeaderValue::from_str(&file.bytes.len().to_string()).unwrap(),
            ),
            (ETAG, quoted(&file.etag)),
        ],
        file.bytes,
    )
        .into_response()
}

fn parse_range_start(range: &str) -> Option<u64> {
    // Accept "bytes=START-" only.
    let s = range.trim();
    if !s.starts_with("bytes=") || !s.ends_with('-') {
        return None;
    }
    let inner = &s[6..s.len() - 1]; // drop "bytes=" and trailing '-'
    inner.parse::<u64>().ok()
}

fn quoted(s: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("\"{}\"", s)).unwrap()
}

fn sanitize_path(p: &str) -> String {
    // Simple traversal prevention: strip .. and normalize separators.
    let parts: Vec<_> = p
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
        .collect();
    parts.join("/")
}

fn sanitize_component(c: &str) -> String {
    // Allow only hex-ish tokens for digests.
    let filtered: String = c.chars().filter(|ch| ch.is_ascii_hexdigit()).collect();
    if filtered.is_empty() {
        // fallback to avoid empty path
        "invalid".to_string()
    } else {
        filtered
    }
}

fn map_fs_err(e: io::Error) -> Response {
    match e.kind() {
        io::ErrorKind::NotFound => (StatusCode::NOT_FOUND, "not found").into_response(),
        io::ErrorKind::PermissionDenied => (StatusCode::FORBIDDEN, "forbidden").into_response(),
        _ => {
            warn!("asset error: {e:?}");
            (StatusCode::INTERNAL_SERVER_ERROR, "error").into_response()
        }
    }
}
