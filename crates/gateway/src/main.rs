use anyhow::{Context, Result};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use clap::Parser;
use index::Index;
use naming::Address;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{fs::File, io::BufReader, net::TcpListener};
use tokio_util::io::ReaderStream;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Parser, Clone)]
#[command(name="gateway", about="Serve bundles via /o/<addr> from Sled index")]
struct Cli {
    /// Bind address (e.g., 127.0.0.1:31555)
    #[arg(long, default_value = "127.0.0.1:31555")]
    bind: String,

    /// Path to index DB
    #[arg(long, default_value = ".data/index")]
    index_db: PathBuf,

    /// Optional default bundle root if not found in index (usually .onions)
    #[arg(long, default_value = ".onions")]
    root: PathBuf,
}

#[derive(Clone)]
struct AppState {
    index: Arc<Index>,
    root: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,axum=info"));
    fmt().with_env_filter(filter).init();

    let cli = Cli::parse();
    let addr: SocketAddr = cli.bind.parse().context("invalid --bind address")?;

    let index = Arc::new(Index::open(&cli.index_db).context("open index")?);
    let state = AppState { index, root: cli.root };

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/o/:addr", get(get_manifest_redirect))
        .route("/o/:addr/Manifest.toml", get(get_manifest))
        .route("/o/:addr/payload.bin", get(get_payload))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!("gateway listening on http://{}", addr);

    // Axum 0.7 style server
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn resolve_bundle_dir(state: &AppState, addr: &str) -> Result<PathBuf, StatusCode> {
    let a: Address = addr.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    match state
        .index
        .get_address(&a)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        Some(ent) => Ok(ent.bundle_dir),
        None => {
            // Fallback to root/<addr> if it exists
            let fallback = state.root.join(a.to_string());
            if tokio::fs::try_exists(&fallback)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            {
                Ok(fallback)
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
    }
}

async fn get_manifest_redirect(Path(addr): Path<String>) -> Response {
    Redirect::temporary(&format!("/o/{addr}/Manifest.toml")).into_response()
}

async fn get_manifest(State(state): State<AppState>, Path(addr): Path<String>) -> Response {
    match serve_file_in_bundle(&state, &addr, "Manifest.toml").await {
        Ok(resp) => resp,
        Err(code) => (code, "error").into_response(),
    }
}

async fn get_payload(State(state): State<AppState>, Path(addr): Path<String>) -> Response {
    match serve_file_in_bundle(&state, &addr, "payload.bin").await {
        Ok(resp) => resp,
        Err(code) => (code, "error").into_response(),
    }
}

async fn serve_file_in_bundle(
    state: &AppState,
    addr: &str,
    name: &str,
) -> Result<Response, StatusCode> {
    let dir = resolve_bundle_dir(state, addr).await?;
    let path = dir.join(name);

    let file = File::open(&path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let meta = file
        .metadata()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_LENGTH, meta.len().into());

    // Guess content type
    let ct = if name.ends_with(".toml") {
        "application/toml"
    } else {
        mime_guess::from_path(&path)
            .first_raw()
            .unwrap_or("application/octet-stream")
    };
    headers.insert(header::CONTENT_TYPE, ct.parse().unwrap());

    let stream = ReaderStream::new(BufReader::new(file));
    let body = Body::from_stream(stream);

    Ok((headers, body).into_response())
}
