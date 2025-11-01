// crates/omnigate/src/lib.rs
#![allow(clippy::needless_return)]

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

use axum::Extension;
use axum::{extract::State, response::IntoResponse, routing::get, Router};
use ron_kernel::metrics::{health::HealthState, readiness::Readiness};
use ron_kernel::Metrics;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, warn};

// ron-policy types (bundle held in Extension; evaluator built per-request)
use ron_policy::PolicyBundle;

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
        metrics.set_amnesia(amnesia_on);

        let health = HealthState::new();
        let ready = Readiness::new(health.clone());

        let (_admin_task, admin_addr) = metrics
            .clone()
            .serve(cfg.server.metrics_addr, health.clone(), ready.clone())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        health.set("omnigate", true);
        health.set("config", true);
        ready.set_config_loaded(true);

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

        // -------------------- ROUTES --------------------
        let api_v1 = Router::new().route("/ping", get(crate::routes::v1::index::ping));

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
            .route("/ops/version", get(crate::routes::ops::version))
            .route("/ops/readyz", get(readyz))
            .route("/ops/healthz", get(healthz))
            .with_state(admin_state.clone());

        let roots = Router::new()
            .route("/versionz", get(crate::routes::ops::versionz))
            .route("/readyz", get(readyz))
            .route("/healthz", get(healthz))
            .with_state(admin_state);

        // Base router (no middleware yet)
        let mut app_router = Router::new().merge(roots).merge(ops).nest("/v1", api_v1);

        // -------------------- POLICY BUNDLE INJECTION --------------------
        // Load & hold the PolicyBundle (owned, Arc) in request extensions.
        // The middleware will build an Evaluator borrowing this bundle per request.
        if cfg.policy.enabled {
            match std::fs::read_to_string(&cfg.policy.bundle_path) {
                Ok(json) => match serde_json::from_str::<PolicyBundle>(&json) {
                    Ok(bundle) => {
                        let bundle = Arc::new(bundle);
                        app_router = app_router.layer(Extension(bundle));
                        crate::metrics::registry::POLICY_BUNDLE_LOADED_TOTAL.inc();
                        info!(path=%cfg.policy.bundle_path, "policy bundle loaded and inserted");
                    }
                    Err(e) => {
                        warn!(error=?e, path=%cfg.policy.bundle_path, "failed to parse policy bundle; PolicyLayer will pass-through");
                    }
                },
                Err(e) => {
                    warn!(error=?e, path=%cfg.policy.bundle_path, "failed to read policy bundle; PolicyLayer will pass-through");
                }
            }
        } else {
            info!("policy disabled in config; PolicyLayer will no-op");
        }

        // -------------------- MIDDLEWARE STACK --------------------
        // Put PolicyLayer after the Extension so it can see the bundle.
        let app_router = middleware::apply(app_router).layer(observability::http_trace_layer());

        Ok(Self {
            router: app_router,
            admin_addr,
        })
    }
}
