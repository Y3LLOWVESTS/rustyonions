//! HTTP server wiring for svc-storage.
//! RO:WHAT  — Build Axum router and run the server task.
//! RO:WHY   — Handlers extract State<AppState>, so Router’s state is AppState.
//! RO:INVARIANTS — Unknown → 404; Range GET → 206; strong ETag; state is Send+Sync+'static.

use std::net::SocketAddr;

use axum::{
    routing::{get, head, put},
    Router,
};
use tracing::{error, info};

use crate::http::extractors::AppState;
#[cfg(feature = "metrics")]
use crate::http::routes::metrics;
use crate::http::routes::version;
use crate::http::routes::{get_object, head_object, put_object};

/// Build a router whose state type is **AppState** (because handlers use State<AppState>).
pub fn build_router() -> Router<AppState> {
    let api = Router::new()
        .route("/o", put(put_object::handler))
        .route(
            "/o/:cid",
            head(head_object::handler).get(get_object::handler),
        )
        .route("/version", get(version::handler));

    #[cfg(feature = "metrics")]
    let api = api.route("/metrics", get(metrics::handler));

    let app = Router::new().merge(api);

    info!("mount: PUT /o; HEAD/GET /o/:cid; GET /version{}", {
        #[cfg(feature = "metrics")]
        {
            "; GET /metrics"
        }
        #[cfg(not(feature = "metrics"))]
        {
            ""
        }
    });

    app
}

pub async fn serve_http(addr: SocketAddr, state: AppState) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("svc-storage listening on {addr}");

    // build_router() -> Router<AppState>, so with_state expects an AppState value
    let app = build_router().with_state(state);

    // Router<AppState> → MakeService, which axum::serve expects
    let make_svc = app.into_make_service();

    axum::serve(listener, make_svc)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        error!("shutdown signal failed: {e}");
    }
}
