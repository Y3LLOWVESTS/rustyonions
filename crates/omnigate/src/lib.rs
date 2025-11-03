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
// NEW: factor readiness into its own module
pub mod readiness;

use axum::{extract::State, response::IntoResponse, routing::get, Extension, Router};
use ron_kernel::metrics::{health::HealthState, readiness::Readiness as KernelReadiness};
use ron_kernel::Metrics;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, warn};

// ron-policy types (bundle held in Extension; evaluator built per-request)
use ron_policy::PolicyBundle;

// used for tolerant parsing & diagnostics
use serde_json::{Map, Value};

#[derive(Clone)]
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

        // Ensure gate metrics are registered before first scrape.
        crate::metrics::gates::init_gate_metrics();

        let health = HealthState::new();
        let kernel_ready = KernelReadiness::new(health.clone());

        let (_admin_task, admin_addr) = metrics
            .clone()
            .serve(
                cfg.server.metrics_addr,
                health.clone(),
                kernel_ready.clone(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        health.set("omnigate", true);
        health.set("config", true);
        kernel_ready.set_config_loaded(true);

        // Dev-ready override (read once)
        let dev_ready = matches!(
            std::env::var("OMNIGATE_DEV_READY").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE") | Ok("on") | Ok("ON")
        );
        if dev_ready {
            info!("OMNIGATE_DEV_READY=on — /readyz will report 200 (dev override)");
        }

        // ---- Local readiness bridge (truth source for /readyz) ----
        let rp = Arc::new(readiness::policy::ReadyPolicy::new());
        let admin_state = readiness::state::AdminState::new(
            health.clone(),
            kernel_ready.clone(),
            dev_ready,
            &cfg.readiness,
            rp.clone(),
        );

        // -------------------- ROUTES --------------------
        let api_v1 = crate::routes::v1::index::router();

        async fn healthz(State(st): State<readiness::state::AdminState>) -> impl IntoResponse {
            ron_kernel::metrics::health::healthz_handler(st.health.clone()).await
        }

        let ops = Router::new()
            .route("/ops/version", get(crate::routes::ops::version))
            .route("/ops/readyz", get(readiness::readyz))
            .route("/ops/healthz", get(healthz))
            .route(
                "/ops/metrics",
                get(|| async {
                    // Expose the default Prometheus registry used by gate counters.
                    use prometheus::TextEncoder;
                    let encoder = TextEncoder::new();
                    let mfs = prometheus::gather();
                    encoder.encode_to_string(&mfs).unwrap_or_default()
                }),
            )
            .with_state(admin_state.clone());

        let roots = Router::new()
            .route("/versionz", get(crate::routes::ops::versionz))
            .route("/readyz", get(readiness::readyz))
            .route("/healthz", get(healthz))
            .with_state(admin_state);

        // Base router (no middleware yet)
        let mut app_router = Router::new().merge(roots).merge(ops).nest("/v1", api_v1);

        // -------------------- POLICY BUNDLE LOAD --------------------
        if cfg.policy.enabled {
            match std::fs::read_to_string(&cfg.policy.bundle_path) {
                Ok(json) => match serde_json::from_str::<PolicyBundle>(&json) {
                    Ok(bundle) => {
                        crate::metrics::registry::POLICY_BUNDLE_LOADED_TOTAL.inc();
                        info!(path=%cfg.policy.bundle_path, "policy bundle loaded and inserted");
                        app_router = app_router.layer(Extension(Arc::new(bundle)));
                    }
                    Err(e1) => {
                        let top_keys = serde_json::from_str::<Value>(&json).ok().and_then(|v| {
                            v.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>())
                        });

                        warn!(error=?e1, ?top_keys, path=%cfg.policy.bundle_path, "failed to parse policy bundle (strict)");

                        match serde_json::from_str::<Value>(&json).ok().and_then(|mut v| {
                            normalize_policy_value(&mut v);
                            serde_json::from_value::<PolicyBundle>(v).ok()
                        }) {
                            Some(bundle) => {
                                crate::metrics::registry::POLICY_BUNDLE_LOADED_TOTAL.inc();
                                info!(path=%cfg.policy.bundle_path, "policy bundle loaded via normalized schema");
                                app_router = app_router.layer(Extension(Arc::new(bundle)));
                            }
                            None => {
                                if let Ok(mut v) = serde_json::from_str::<Value>(&json) {
                                    normalize_policy_value(&mut v);
                                    let norm_keys = v
                                        .as_object()
                                        .map(|o| o.keys().cloned().collect::<Vec<_>>());
                                    warn!(path=%cfg.policy.bundle_path, ?norm_keys, "policy bundle still failed after normalization; PolicyLayer will pass-through");
                                } else {
                                    warn!(path=%cfg.policy.bundle_path, "policy bundle is not valid JSON; PolicyLayer will pass-through");
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    warn!(error=?e, path=%cfg.policy.bundle_path, "failed to read policy bundle; PolicyLayer will pass-through");
                }
            }
        } else {
            info!("policy disabled in config; PolicyLayer will no-op");
        }

        // -------------------- HTTP MIDDLEWARE + ADMISSION --------------------
        app_router = middleware::apply_with_cfg(app_router, &cfg.admission)
            .layer(observability::http_trace_layer());

        app_router = crate::admission::attach_with_cfg(app_router, &cfg.admission);

        // ---- GLOBAL INFLIGHT BRIDGE (outermost so it sees ALL requests) ----
        app_router = middleware::inflight::attach(app_router, rp.clone());

        // -------------------- READINESS SAMPLER (error-rate rolling window) --------------------
        readiness::sampler::spawn_err_rate_sampler(rp.clone(), cfg.readiness.window_secs);

        Ok(Self {
            router: app_router,
            admin_addr,
        })
    }
}

/// Normalize JSON from various shapes into the strict ron_policy::PolicyBundle schema.
fn normalize_policy_value(root: &mut Value) {
    let obj = match root.as_object_mut() {
        Some(m) => m,
        None => return,
    };
    if let Some(v) = obj.get_mut("version") {
        if v.is_string() {
            if let Ok(n) = v.as_str().unwrap_or_default().parse::<u32>() {
                *v = Value::Number(serde_json::Number::from(n));
            }
        }
    } else {
        obj.insert("version".to_string(), Value::Number(1u32.into()));
    }
    if let Some(desc) = obj.remove("description") {
        let meta = obj
            .entry("meta")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(mo) = meta.as_object_mut() {
            mo.entry("name".to_string()).or_insert(desc);
        }
    } else {
        obj.entry("meta")
            .or_insert_with(|| Value::Object(Map::new()));
    }
    let mut defaults_obj = obj
        .remove("defaults")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    if let Some(def) = obj.remove("default") {
        defaults_obj
            .entry("default_action".to_string())
            .or_insert(def);
    }
    if let Some(v) = defaults_obj.remove("effect") {
        defaults_obj
            .entry("default_action".to_string())
            .or_insert(v);
    }
    defaults_obj
        .entry("default_action".to_string())
        .or_insert(Value::String("deny".to_string()));
    obj.insert("defaults".to_string(), Value::Object(defaults_obj));

    let mut rules = obj
        .remove("rules")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_else(Vec::new);
    for r in &mut rules {
        if let Some(ro) = r.as_object_mut() {
            if let Some(eff) = ro.remove("effect") {
                ro.entry("action".to_string()).or_insert(eff);
            }
        }
    }
    obj.insert("rules".to_string(), Value::Array(rules));
}
