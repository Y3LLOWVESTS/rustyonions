#![forbid(unsafe_code)]

use axum::http::{HeaderMap, HeaderValue};

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
    h.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
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
