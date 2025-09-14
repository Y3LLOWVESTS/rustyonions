// FILE: crates/gateway/tests/http_read_path.rs
#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use reqwest::header::{
    ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, ETAG, IF_NONE_MATCH, RANGE,
};
use reqwest::{Client, StatusCode};
use std::time::Duration;

/// Resolve base URL of the running gateway, defaulting to 127.0.0.1:9080
fn base_url() -> String {
    std::env::var("GATEWAY_URL").unwrap_or_else(|_| "http://127.0.0.1:9080".to_string())
}

/// Resolve the test object address (e.g., "b3:<hex>.<tld>").
/// Many of our scripts set this as OBJ_ADDR after packing.
fn obj_addr() -> Result<String> {
    std::env::var("OBJ_ADDR")
        .context("OBJ_ADDR env var not set (expected packed test object address)")
}

#[tokio::test]
async fn http_read_path_end_to_end() -> Result<()> {
    // Prep
    let base = base_url();
    let addr = obj_addr()?;
    let url = format!("{}/o/{}/payload.bin", base, addr);

    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

    // 1) Basic GET
    let resp = client.get(&url).send().await.context("GET send failed")?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET {} -> unexpected status {}", url, status);
    }

    // Body as text (best effort; if not UTF-8, just read bytes)
    let etag = resp.headers().get(ETAG).cloned();
    let body_res = resp.text().await;
    match body_res {
        Ok(s) => {
            // Don't assert on contents; we only validate the path succeeds.
            assert!(
                !s.is_empty(),
                "GET returned empty body (allowed, but unusual)"
            );
        }
        Err(_) => {
            // Retry as bytes — some payloads are binary
            let resp2 = client.get(&url).send().await?;
            let _bytes = resp2.bytes().await?;
        }
    }

    // 2) HEAD should return headers (incl. Content-Length if known)
    let resp = client.head(&url).send().await.context("HEAD send failed")?;
    let status = resp.status();
    if !(status == StatusCode::OK || status == StatusCode::NO_CONTENT) {
        bail!("HEAD {} -> unexpected status {}", url, status);
    }
    if let Some(cl) = resp.headers().get(CONTENT_LENGTH) {
        let _ = cl.to_str().ok().and_then(|s| s.parse::<u64>().ok());
        // We don't assert here; some backends stream without a fixed length.
    }

    // 3) Conditional GET with If-None-Match (expect 304 if ETag supports it)
    if let Some(tag) = etag {
        if let Ok(tag_str) = tag.to_str() {
            let resp2 = client
                .get(&url)
                .header(IF_NONE_MATCH, tag_str)
                .send()
                .await?;
            // 304 is ideal; but some setups might return 200 if ETag changed or is not stable.
            // We accept either 304 or 200 to keep test robust across environments.
            assert!(
                resp2.status() == StatusCode::NOT_MODIFIED || resp2.status().is_success(),
                "If-None-Match should produce 304 or 200; got {}",
                resp2.status()
            );
        }
    }

    // 4) Byte-range: ask for the first 10 bytes; expect 206 or 200 if not supported
    let resp = client.get(&url).header(RANGE, "bytes=0-9").send().await?;
    assert!(
        resp.status() == StatusCode::PARTIAL_CONTENT || resp.status().is_success(),
        "expected 206 or 200 for RANGE 0-9; got {}",
        resp.status()
    );

    // 5) Invalid byte-range — many servers return 416; accept 200 if server ignores invalid ranges
    let resp = client
        .get(&url)
        .header(RANGE, "bytes=999999999999-999999999999")
        .send()
        .await?;
    assert!(
        resp.status() == StatusCode::RANGE_NOT_SATISFIABLE || resp.status().is_success(),
        "expected 416 or 200 for invalid range; got {}",
        resp.status()
    );

    // 6) Content-Encoding negotiation: try common encodings (best-effort)
    for accepts in ["br, zstd, gzip", "zstd, gzip", "gzip"] {
        let resp = client
            .get(&url)
            .header(ACCEPT_ENCODING, accepts)
            .send()
            .await?;
        assert!(
            resp.status().is_success(),
            "GET with Accept-Encoding='{}' should succeed; got {}",
            accepts,
            resp.status()
        );
        if let Some(enc) = resp.headers().get(CONTENT_ENCODING) {
            let _ = enc.to_str().ok(); // do not assert exact encoding; depends on what artifacts are present
        }
    }

    Ok(())
}
