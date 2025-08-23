// crates/gateway/src/main.rs
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use axum::{
    extract::Path,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use bytes::Bytes;
use clap::Parser;
use index::Index;
use naming::{Address};
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf};
use tokio::fs;

#[derive(Parser, Debug)]
#[command(name = "gateway", version)]
struct Args {
    /// Path to the index database (Sled dir)
    #[arg(long, default_value = ".data/index")]
    index_db: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Bind on a random localhost port like before (0 = random).
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

    let app = Router::new()
        .route("/o/:addr/Manifest.toml", get({
            let index_db = args.index_db.clone();
            move |path| serve_manifest(index_db.clone(), path)
        }))
        .route("/o/:addr/payload.bin", get({
            let index_db = args.index_db.clone();
            move |path, headers| serve_payload(index_db.clone(), path, headers)
        }));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local = listener.local_addr()?;
    println!("gateway listening on http://{}", local);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn serve_manifest(index_db: PathBuf, Path(addr_str): Path<String>) -> impl IntoResponse {
    match resolve_bundle(&index_db, &addr_str).await {
        Ok(dir) => {
            let path = dir.join("Manifest.toml");
            match fs::read(path).await {
                Ok(bytes) => (
                    StatusCode::OK,
                    basic_headers("text/plain; charset=utf-8", None, None),
                    Bytes::from(bytes),
                ),
                Err(_) => (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()),
            }
        }
        Err(_) => (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()),
    }
}

async fn serve_payload(
    index_db: PathBuf,
    Path(addr_str): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Resolve bundle dir
    let dir = match resolve_bundle(&index_db, &addr_str).await {
        Ok(d) => d,
        Err(_) => return (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()),
    };

    // Read and parse manifest v2 (fallback to v1 if needed)
    let manifest_path = dir.join("Manifest.toml");
    let raw = match fs::read_to_string(&manifest_path).await {
        Ok(s) => s,
        Err(_) => return (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()),
    };

    // Try v2 first
    #[derive(Deserialize)]
    struct EncV2 {
        coding: String,
        level: i32,
        bytes: u64,
        filename: String,
        hash_hex: String,
    }
    #[derive(Deserialize)]
    struct ManV2 {
        schema_version: u32,
        mime: String,
        stored_filename: String,
        hash_hex: String,
        encodings: Option<Vec<EncV2>>,
    }
    // Minimal v1 fallback
    #[derive(Deserialize)]
    struct ManV1 {
        schema_version: u32,
        stored_filename: String,
        hash_hex: String,
    }

    // Select manifest flavor
    let (mime, stored_filename, etag_b3, encodings) = match toml::from_str::<ManV2>(&raw) {
        Ok(m) if m.schema_version == 2 => {
            (m.mime, m.stored_filename, m.hash_hex, m.encodings.unwrap_or_default())
        }
        _ => match toml::from_str::<ManV1>(&raw) {
            Ok(m1) => ("application/octet-stream".to_string(), m1.stored_filename, m1.hash_hex, vec![]),
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, HeaderMap::new(), Bytes::new()),
        },
    };

    // Parse Accept-Encoding (prefer zstd > br > identity)
    let accept = headers
        .get("accept-encoding")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let want = choose_encoding(accept);

    // If we have a matching precompressed file listed in encodings, use it.
    let mut chosen_path = dir.join(&stored_filename);
    let mut content_encoding: Option<&'static str> = None;

    if want == "zstd" {
        if let Some(e) = encodings.iter().find(|e| e.coding == "zstd") {
            chosen_path = dir.join(&e.filename);
            content_encoding = Some("zstd");
        }
    } else if want == "br" {
        if let Some(e) = encodings.iter().find(|e| e.coding == "br") {
            chosen_path = dir.join(&e.filename);
            content_encoding = Some("br");
        }
    }

    let body = match fs::read(&chosen_path).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::NOT_FOUND, HeaderMap::new(), Bytes::new()),
    };

    let mut h = basic_headers(&mime, Some(&etag_b3), content_encoding);
    (StatusCode::OK, h, Bytes::from(body))
}

async fn resolve_bundle(index_db: &PathBuf, addr_str: &str) -> Result<PathBuf> {
    let address = Address::parse(addr_str).context("parse address")?;
    let idx = Index::open(index_db).context("open index")?;
    let entry = idx
        .get_address(&address)
        .context("get address")?
        .ok_or_else(|| anyhow::anyhow!("not found"))?;
    Ok(entry.bundle_dir)
}

fn choose_encoding(accept: &str) -> &'static str {
    // Naive but effective: look for tokens in order of preference
    let a = accept.to_ascii_lowercase();
    if a.contains("zstd") || a.contains("zst") {
        "zstd"
    } else if a.contains("br") {
        "br"
    } else {
        "identity"
    }
}

fn basic_headers(
    content_type: &str,
    etag_b3: Option<&str>,
    content_encoding: Option<&str>,
) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert("Content-Type", HeaderValue::from_str(content_type).unwrap());
    if let Some(tag) = etag_b3 {
        // strong cache identity: canonical content hash of original bytes
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
    h.insert("Critical-CH", HeaderValue::from_static("DPR, Width, Viewport-Width"));
    h
}
