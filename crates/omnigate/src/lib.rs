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
use std::time::Duration;
use tracing::{info, warn};

// ron-policy types (bundle held in Extension; evaluator built per-request)
use ron_policy::PolicyBundle;

// used for tolerant parsing & diagnostics
use serde_json::{Map, Value};

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
            .route("/readyz", get(readyz))
            .route("/healthz", get(healthz))
            .with_state(admin_state);

        // Base router (no middleware yet) — immutable is fine here.
        let app_router = Router::new().merge(roots).merge(ops).nest("/v1", api_v1);

        // -------------------- POLICY BUNDLE LOAD (hold for later layering) --------------------
        let mut policy_bundle_arc: Option<Arc<PolicyBundle>> = None;
        if cfg.policy.enabled {
            match std::fs::read_to_string(&cfg.policy.bundle_path) {
                Ok(json) => {
                    // First try: strict parse into ron_policy::PolicyBundle.
                    match serde_json::from_str::<PolicyBundle>(&json) {
                        Ok(bundle) => {
                            policy_bundle_arc = Some(Arc::new(bundle));
                            crate::metrics::registry::POLICY_BUNDLE_LOADED_TOTAL.inc();
                            info!(path=%cfg.policy.bundle_path, "policy bundle loaded and inserted");
                        }
                        Err(e1) => {
                            // DEV DIAGNOSTIC: log top-level keys we actually have.
                            let top_keys =
                                serde_json::from_str::<Value>(&json).ok().and_then(|v| {
                                    v.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>())
                                });

                            warn!(error=?e1, ?top_keys, path=%cfg.policy.bundle_path, "failed to parse policy bundle (strict)");

                            // Try tolerant normalization.
                            match serde_json::from_str::<Value>(&json).ok().and_then(|mut v| {
                                normalize_policy_value(&mut v);
                                serde_json::from_value::<PolicyBundle>(v).ok()
                            }) {
                                Some(bundle) => {
                                    policy_bundle_arc = Some(Arc::new(bundle));
                                    crate::metrics::registry::POLICY_BUNDLE_LOADED_TOTAL.inc();
                                    info!(path=%cfg.policy.bundle_path, "policy bundle loaded via normalized schema");
                                }
                                None => {
                                    // Final diagnostic: show normalized keys.
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
                    }
                }
                Err(e) => {
                    warn!(error=?e, path=%cfg.policy.bundle_path, "failed to read policy bundle; PolicyLayer will pass-through");
                }
            }
        } else {
            info!("policy disabled in config; PolicyLayer will no-op");
        }

        // -------------------- MIDDLEWARE STACK --------------------
        // Build core middleware (includes PolicyLayer).
        let app_router = middleware::apply_with_cfg(app_router, &cfg.admission)
            .layer(observability::http_trace_layer());

        // Attach admission with *config-driven* limits (still inside).
        let mut app_router = crate::admission::attach_with_cfg(app_router, &cfg.admission);

        // IMPORTANT: Layer Extension(bundle) *after* the middleware so it runs first at request time.
        if let Some(bundle) = policy_bundle_arc {
            app_router = app_router.layer(Extension(bundle));
        }

        // -------------------- READINESS SAMPLER (lightweight; metrics-only for now) --------------------
        // Rolls 429/503 error deltas into READY_ERROR_RATE_PCT each window.
        {
            use crate::metrics::gates::POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL;
            use crate::metrics::gates::READY_ERROR_RATE_PCT;
            // These two live at crate::metrics root in your tree:
            use crate::metrics::{ADMISSION_QUOTA_EXHAUSTED_TOTAL, FAIR_Q_EVENTS_TOTAL};

            let window_secs = cfg.readiness.window_secs.max(1);
            tokio::spawn(async move {
                let mut last_quota = {
                    let g = ADMISSION_QUOTA_EXHAUSTED_TOTAL
                        .with_label_values(&["global"])
                        .get();
                    let i = ADMISSION_QUOTA_EXHAUSTED_TOTAL
                        .with_label_values(&["ip"])
                        .get();
                    g + i
                } as f64;
                let mut last_policy_503 = POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL
                    .with_label_values(&["503"])
                    .get() as f64;
                let mut last_fair_drops =
                    FAIR_Q_EVENTS_TOTAL.with_label_values(&["dropped"]).get() as f64;

                loop {
                    tokio::time::sleep(Duration::from_secs(window_secs)).await;

                    let quota_now = {
                        let g = ADMISSION_QUOTA_EXHAUSTED_TOTAL
                            .with_label_values(&["global"])
                            .get();
                        let i = ADMISSION_QUOTA_EXHAUSTED_TOTAL
                            .with_label_values(&["ip"])
                            .get();
                        g + i
                    } as f64;
                    let policy_503_now = POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL
                        .with_label_values(&["503"])
                        .get() as f64;
                    let fair_drops_now =
                        FAIR_Q_EVENTS_TOTAL.with_label_values(&["dropped"]).get() as f64;

                    let d_quota = (quota_now - last_quota).max(0.0);
                    let d_p503 = (policy_503_now - last_policy_503).max(0.0);
                    let d_drops = (fair_drops_now - last_fair_drops).max(0.0);

                    // Treat (429 + 503) frequency as a crude error-rate proxy over the window.
                    // We don't have a total-requests counter here, so publish the per-window
                    // "events per second" as a percentage-like gauge scaled to window.
                    let err_events = d_quota + d_p503 + d_drops;
                    let per_sec = err_events / (window_secs as f64);
                    let pct_like = (per_sec * 100.0).min(100.0);
                    READY_ERROR_RATE_PCT.set(pct_like);

                    last_quota = quota_now;
                    last_policy_503 = policy_503_now;
                    last_fair_drops = fair_drops_now;
                }
            });
        }

        Ok(Self {
            router: app_router,
            admin_addr,
        })
    }
}

/// Normalize JSON from various shapes into the strict ron_policy::PolicyBundle schema.
/// - defaults.default_action = "allow" | "deny"
/// - rule.action            = "allow" | "deny"
///   We accept older or Grok-like shapes and rewrite to the strict form.
fn normalize_policy_value(root: &mut Value) {
    // Top level must be an object
    let obj = match root.as_object_mut() {
        Some(m) => m,
        None => return,
    };

    // version: accept "1" as string and coerce to number, or default to 1
    if let Some(v) = obj.get_mut("version") {
        if v.is_string() {
            if let Ok(n) = v.as_str().unwrap_or_default().parse::<u32>() {
                *v = Value::Number(serde_json::Number::from(n));
            }
        }
    } else {
        obj.insert("version".to_string(), Value::Number(1u32.into()));
    }

    // meta: move description into meta.name (optional)
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

    // defaults:
    // Accept any of: { default: "allow" }, { defaults: { default_action: "deny" } }, { defaults: { effect: "allow" } }
    // Emit strict:   { defaults: { default_action: "allow" | "deny", ... passthrough ... } }
    let mut defaults_obj = obj
        .remove("defaults")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_else(Map::new);

    // lift top-level "default" to defaults.default_action
    if let Some(def) = obj.remove("default") {
        defaults_obj
            .entry("default_action".to_string())
            .or_insert(def);
    }

    // convert legacy/effect -> default_action if present
    if let Some(v) = defaults_obj.remove("effect") {
        defaults_obj
            .entry("default_action".to_string())
            .or_insert(v);
    }

    // ensure default_action exists (fallback = "deny")
    defaults_obj
        .entry("default_action".to_string())
        .or_insert(Value::String("deny".to_string()));

    // keep other known defaults fields (e.g., max_body_bytes, decompress_max_ratio) as-is
    obj.insert("defaults".to_string(), Value::Object(defaults_obj));

    // rules: ensure array; map effect<->action so "action" is present
    let mut rules = obj
        .remove("rules")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_else(Vec::new);

    for r in &mut rules {
        if let Some(ro) = r.as_object_mut() {
            // If rule has "effect", copy/move to "action" (strict wants "action")
            if let Some(eff) = ro.remove("effect") {
                ro.entry("action".to_string()).or_insert(eff);
            }
            // If rule has neither, leave as-is; strict parse will report precisely.
        }
    }
    obj.insert("rules".to_string(), Value::Array(rules));
}
