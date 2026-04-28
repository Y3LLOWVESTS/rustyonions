//! HTTP server wiring for svc-storage.
//! RO:WHAT  — Build Axum router and run the server task.
//! RO:WHY   — Handlers extract State<AppState>, so Router’s state is AppState.
//! RO:INVARIANTS — Unknown → 404; Range GET → 206; strong ETag; paid writes require proof headers.

use std::net::SocketAddr;

use axum::{
    routing::{get, head, put},
    Router,
};
use tracing::{error, info};

use crate::http::extractors::AppState;
#[cfg(feature = "metrics")]
use crate::http::routes::metrics;
use crate::http::routes::{get_object, head_object, paid_object, put_object};
use crate::http::routes::{health, ready, version};

/// Build a router whose state type is **AppState**.
pub fn build_router() -> Router<AppState> {
    let api = Router::new()
        // Free/dev object APIs: accept both PUT and POST for ingest.
        .route("/o", put(put_object::handler).post(put_object::handler))
        .route(
            "/o/:cid",
            head(head_object::handler).get(get_object::handler),
        )
        // Paid object APIs: same CAS write semantics, but payment proof is required.
        .route(
            "/paid/o",
            put(paid_object::handler).post(paid_object::handler),
        )
        // Observability & version.
        .route("/version", get(version::handler))
        .route("/healthz", get(health::handler))
        .route("/readyz", get(ready::handler));

    #[cfg(feature = "metrics")]
    let api = api.route("/metrics", get(metrics::handler));

    let app = Router::new().merge(api);

    info!(
        "mount: POST/PUT /o; POST/PUT /paid/o; HEAD/GET /o/:cid; GET /version; GET /healthz; GET /readyz{}",
        {
            #[cfg(feature = "metrics")]
            {
                "; GET /metrics"
            }
            #[cfg(not(feature = "metrics"))]
            {
                ""
            }
        }
    );

    app
}

pub async fn serve_http(addr: SocketAddr, state: AppState) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("svc-storage listening on {addr}");

    let app = build_router().with_state(state);
    let make_svc = app.into_make_service();

    axum::serve(listener, make_svc)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        error!("shutdown signal failed: {err}");
    }
}
