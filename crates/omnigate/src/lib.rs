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
    pub async fn build(cfg: config::Config) -> anyhow::Result<Self> {
        // ---------- Resolve amnesia first, and tell the kernel via env ----------
        // Source of truth: config with optional OMNIGATE_AMNESIA env override (for local smoke).
        let amnesia_from_cfg = cfg.server.amnesia;
        let amnesia_from_env = matches!(
            std::env::var("OMNIGATE_AMNESIA").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE") | Ok("on") | Ok("ON")
        );
        let amnesia_on = amnesia_from_cfg || amnesia_from_env;

        // Bridge to kernel until we expose a typed builder here:
        // RON_AMNESIA=on|off is read by ron-kernel at metrics/bootstrap time.
        if amnesia_on {
            std::env::set_var("RON_AMNESIA", "on");
        } else {
            // ensure it's not stuck from a previous run in the same shell
            std::env::remove_var("RON_AMNESIA");
        }
        info!(
            amnesia_from_cfg,
            amnesia_from_env, amnesia_on, "amnesia mode resolved"
        );

        // ---------- Now boot kernel metrics/admin plane ----------
        let metrics = Metrics::new(false);
        let health = HealthState::new();
        let ready = Readiness::new(health.clone());

        let (_admin_task, admin_addr) = metrics
            .clone()
            .serve(cfg.server.metrics_addr, health.clone(), ready.clone())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Liveness and initial readiness
        health.set("omnigate", true);
        health.set("config", true);
        ready.set_config_loaded(true);

        // (Do NOT set a separate omnigate-local amnesia gauge; the kernel’s registry is what /metrics serves.)

        // Emit a one-shot “policy bundle loaded” log; the counter lives on our default registry
        // and won’t appear on /metrics served by the kernel, so we keep the log only for now.
        info!("policy bundle loaded");

        // Dev-ready override (truthful /readyz is still controlled by Readiness gates)
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

        let api_v1 = Router::new().route("/ping", get(routes::v1::ping));

        async fn healthz(State(st): State<AdminState>) -> impl IntoResponse {
            ron_kernel::metrics::health::healthz_handler(st.health.clone()).await
        }
        async fn readyz(State(st): State<AdminState>) -> impl IntoResponse {
            if st.dev_ready {
                return (axum::http::StatusCode::OK, "ready (dev override)").into_response();
            }
            ron_kernel::metrics::readiness::readyz_handler(st.ready.clone()).await
        }

        let ops = Router::new()
            .route("/ops/version", get(routes::ops::version))
            .route("/ops/readyz", get(readyz))
            .route("/ops/healthz", get(healthz))
            .with_state(admin_state.clone());

        let roots = Router::new()
            .route("/versionz", get(routes::ops::versionz))
            .route("/readyz", get(readyz))
            .route("/healthz", get(healthz))
            .with_state(admin_state);

        let app_router = Router::new().merge(roots).merge(ops).nest("/v1", api_v1);

        let app_router = middleware::apply(app_router).layer(observability::http_trace_layer());

        Ok(Self {
            router: app_router,
            admin_addr,
        })
    }
}
