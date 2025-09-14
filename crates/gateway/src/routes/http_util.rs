// crates/gateway/src/routes/http_util.rs
#![forbid(unsafe_code)]

use axum::http::HeaderValue;

#[inline]
pub fn is_manifest(rel: &str) -> bool {
    rel.eq_ignore_ascii_case("manifest.toml")
}

#[inline]
pub fn etag_hex_from_addr(addr_b3: &str) -> Option<String> {
    // addr like "b3:<hex>.<tld>"
    if let Some(stripped) = addr_b3.strip_prefix("b3:") {
        let hex = stripped.split('.').next().unwrap_or_default();
        if !hex.is_empty() {
            return Some(hex.to_string()); // just the hex
        }
    }
    None
}

pub fn etag_matches(if_none_match: &HeaderValue, our_etag_quoted: &str) -> bool {
    let hdr = if_none_match.to_str().unwrap_or_default();
    if hdr.trim() == "*" {
        return true;
    }
    // Accept either quoted or unquoted; allow comma-separated list.
    let needle_q = our_etag_quoted; // "\"b3:<hex>\""
    let needle_u = needle_q.trim_matches('"'); //  "b3:<hex>"
    hdr.split(',')
        .map(|s| s.trim())
        .any(|tok| tok.trim_matches('"') == needle_u || tok == needle_q)
}

pub fn guess_ct(rel: &str) -> &'static str {
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

/// Parse a single Range header of the form "bytes=start-end".
/// Ok(Some((start,end))) for a satisfiable single range
/// Ok(None) if header isn't a "bytes=" spec we handle
/// Err(()) if syntactically wrong or unsatisfiable.
pub fn parse_single_range(range: &str, len: u64) -> Result<Option<(u64, u64)>, ()> {
    let s = range.trim();
    if !s.starts_with("bytes=") {
        return Ok(None);
    }
    let spec = &s["bytes=".len()..];
    if spec.contains(',') {
        return Err(()); // multi-range unsupported
    }
    let (start_s, end_s) = match spec.split_once('-') {
        Some(v) => v,
        None => return Err(()),
    };
    if start_s.is_empty() {
        // "-suffix" (last N bytes)
        let suffix: u64 = end_s.parse().map_err(|_| ())?;
        if suffix == 0 {
            return Err(());
        }
        if suffix > len {
            return Ok(Some((0, len.saturating_sub(1))));
        }
        return Ok(Some((len - suffix, len - 1)));
    }
    let start: u64 = start_s.parse().map_err(|_| ())?;
    let end: u64 = if end_s.is_empty() {
        len.saturating_sub(1)
    } else {
        end_s.parse().map_err(|_| ())?
    };
    if start >= len || end < start {
        return Err(());
    }
    let end = end.min(len.saturating_sub(1));
    Ok(Some((start, end)))
}
