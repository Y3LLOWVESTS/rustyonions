// crates/gateway/src/routes/object.rs
#![forbid(unsafe_code)]

use crate::pay_enforce;
use crate::quotas;
use crate::state::AppState;
use crate::utils::basic_headers;

use super::errors::{not_found, too_many_requests, unavailable};
use super::http_util::{etag_hex_from_addr, etag_matches, guess_ct, is_manifest, parse_single_range};

use axum::{
    extract::{Extension, Path},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use tracing::{error, info};

/// GET/HEAD /o/:addr/*tail — fetch bytes via svc-overlay.
/// - Normalizes "<hex>.<tld>" → "b3:<hex>.<tld>".
/// - Quotas: X-RON-CAP / X-API-Key / X-Tenant (429 + Retry-After).
/// - Caching: ETag="b3:<hex>", Cache-Control, If-None-Match → 304, Vary: Accept-Encoding
/// - Precompressed selection: .br / .zst based on Accept-Encoding
/// - Ranges: single "bytes=start-end" supported; 206/416
/// - HEAD: identical headers, no body.
pub async fn serve_object(
    method: Method,
    Extension(state): Extension<AppState>,
    Path((addr_in, tail)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    // Normalize: allow "<hex>.<tld>" or "b3:<hex>.<tld>".
    let addr = if addr_in.contains(':') { addr_in.clone() } else { format!("b3:{addr_in}") };
    let rel = if tail.is_empty() { "payload.bin" } else { tail.as_str() };

    // Tenant identity (best-effort): CAP or API key header; fall back to "public".
    let tenant = headers
        .get("x-ron-cap")
        .or_else(|| headers.get("x-api-key"))
        .or_else(|| headers.get("x-tenant"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("public")
        .to_string();

    info!(%tenant, %addr_in, %addr, %rel, method = %method, "gateway request");

    // Quota guard (429 w/ Retry-After when enabled + exhausted).
    if let Some(retry_after) = quotas::check(&tenant).await {
        return too_many_requests("quota_exhausted", Some(retry_after));
    }

    // Optional payment guard via Manifest.toml (best-effort).
    if state.enforce_payments {
        if let Ok(Some(manifest)) = state.overlay.get_bytes(&addr, "Manifest.toml") {
            if let Err((_code, rsp)) = pay_enforce::guard_bytes(&manifest) {
                return rsp;
            }
        }
    }

    // Derive ETag pieces:
    //  - etag_hex: just <hex>
    //  - etag_str: "\"b3:<hex>\"" for header/matching
    let etag_hex = etag_hex_from_addr(&addr);
    let etag_str = etag_hex.as_ref().map(|h| format!("\"b3:{h}\""));
    let etag_hdr = etag_str.as_deref().and_then(|s| HeaderValue::from_str(s).ok());

    // Conditional GET/HEAD: If-None-Match short-circuit
    if let (Some(etag), Some(if_none)) = (etag_str.as_deref(), headers.get(header::IF_NONE_MATCH)) {
        if etag_matches(if_none, etag) {
            let mut h = HeaderMap::new();
            if let Some(v) = etag_hdr.clone() {
                h.insert(header::ETAG, v);
            }
            h.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
            h.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static(if is_manifest(rel) {
                    "public, max-age=60"
                } else {
                    "public, max-age=31536000, immutable"
                }),
            );
            h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
            return (StatusCode::NOT_MODIFIED, h).into_response();
        }
    }

    // Select encoding based on Accept-Encoding + availability (.br/.zst).
    let ae = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut chosen_rel = rel.to_string();
    let mut content_encoding: Option<HeaderValue> = None;

    if !is_manifest(rel) {
        if ae.contains("br") {
            let candidate = format!("{rel}.br");
            if let Ok(Some(_)) = state.overlay.get_bytes(&addr, &candidate) {
                chosen_rel = candidate;
                content_encoding = Some(HeaderValue::from_static("br"));
            }
        }
        if content_encoding.is_none() && (ae.contains("zstd") || ae.contains("zst")) {
            let candidate = format!("{rel}.zst");
            if let Ok(Some(_)) = state.overlay.get_bytes(&addr, &candidate) {
                chosen_rel = candidate;
                content_encoding = Some(HeaderValue::from_static("zstd"));
            }
        }
    }

    // Fetch the chosen bytes (for GET and to compute Content-Length for HEAD/RANGE).
    let fetch = state.overlay.get_bytes(&addr, &chosen_rel);
    let Some(bytes) = (match fetch {
        Ok(Some(b)) => Some(b),
        Ok(None) => None,
        Err(e) => {
            error!(error=?e, %addr, rel=%chosen_rel, "overlay get error");
            return unavailable("backend unavailable", None);
        }
    }) else {
        return not_found("object or file not found");
    };

    // Derive content-type from *original* rel (not the encoded suffix).
    let ctype = guess_ct(rel);

    // Common headers (basic_headers expects plain hex for ETag input)
    let mut h: HeaderMap = basic_headers(ctype, etag_hex.as_deref(), None);
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if is_manifest(rel) {
            "public, max-age=60"
        } else {
            "public, max-age=31536000, immutable"
        }),
    );
    h.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    if let Some(enc) = &content_encoding {
        h.insert(header::CONTENT_ENCODING, enc.clone());
    }
    if !is_manifest(rel) {
        h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    }

    // HEAD: headers only
    if method == Method::HEAD {
        h.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&bytes.len().to_string()).unwrap(),
        );
        return (StatusCode::OK, h).into_response();
    }

    // RANGES: support a single "bytes=start-end" range
    if let Some(range_hdr) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        match parse_single_range(range_hdr, bytes.len() as u64) {
            Ok(Some((start, end))) => {
                let start_i = start as usize;
                let end_i = end as usize; // inclusive
                if start_i < bytes.len() && end_i < bytes.len() && start_i <= end_i {
                    let body = bytes[start_i..=end_i].to_vec();
                    let mut h206 = h.clone();
                    h206.insert(
                        header::CONTENT_RANGE,
                        HeaderValue::from_str(&format!("bytes {}-{}/{}", start, end, bytes.len()))
                            .unwrap(),
                    );
                    h206.insert(
                        header::CONTENT_LENGTH,
                        HeaderValue::from_str(&body.len().to_string()).unwrap(),
                    );
                    return (StatusCode::PARTIAL_CONTENT, h206, body).into_response();
                }
            }
            Ok(None) => { /* ignore: serve full */ }
            Err(_) => {
                let mut h416 = HeaderMap::new();
                h416.insert(
                    header::CONTENT_RANGE,
                    HeaderValue::from_str(&format!("bytes */{}", bytes.len())).unwrap(),
                );
                return (StatusCode::RANGE_NOT_SATISFIABLE, h416).into_response();
            }
        }
    }

    // Full body
    let mut h200 = h;
    h200.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&bytes.len().to_string()).unwrap(),
    );
    (StatusCode::OK, h200, bytes).into_response()
}
