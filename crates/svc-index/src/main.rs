//! RO:WHAT — svc-index entry: config → state → router → server (Axum 0.7).
//! RO:WHY  — Avoid stateful Router at serve-time; inject state with `.with_state`.
//! RO:INTERACTS — crate::{config, state, router, logging}.
//! RO:INVARIANTS — single bind; AppState behind Arc; graceful shutdown.

use std::{net::SocketAddr, sync::Arc};

use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info, warn};

use svc_index::{build_router, config::Config, logging, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 0) logging/telemetry
    logging::init();

    // 1) Load config
    let cfg = Config::load()?;

    // 2) Build shared state
    let state: Arc<AppState> = Arc::new(AppState::new(cfg.clone()).await?);

    // 3) Optional bootstrap gates (flip readiness, warm caches, etc.)
    let state = AppState::bootstrap(state).await;

    // 4) Build router WITHOUT state and inject state at the end
    //    This turns Router<Arc<AppState>> → Router<()>, which Axum 0.7 can serve.
    let app = build_router().with_state(state.clone());

    // 5) Bind (+ env override) + serve
    //    Respect INDEX_BIND if present; otherwise use cfg.bind; fallback to 127.0.0.1:5304.
    let bind_str = std::env::var("INDEX_BIND").unwrap_or_else(|_| cfg.bind.clone());
    let bind: SocketAddr = bind_str
        .parse()
        .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 5304)));

    let listener: TcpListener = TcpListener::bind(bind).await?;
    info!(
        version = env!("CARGO_PKG_VERSION"),
        %bind,
        "svc-index starting"
    );

    // Serve with graceful shutdown
    let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());
    if let Err(e) = server.await {
        error!(error=?e, "server error");
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    warn!("shutdown signal received");
}
