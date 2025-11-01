//! RO:WHAT — Axum HTTP server bootstrap for the API plane.
//! RO:WHY  — Separate from main; Concerns: RES/PERF; handles bind + graceful-ish shutdown.
//! RO:INTERACTS — axum::Router, crate::config::Server, ron-kernel readiness (future toggle).
//! RO:INVARIANTS — bind before marking ready; one server task, one listener; stop cleanly on Ctrl-C.

use axum::Router;
use std::net::SocketAddr;
use tokio::task::JoinHandle;
use tracing::{error, info};

pub async fn serve(
    cfg: crate::config::Server,
    router: Router,
) -> anyhow::Result<(JoinHandle<()>, SocketAddr)> {
    // Bind first to satisfy "bind before ready".
    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    let local: SocketAddr = listener.local_addr()?;
    info!(%local, "api listener bound");

    // Axum/Hyper server with graceful shutdown on Ctrl-C.
    // (Main still holds the JoinHandle and can abort on top; this just ensures
    //  a clean drain when Ctrl-C is delivered to the process.)
    let http = axum::serve(listener, router).with_graceful_shutdown(async {
        // Best-effort: if ctrl_c fails, just keep serving.
        if let Err(e) = tokio::signal::ctrl_c().await {
            // Log once; we don't bubble this up because we want the server to continue.
            error!(error=?e, "ctrl-c listener failed in server task");
        }
    });

    let task = tokio::spawn(async move {
        if let Err(e) = http.await {
            // This fires on listener errors or if the accept loop ends unexpectedly.
            tracing::error!(error=?e, "http server stopped with error");
        } else {
            tracing::info!("http server exited");
        }
    });

    Ok((task, local))
}
