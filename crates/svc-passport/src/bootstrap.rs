//! RO:WHAT — Build routers, bind HTTP (public + admin), expose metrics/health/ready.
//! RO:WHY  — Keep main.rs tiny; centralize boot logic.
//! RO:INTERACTS — http::router, telemetry::prometheus (default registry served via axum)

use crate::{health::Health, http::router::build_router, telemetry::prometheus as prom, Config};
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::task::JoinHandle;
use tracing::info;

pub async fn run(
    bind: SocketAddr,
    admin_bind: SocketAddr,
    cfg: Config,
) -> anyhow::Result<((JoinHandle<()>), SocketAddr)> {
    let health = Health::new();

    // mark service healthy; readiness will flip when router wires KMS cache
    health.health.set("svc-passport", true);

    let app = build_router(cfg.clone(), health.clone());

    // Admin plane: /metrics, /healthz, /readyz (from default registry)
    let admin =
        Router::new()
            .route("/metrics", get(prom::metrics_handler))
            .route(
                "/healthz",
                get({
                    let h = health.health.clone();
                    move || prom::healthz_handler(h.clone())
                }),
            )
            .route(
                "/readyz",
                get({
                    let r = health.ready.clone();
                    move || async move {
                        ron_kernel::metrics::readiness::readyz_handler(r.clone()).await
                    }
                }),
            );

    let server = axum::serve(
        tokio::net::TcpListener::bind(bind).await?,
        app.into_make_service(),
    );
    let admin_srv = axum::serve(
        tokio::net::TcpListener::bind(admin_bind).await?,
        admin.into_make_service(),
    );

    let addr = server.local_addr();
    let h1 = tokio::spawn(server);
    let _h2 = tokio::spawn(admin_srv);

    info!(%addr, "http bound");
    Ok((h1, addr))
}
