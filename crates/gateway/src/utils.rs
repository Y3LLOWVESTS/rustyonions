// crates/gateway/src/utils.rs
#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use index::Index;
use naming::Address;
use std::path::PathBuf;

/// Resolve a bundle directory from the index using the canonical address.
pub async fn resolve_bundle(index_db: &PathBuf, addr_str: &str) -> Result<PathBuf> {
    let address = Address::parse(addr_str).context("parse address")?;
    let idx = Index::open(index_db).context("open index")?;
    let entry = idx
        .get_address(&address)
        .context("get address")?
        .ok_or_else(|| anyhow!("not found"))?;
    Ok(entry.bundle_dir)
}

/// Choose best encoding given an Accept-Encoding header.
pub fn choose_encoding(accept: &str) -> &'static str {
    let a = accept.to_ascii_lowercase();
    if a.contains("zstd") || a.contains("zst") {
        "zstd"
    } else if a.contains("br") {
        "br"
    } else {
        "identity"
    }
}

/// Verify both byte length and BLAKE3(hex) of the encoded data.
pub fn verify_bytes_and_hash(data: &[u8], expect_bytes: u64, expect_hash_hex: &str) -> bool {
    if data.len() as u64 != expect_bytes {
        return false;
    }
    let got = blake3::hash(data).to_hex().to_string();
    got.eq_ignore_ascii_case(expect_hash_hex)
}

/// Common response headers for object delivery.
pub fn basic_headers(
    content_type: &str,
    etag_b3: Option<&str>,
    content_encoding: Option<&str>,
) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert("Content-Type", HeaderValue::from_str(content_type).unwrap());
    if let Some(tag) = etag_b3 {
        let v = format!("\"b3:{}\"", tag);
        h.insert("ETag", HeaderValue::from_str(&v).unwrap());
    }
    if let Some(enc) = content_encoding {
        h.insert("Content-Encoding", HeaderValue::from_str(enc).unwrap());
    }
    h.insert(
        "Cache-Control",
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    h.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    h.insert(
        "Vary",
        HeaderValue::from_static(
            "Accept, Accept-Encoding, DPR, Width, Viewport-Width, Sec-CH-UA, Sec-CH-UA-Platform",
        ),
    );
    h.insert(
        "Accept-CH",
        HeaderValue::from_static(
            "Sec-CH-UA, Sec-CH-UA-Mobile, Sec-CH-UA-Platform, DPR, Width, Viewport-Width, Save-Data",
        ),
    );
    h.insert(
        "Critical-CH",
        HeaderValue::from_static("DPR, Width, Viewport-Width"),
    );
    h
}
