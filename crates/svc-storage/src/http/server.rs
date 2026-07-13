//! HTTP server wiring for svc-storage.
//!
//! Handlers extract `AppState`, while exact-b3 moderation is installed as an
//! immutable router extension. The default builder remains permissive through
//! an empty policy; production-shaped runtimes may inject a loaded snapshot.

use std::{net::SocketAddr, sync::Arc};

use axum::{
    routing::{get, head, post, put},
    Extension, Router,
};
use ron_policy::ModerationPolicy;
use tracing::{error, info};

use crate::http::extractors::AppState;
#[cfg(feature = "metrics")]
use crate::http::routes::metrics;
use crate::http::routes::{
    get_object, head_object, health, oap_object_get, paid_estimate, paid_object, put_object, ready,
    version,
};

/// Build a router with an empty moderation snapshot.
pub fn build_router() -> Router<AppState> {
    build_router_with_moderation(Arc::new(ModerationPolicy::default()))
}

/// Build a router with an explicit immutable moderation snapshot.
pub fn build_router_with_moderation(moderation: Arc<ModerationPolicy>) -> Router<AppState> {
    let api = Router::new()
        // Free/dev object APIs: accept both PUT and POST for ingest.
        .route("/o", put(put_object::handler).post(put_object::handler))
        .route(
            "/o/:cid",
            head(head_object::handler).get(get_object::handler),
        )
        // Binary OAP/1 OBJ_GET request → START/DATA/END stream.
        .route("/oap/obj-get", post(oap_object_get::handler))
        // Read-only paid-storage price estimate.
        .route("/paid/o/estimate", get(paid_estimate::handler))
        // Paid writes require an admitted payment proof.
        .route(
            "/paid/o",
            put(paid_object::handler).post(paid_object::handler),
        )
        .route("/version", get(version::handler))
        .route("/healthz", get(health::handler))
        .route("/readyz", get(ready::handler));

    #[cfg(feature = "metrics")]
    let api = api.route("/metrics", get(metrics::handler));

    let app = Router::new().merge(api).layer(Extension(moderation));

    info!(
        "mount: POST/PUT /o; POST /oap/obj-get; \
         GET /paid/o/estimate; POST/PUT /paid/o; \
         HEAD/GET /o/:cid; GET /version; GET /healthz; \
         GET /readyz{}",
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

/// Bind and run the default empty-moderation storage HTTP server.
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
