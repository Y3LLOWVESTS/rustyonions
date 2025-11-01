//! RO:WHAT — Omnigate service library: bootstrap, config, admin plane, and Router wiring.
//! RO:WHY  — P6 Ingress/App BFF foundation; Concerns: SEC/RES/PERF/GOV.
//! RO:INVARIANTS — no locks across .await; single writer per conn.

pub mod admission;
pub mod bootstrap;
pub mod config;
pub mod errors;
pub mod metrics;
pub mod middleware;
pub mod observability;
pub mod routes;
pub mod runtime;
pub mod types;
pub mod zk;

use axum::{extract::State, response::IntoResponse, routing::get, Router};
use ron_kernel::metrics::{health::HealthState, readiness::Readiness};
use ron_kernel::Metrics;
use std::net::SocketAddr;
use tracing::info;

#[derive(Clone)]
struct AdminState {
    health: HealthState,
    ready: Readiness,
    dev_ready: bool,
}

pub struct App {
    pub router: Router,
    pub admin_addr: SocketAddr,
}

impl App {
    /// Build the main app router and start the admin plane (metrics/health/ready).
    pub async fn build(cfg: config::Config) -> anyhow::Result<Self> {
        // ---- Resolve amnesia (cfg + optional OMNIGATE_AMNESIA override for local smoke) ----
        let amnesia_from_cfg = cfg.server.amnesia;
        let amnesia_from_env = matches!(
            std::env::var("OMNIGATE_AMNESIA").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE") | Ok("on") | Ok("ON")
        );
        let amnesia_on = amnesia_from_cfg || amnesia_from_env;
        info!(
            amnesia_from_cfg,
            amnesia_from_env, amnesia_on, "amnesia mode resolved"
        );

        // ---- Boot kernel metrics/admin plane ----
        let metrics = Metrics::new(false);

        // IMPORTANT: flip the KERNEL'S amnesia gauge on the SAME registry that serves /metrics.
        // This is what your sanity script scrapes.
        metrics.set_amnesia(amnesia_on);

        let health = HealthState::new();
        let ready = Readiness::new(health.clone());

        // Serve admin plane (Prometheus /metrics + /healthz + /readyz) on cfg.server.metrics_addr.
        let (_admin_task, admin_addr) = metrics
            .clone()
            .serve(cfg.server.metrics_addr, health.clone(), ready.clone())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Liveness: process is up & config parsed.
        health.set("omnigate", true);
        health.set("config", true);

        // Readiness: flip the specific "config loaded" gate (what /readyz checks).
        ready.set_config_loaded(true);

        // One-shot policy load notice (counter on default registry won’t show on /metrics; keep log only)
        info!("policy bundle loaded");

        // Dev-ready override (read once)
        let dev_ready = matches!(
            std::env::var("OMNIGATE_DEV_READY").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE") | Ok("on") | Ok("ON")
        );
        if dev_ready {
            info!("OMNIGATE_DEV_READY=on — /readyz will report 200 (dev override)");
        }

        let admin_state = AdminState {
            health: health.clone(),
            ready: ready.clone(),
            dev_ready,
        };

        // v1 API (expand in routes/v1/*)
        let api_v1 = Router::new().route("/ping", get(routes::v1::ping));

        // Shared handlers (root and /ops use the same functions)
        async fn healthz(State(st): State<AdminState>) -> impl IntoResponse {
            ron_kernel::metrics::health::healthz_handler(st.health.clone()).await
        }
        async fn readyz(State(st): State<AdminState>) -> impl IntoResponse {
            if st.dev_ready {
                return (axum::http::StatusCode::OK, "ready (dev override)").into_response();
            }
            ron_kernel::metrics::readiness::readyz_handler(st.ready.clone()).await
        }

        // Ops routes (namespaced)
        let ops = Router::new()
            .route("/ops/version", get(routes::ops::version)) // back-compat
            .route("/ops/readyz", get(readyz))
            .route("/ops/healthz", get(healthz))
            .with_state(admin_state.clone());

        // Root aliases (+ /versionz for sanity script/tools)
        let roots = Router::new()
            .route("/versionz", get(routes::ops::versionz))
            .route("/readyz", get(readyz))
            .route("/healthz", get(healthz))
            .with_state(admin_state);

        // Base router → root aliases + /ops + versioned API
        let app_router = Router::new().merge(roots).merge(ops).nest("/v1", api_v1);

        // Middleware stack (corr-id → classify → decompress_guard → body_caps → slow_loris) + HTTP tracing.
        let app_router = middleware::apply(app_router).layer(observability::http_trace_layer());

        Ok(Self {
            router: app_router,
            admin_addr,
        })
    }
}
