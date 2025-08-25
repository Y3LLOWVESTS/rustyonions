// crates/gateway/src/routes.rs
#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use bytes::Bytes;
use serde::Deserialize;
use tokio::{
    fs,
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
};
use tokio_util::io::ReaderStream;

use crate::state::AppState;
use crate::utils::{basic_headers, choose_encoding, resolve_bundle, verify_bytes_and_hash};

/// Build the router with our two object routes.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/o/:addr/Manifest.toml", get(serve_manifest))
        .route("/o/:addr/payload.bin", get(serve_payload))
        .with_state(state)
}

/// Serve the raw Manifest.toml (mostly for debugging/tests)
async fn serve_manifest(
    State(state): State<Arc<AppState>>,
    Path(addr_str): Path<String>,
) -> Response {
    // Resolve bundle directory via index
    let dir = match resolve_bundle(&state.index_db, &addr_str).await {
        Ok(d) => d,
        Err(_) => return (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()).into_response(),
    };

    // 🔐 Enforcement
    if let Err((_code, resp)) = state.enforcer.guard(&dir, &addr_str) {
        return resp;
    }

    let path = dir.join("Manifest.toml");
    match fs::read(path).await {
        Ok(bytes) => {
            // We advertise range on manifest too (harmless)
            let mut h = basic_headers("text/plain; charset=utf-8", None, None);
            let _ = h.insert("Accept-Ranges", HeaderValue::from_static("bytes"));
            (StatusCode::OK, h, Bytes::from(bytes)).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()).into_response(),
    }
}

/// Serve the object payload, performing encoding negotiation and integrity checks.
/// Adds HTTP Range (single range) + If-Range (ETag) support. Range requests always
/// serve the identity representation.
async fn serve_payload(
    State(state): State<Arc<AppState>>,
    Path(addr_str): Path<String>,
    headers: HeaderMap,
) -> Response {
    // Resolve bundle directory via index
    let dir = match resolve_bundle(&state.index_db, &addr_str).await {
        Ok(d) => d,
        Err(_) => return (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()).into_response(),
    };

    // 🔐 Enforcement
    if let Err((_code, resp)) = state.enforcer.guard(&dir, &addr_str) {
        return resp;
    }

    // Load and parse manifest
    let manifest_path = dir.join("Manifest.toml");
    let raw = match fs::read_to_string(&manifest_path).await {
        Ok(s) => s,
        Err(_) => return (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()).into_response(),
    };

    // Minimal inline structs (avoid pulling full naming::manifest here)
    #[derive(Deserialize)]
    struct PayLite {
        #[serde(default)]
        required: bool,
        #[serde(default)]
        currency: String,
        #[serde(default)]
        price_model: String,
        #[serde(default)]
        price: f64,
        #[serde(default)]
        wallet: String,
    }

    #[derive(Deserialize)]
    struct EncV2 {
        coding: String,
        filename: String,
        bytes: u64,
        hash_hex: String,
    }

    #[derive(Deserialize)]
    struct ManV2 {
        schema_version: u32,
        mime: String,
        stored_filename: String,
        hash_hex: String, // hash of ORIGINAL bytes
        #[serde(default)]
        encodings: Vec<EncV2>,
        #[serde(default)]
        payment: Option<PayLite>,
    }

    let (mime, stored_filename, etag_b3, encodings, payment) = match toml::from_str::<ManV2>(&raw) {
        Ok(m) if m.schema_version == 2 => (
            m.mime,
            m.stored_filename,
            m.hash_hex,
            m.encodings,
            m.payment,
        ),
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                Bytes::new(),
            )
                .into_response()
        }
    };

    // Build a canonical ETag (matches your existing format)
    let etag_value = format!("\"b3:{}\"", etag_b3);

    // Parse Range / If-Range
    let range_header = headers
        .get("range")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    let if_range_ok = if let Some(ir) = headers.get("if-range").and_then(|v| v.to_str().ok()) {
        // Only support ETag validator (exact match). If not equal, ignore Range.
        ir.trim() == etag_value
    } else {
        true // no If-Range given ⇒ treat as OK
    };

    // If a Range header is present and either no If-Range or it matches our ETag,
    // serve identity with 206.
    if let Some(rh) = range_header {
        if if_range_ok {
            let path = dir.join(&stored_filename);
            let mut file = match File::open(&path).await {
                Ok(f) => f,
                Err(_) => {
                    return (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()).into_response()
                }
            };
            let meta = match file.metadata().await {
                Ok(m) => m,
                Err(_) => {
                    return (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()).into_response()
                }
            };
            let full_len = meta.len();

            let range = match parse_single_range(rh, full_len) {
                Ok(r) => r,
                Err(_) => {
                    // 416 Range Not Satisfiable
                    let mut h = HeaderMap::new();
                    let _ = h.insert(
                        "Content-Range",
                        HeaderValue::from_str(&format!("bytes */{full_len}")).unwrap(),
                    );
                    return (StatusCode::RANGE_NOT_SATISFIABLE, h, Bytes::new()).into_response();
                }
            };

            let (start, end) = range;
            let count = end - start + 1;

            // Seek and stream only the requested slice
            if let Err(_) = file.seek(std::io::SeekFrom::Start(start)).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    HeaderMap::new(),
                    Bytes::new(),
                )
                    .into_response();
            }
            let limited = file.take(count);
            let stream = ReaderStream::new(limited);
            let body = Body::from_stream(stream);

            // Build headers (no Content-Encoding on range; identity only)
            let mut h = basic_headers(&mime, Some(&etag_b3), None);
            let _ = h.insert("Accept-Ranges", HeaderValue::from_static("bytes"));
            let _ = h.insert(
                "Content-Range",
                HeaderValue::from_str(&format!("bytes {}-{}/{}", start, end, full_len)).unwrap(),
            );
            let _ = h.insert(
                "Content-Length",
                HeaderValue::from_str(&format!("{}", count)).unwrap(),
            );

            // Advisory payment headers (even with range)
            if let Some(p) = &payment {
                if p.required {
                    let _ = h.insert("X-Payment-Required", HeaderValue::from_static("true"));
                    if !p.currency.is_empty() {
                        let _ = h.insert(
                            "X-Payment-Currency",
                            HeaderValue::from_str(&p.currency).unwrap(),
                        );
                    }
                    if !p.price_model.is_empty() {
                        let _ = h.insert(
                            "X-Payment-Price-Model",
                            HeaderValue::from_str(&p.price_model).unwrap(),
                        );
                    }
                    if p.price > 0.0 {
                        let _ = h.insert(
                            "X-Payment-Price",
                            HeaderValue::from_str(&format!("{}", p.price)).unwrap(),
                        );
                    }
                    if !p.wallet.is_empty() {
                        let _ = h.insert(
                            "X-Payment-Wallet",
                            HeaderValue::from_str(&p.wallet).unwrap(),
                        );
                    }
                }
            }

            return (StatusCode::PARTIAL_CONTENT, h, body).into_response();
        }
        // If-Range present but doesn't match → ignore Range, fall through to normal 200 selection
    }

    // ---------- Normal (no-range) path below ----------

    // Accept-Encoding negotiation
    let accept = headers
        .get("accept-encoding")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let want = choose_encoding(accept);

    // Candidate selection: preferred encoding first → identity fallback
    #[derive(Clone)]
    struct Candidate<'a> {
        path_rel: String,
        content_encoding: Option<&'a str>,
        expect_bytes: Option<u64>,
        expect_hash_hex: Option<String>,
    }

    let mut candidates: Vec<Candidate> = Vec::new();
    if want == "zstd" {
        if let Some(e) = encodings.iter().find(|e| e.coding == "zstd") {
            candidates.push(Candidate {
                path_rel: e.filename.clone(),
                content_encoding: Some("zstd"),
                expect_bytes: Some(e.bytes),
                expect_hash_hex: Some(e.hash_hex.clone()),
            });
        }
    } else if want == "br" {
        if let Some(e) = encodings.iter().find(|e| e.coding == "br") {
            candidates.push(Candidate {
                path_rel: e.filename.clone(),
                content_encoding: Some("br"),
                expect_bytes: Some(e.bytes),
                expect_hash_hex: Some(e.hash_hex.clone()),
            });
        }
    }
    // Always include identity as a last resort
    candidates.push(Candidate {
        path_rel: stored_filename.clone(),
        content_encoding: None,
        expect_bytes: None,
        expect_hash_hex: None,
    });

    // Try candidates in order; verify encoded ones
    for cand in candidates {
        let path = dir.join(&cand.path_rel);
        let data = match fs::read(&path).await {
            Ok(b) => b,
            Err(_) => continue,
        };

        if let (Some(exp_bytes), Some(exp_hash)) =
            (cand.expect_bytes, cand.expect_hash_hex.as_ref())
        {
            if !verify_bytes_and_hash(&data, exp_bytes, exp_hash) {
                // Integrity failed → try next (likely identity)
                continue;
            }
        }

        // Build headers
        let mut h = basic_headers(&mime, Some(&etag_b3), cand.content_encoding);
        // Advertise range capability
        let _ = h.insert("Accept-Ranges", HeaderValue::from_static("bytes"));

        // Advisory payment headers (even when enforcement is off)
        if let Some(p) = &payment {
            if p.required {
                let _ = h.insert("X-Payment-Required", HeaderValue::from_static("true"));
                if !p.currency.is_empty() {
                    let _ = h.insert(
                        "X-Payment-Currency",
                        HeaderValue::from_str(&p.currency).unwrap(),
                    );
                }
                if !p.price_model.is_empty() {
                    let _ = h.insert(
                        "X-Payment-Price-Model",
                        HeaderValue::from_str(&p.price_model).unwrap(),
                    );
                }
                if p.price > 0.0 {
                    let _ = h.insert(
                        "X-Payment-Price",
                        HeaderValue::from_str(&format!("{}", p.price)).unwrap(),
                    );
                }
                if !p.wallet.is_empty() {
                    let _ = h.insert(
                        "X-Payment-Wallet",
                        HeaderValue::from_str(&p.wallet).unwrap(),
                    );
                }
            }
        }

        return (StatusCode::OK, h, Bytes::from(data)).into_response();
    }

    // If we got here, all candidates failed (missing or integrity mismatch)
    let mut h = HeaderMap::new();
    let _ = h.insert(
        "Content-Type",
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        h,
        Bytes::from_static(b"integrity check failed"),
    )
        .into_response()
}

/// Parse a single HTTP byte range. Supports:
/// - "bytes=<start>-<end>"
/// - "bytes=<start>-"
/// - "bytes=-<suffix_len>"
fn parse_single_range(h: &str, full_len: u64) -> Result<(u64, u64), ()> {
    let s = h.trim();
    if !s.to_ascii_lowercase().starts_with("bytes=") {
        return Err(());
    }
    let spec = &s[6..]; // after "bytes="
                        // Disallow multiple ranges for now
    if spec.contains(',') {
        return Err(());
    }
    if spec.starts_with('-') {
        // suffix: last N bytes
        let n: u64 = spec[1..].parse().map_err(|_| ())?;
        if n == 0 {
            return Err(()); // invalid
        }
        if n >= full_len {
            return Ok((0, full_len.saturating_sub(1)));
        }
        let start = full_len - n;
        let end = full_len - 1;
        Ok((start, end))
    } else if let Some((a, b)) = spec.split_once('-') {
        let start: u64 = a.parse().map_err(|_| ())?;
        if b.is_empty() {
            // open-ended "start-"
            if start >= full_len {
                return Err(());
            }
            return Ok((start, full_len - 1));
        } else {
            let end: u64 = b.parse().map_err(|_| ())?;
            if start > end {
                return Err(());
            }
            if end >= full_len {
                return Err(());
            }
            return Ok((start, end));
        }
    } else {
        Err(())
    }
}
