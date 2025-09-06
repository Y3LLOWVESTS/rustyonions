// crates/gateway/src/main.rs
#![forbid(unsafe_code)]

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::task::{Context, Poll};

use anyhow::Result;
use axum::Router;
use axum::body::Body;
use axum::http::Request;
use clap::Parser;
use tower::Service;
use tower::make::Shared;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod index_client;
mod overlay_client;
mod pay_enforce;
mod routes;
mod state;
mod utils;

use crate::index_client::IndexClient;
use crate::overlay_client::OverlayClient;
use crate::routes::router; // now returns Router<()>
use crate::state::AppState;

/// Gateway CLI
#[derive(Debug, Parser)]
#[command(name = "gateway")]
#[command(about = "RustyOnions HTTP gateway (serves /o/<addr> via svc-overlay)")]
struct Args {
    /// Address to bind (host:port). Use 127.0.0.1:0 to auto-pick a port.
    #[arg(long, default_value = "127.0.0.1:0")]
    bind: SocketAddr,

    /// Path to legacy index DB (kept for compat; some code paths may still read it).
    #[arg(long, default_value = ".data/index")]
    #[allow(dead_code)]
    index_db: PathBuf,

    /// Enforce payment requirements from Manifest.toml (returns 402).
    #[arg(long, default_value_t = false)]
    enforce_payments: bool,
}

#[derive(Clone)]
struct AddState<S> {
    inner: S,
    state: AppState,
}

impl<S> AddState<S> {
    fn new(inner: S, state: AppState) -> Self {
        Self { inner, state }
    }
}

impl<S, B> Service<Request<B>> for AddState<S>
where
    S: Service<Request<B>, Error = Infallible> + Clone + Send + 'static,
    S::Response: Send + 'static,
    S::Future: Send + 'static,
    AppState: Clone + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = Infallible;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        // Make the state available to extractors via request extensions.
        req.extensions_mut().insert(self.state.clone());
        self.inner.call(req)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Logging: honor RUST_LOG (fallback to info)
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let args = Args::parse();

    // Clients from env sockets (with sensible defaults)
    let index_client = IndexClient::from_env_or("/tmp/ron/svc-index.sock");
    let overlay_client = OverlayClient::from_env_or("/tmp/ron/svc-overlay.sock");

    // App state (Clone via Arc — see state.rs)
    let state = AppState::new(index_client, overlay_client, args.enforce_payments);

    // 1) Build a STATELESS router (Router<()>)
    let app: Router<()> = router();

    // 2) Turn it into a per-request service that Axum accepts
    let svc = app.into_service::<Body>();

    // 3) Inject AppState into each request’s extensions
    let svc = AddState::new(svc, state);

    // 4) Turn the per-request service into a make-service
    let make_svc = Shared::new(svc);

    // Bind + serve
    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    let local = listener.local_addr()?;
    info!(%local, "gateway listening");

    axum::serve(listener, make_svc).await?;
    Ok(())
}
