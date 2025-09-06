// crates/gateway/tests/http_read_path.rs
//! Integration checks for the Gateway read path: HEAD/ETag/304, Range 206/416,
//! and precompressed selection signals.
//!
//! Env:
//!   GATEWAY_URL = http://127.0.0.1:9080
//!   OBJ_ADDR    = b3:<hex>[.suffix]?   (e.g., b3:abcd... or b3:... .text/.post)
//!   REL         = Manifest.toml        (default) — relative file to fetch under the object
//!   ACCEPTS     = "br, zstd, gzip, identity" (optional)

use std::env;

use http::header::{ACCEPT_ENCODING, CONTENT_RANGE, CONTENT_TYPE, ETAG, IF_NONE_MATCH, RANGE};
use reqwest::{Client, StatusCode};

fn var(name: &str) -> Option<String> { env::var(name).ok().filter(|s| !s.trim().is_empty()) }
fn gw() -> Option<String> { var("GATEWAY_URL") }
fn obj() -> Option<String> { var("OBJ_ADDR") }
fn rel() -> String { var("REL").unwrap_or_else(|| "Manifest.toml".to_string()) }

#[tokio::test]
async fn not_found_envelope_is_stable_json() {
    let Some(base) = gw() else { eprintln!("SKIP: set GATEWAY_URL"); return; };
    let url = format!("{base}/__definitely_missing__");
    let resp = Client::new().get(url).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "expected 404");

    let ct = resp.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
    assert!(ct.starts_with("application/json"), "content-type should be application/json*, got {ct}");

    let body = resp.text().await.unwrap();
    for k in ["\"code\"", "\"message\"", "\"retryable\"", "\"corr_id\""] {
        assert!(body.contains(k), "missing key {k} in error envelope");
    }
}

#[tokio::test]
async fn head_reports_content_length() {
    let (Some(base), Some(addr)) = (gw(), obj()) else { eprintln!("SKIP: set GATEWAY_URL and OBJ_ADDR"); return; };
    let url = format!("{base}/o/{addr}/{}", rel());
    let resp = Client::new().head(url).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "HEAD should be 200");
    let len = resp.headers().get(http::header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()).and_then(|s| s.parse::<u64>().ok());
    assert!(len.is_some(), "HEAD must include Content-Length");
}

#[tokio::test]
async fn conditional_get_yields_304() {
    let (Some(base), Some(addr)) = (gw(), obj()) else { eprintln!("SKIP: set GATEWAY_URL and OBJ_ADDR"); return; };
    let url = format!("{base}/o/{addr}/{}", rel());
    let resp = Client::new().get(&url).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "first GET should be 200");
    let Some(etag) = resp.headers().get(ETAG).cloned() else { eprintln!("SKIP: server did not return ETag"); return; };
    let resp2 = Client::new().get(&url).header(IF_NONE_MATCH, etag).send().await.unwrap();
    assert_eq!(resp2.status(), StatusCode::NOT_MODIFIED, "expected 304 on If-None-Match");
}

#[tokio::test]
async fn range_requests_support_206() {
    let (Some(base), Some(addr)) = (gw(), obj()) else { eprintln!("SKIP: set GATEWAY_URL and OBJ_ADDR"); return; };
    let url = format!("{base}/o/{addr}/{}", rel());
    let resp = Client::new().get(&url).header(RANGE, "bytes=0-9").send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT, "expect 206 for satisfiable range");
    let cr = resp.headers().get(CONTENT_RANGE).and_then(|v| v.to_str().ok()).unwrap_or("");
    assert!(cr.starts_with("bytes "), "Content-Range header must be present on 206");
}

#[tokio::test]
async fn unsatisfiable_range_yields_416() {
    let (Some(base), Some(addr)) = (gw(), obj()) else { eprintln!("SKIP: set GATEWAY_URL and OBJ_ADDR"); return; };
    let url = format!("{base}/o/{addr}/{}", rel());
    let resp = Client::new().get(&url).header(RANGE, "bytes=999999999999-999999999999").send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::RANGE_NOT_SATISFIABLE, "expect 416 for unsatisfiable range");
}

#[tokio::test]
async fn precompressed_selection_signals_encoding_if_used() {
    let (Some(base), Some(addr)) = (gw(), obj()) else { eprintln!("SKIP: set GATEWAY_URL and OBJ_ADDR"); return; };
    let url = format!("{base}/o/{addr}/{}", rel());
    let accepts = var("ACCEPTS").unwrap_or_else(|| "br, zstd, gzip, identity".to_string());
    let resp = Client::new().get(&url).header(ACCEPT_ENCODING, accepts).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let enc = resp.headers().get(http::header::CONTENT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("identity");
    assert!(["br", "zstd", "zst", "gzip", "identity"].contains(&enc), "unexpected Content-Encoding: {enc}");
}
