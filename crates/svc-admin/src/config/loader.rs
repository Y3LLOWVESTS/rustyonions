// crates/svc-admin/src/config/loader.rs
//
// WHAT: Env-based configuration loading and validation for Config.
// WHY: Keeps parsing and invariants separate from the type definitions.
//
// NODES (truthful):
//   Supports multi-node env registry via:
//     SVC_ADMIN_NODES_CLEAR=1
//     SVC_ADMIN_NODES__<ID>__BASE_URL=http://127.0.0.1:8080
//     SVC_ADMIN_NODES__<ID>__DISPLAY_NAME=Macronode
//     SVC_ADMIN_NODES__<ID>__ENVIRONMENT=dev
//     SVC_ADMIN_NODES__<ID>__INSECURE_HTTP=true
//     SVC_ADMIN_NODES__<ID>__FORCED_PROFILE=macronode
//     SVC_ADMIN_NODES__<ID>__MACAROON_PATH=/path/to/macaroon (optional)
//     SVC_ADMIN_NODES__<ID>__DEFAULT_TIMEOUT=2 (seconds, optional; best-effort)
//
// Legacy compatibility (dev convenience):
//   SVC_ADMIN_EXAMPLE_NODE_URL / SVC_ADMIN_EXAMPLE_NODE_ENV
//
// AUTH (modes):
//   SVC_ADMIN_AUTH_MODE=none|ingress|passport|local
//
// Local auth env knobs (mode=local):
//   SVC_ADMIN_AUTH_COOKIE_NAME
//   SVC_ADMIN_AUTH_COOKIE_SECURE
//   SVC_ADMIN_AUTH_SESSION_TTL_SEC
//   SVC_ADMIN_AUTH_SESSION_IDLE_TTL_SEC
//   SVC_ADMIN_AUTH_RBAC_PATH
//   SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_USERNAME
//   SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_PASSWORD_ENV

use crate::error::{Error, Result};
use std::{collections::BTreeMap, env, net::SocketAddr, path::PathBuf, time::Duration};

use super::{nodes::NodeCfg, Config};

impl Config {
    /// Load config from environment only (for now).
    ///
    /// File- and CLI-based config will be layered on in a later step.
    pub fn load() -> Result<Self> {
        if env::var_os("SVC_ADMIN_CONFIG").is_some() {
            // Guardrail until file loading is implemented.
            return Err(Error::Config(
                "SVC_ADMIN_CONFIG is not yet supported (file-based config TODO)".to_string(),
            ));
        }

        let mut cfg = Config::default();

        // --- Server / listener ---

        cfg.server.bind_addr = load_addr("SVC_ADMIN_BIND_ADDR", &cfg.server.bind_addr)?;
        cfg.server.metrics_addr = load_addr("SVC_ADMIN_METRICS_ADDR", &cfg.server.metrics_addr)?;

        cfg.server.max_conns = load_usize("SVC_ADMIN_MAX_CONNS", cfg.server.max_conns)?;

        cfg.server.read_timeout = load_duration("SVC_ADMIN_READ_TIMEOUT", cfg.server.read_timeout)?;
        cfg.server.write_timeout =
            load_duration("SVC_ADMIN_WRITE_TIMEOUT", cfg.server.write_timeout)?;
        cfg.server.idle_timeout = load_duration("SVC_ADMIN_IDLE_TIMEOUT", cfg.server.idle_timeout)?;

        cfg.server.tls.enabled = load_bool("SVC_ADMIN_TLS_ENABLED", cfg.server.tls.enabled)?;
        cfg.server.tls.cert_path =
            load_opt_path("SVC_ADMIN_TLS_CERT_PATH", cfg.server.tls.cert_path);
        cfg.server.tls.key_path = load_opt_path("SVC_ADMIN_TLS_KEY_PATH", cfg.server.tls.key_path);

        // --- Logging ---

        if let Ok(fmt) = env::var("SVC_ADMIN_LOG_FORMAT") {
            cfg.log.format = fmt;
        }
        if let Ok(level) = env::var("SVC_ADMIN_LOG_LEVEL") {
            cfg.log.level = level;
        }

        // --- Polling / metrics ---

        cfg.polling.metrics_interval = load_duration(
            "SVC_ADMIN_POLLING_METRICS_INTERVAL",
            cfg.polling.metrics_interval,
        )?;
        cfg.polling.metrics_window = load_duration(
            "SVC_ADMIN_POLLING_METRICS_WINDOW",
            cfg.polling.metrics_window,
        )?;

        // --- UI defaults ---

        // Support both new and old names for themes/language to keep dev UX smooth.
        if let Ok(theme) = env::var("SVC_ADMIN_UI_THEME") {
            cfg.ui.default_theme = theme;
        } else if let Ok(theme) = env::var("SVC_ADMIN_UI_DEFAULT_THEME") {
            cfg.ui.default_theme = theme;
        }

        if let Ok(lang) = env::var("SVC_ADMIN_UI_LANGUAGE") {
            cfg.ui.default_language = lang;
        } else if let Ok(lang) = env::var("SVC_ADMIN_UI_DEFAULT_LANGUAGE") {
            cfg.ui.default_language = lang;
        }

        cfg.ui.read_only = load_bool("SVC_ADMIN_UI_READ_ONLY", cfg.ui.read_only)?;
        cfg.ui.dev.enable_app_playground = load_bool(
            "SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND",
            cfg.ui.dev.enable_app_playground,
        )?;

        // --- Actions ---

        cfg.actions.enable_reload =
            load_bool("SVC_ADMIN_ACTIONS_ENABLE_RELOAD", cfg.actions.enable_reload)?;
        cfg.actions.enable_shutdown = load_bool(
            "SVC_ADMIN_ACTIONS_ENABLE_SHUTDOWN",
            cfg.actions.enable_shutdown,
        )?;

        // --- Auth ---

        cfg.auth.mode = load_auth_mode("SVC_ADMIN_AUTH_MODE", &cfg.auth.mode)?;

        if let Ok(issuer) = env::var("SVC_ADMIN_AUTH_PASSPORT_ISSUER") {
            cfg.auth.passport_issuer = Some(issuer);
        }
        if let Ok(aud) = env::var("SVC_ADMIN_AUTH_PASSPORT_AUDIENCE") {
            cfg.auth.passport_audience = Some(aud);
        }
        if let Ok(jwks) = env::var("SVC_ADMIN_AUTH_PASSPORT_JWKS_URL") {
            cfg.auth.passport_jwks_url = Some(jwks);
        }

        // Local auth knobs (loaded regardless of mode; used only when mode="local")
        if let Ok(v) = env::var("SVC_ADMIN_AUTH_COOKIE_NAME") {
            cfg.auth.cookie_name = Some(v);
        }
        if let Ok(v) = env::var("SVC_ADMIN_AUTH_COOKIE_SECURE") {
            cfg.auth.cookie_secure = Some(parse_bool("SVC_ADMIN_AUTH_COOKIE_SECURE", &v)?);
        }
        if let Ok(v) = env::var("SVC_ADMIN_AUTH_SESSION_TTL_SEC") {
            let secs: u64 = v.parse().map_err(|e| {
                Error::Config(format!(
                    "invalid duration seconds for SVC_ADMIN_AUTH_SESSION_TTL_SEC: {} ({e})",
                    v
                ))
            })?;
            cfg.auth.session_ttl_sec = Some(secs);
        }
        if let Ok(v) = env::var("SVC_ADMIN_AUTH_SESSION_IDLE_TTL_SEC") {
            let secs: u64 = v.parse().map_err(|e| {
                Error::Config(format!(
                    "invalid duration seconds for SVC_ADMIN_AUTH_SESSION_IDLE_TTL_SEC: {} ({e})",
                    v
                ))
            })?;
            cfg.auth.session_idle_ttl_sec = Some(secs);
        }
        if let Ok(v) = env::var("SVC_ADMIN_AUTH_RBAC_PATH") {
            cfg.auth.rbac_path = Some(v);
        }
        if let Ok(v) = env::var("SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_USERNAME") {
            cfg.auth.bootstrap_admin_username = Some(v);
        }
        if let Ok(v) = env::var("SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_PASSWORD_ENV") {
            cfg.auth.bootstrap_admin_password_env = Some(v);
        }

        // --- Nodes (truthful, multi-node) ---

        apply_nodes_from_env(&mut cfg)?;

        cfg.validate()?;
        Ok(cfg)
    }

    /// Basic invariants for Config v2.
    fn validate(&self) -> Result<()> {
        if self.server.max_conns == 0 {
            return Err(Error::Config("server.max_conns must be > 0".to_string()));
        }

        if self.polling.metrics_interval == Duration::from_secs(0) {
            return Err(Error::Config(
                "polling.metrics_interval must be > 0s".to_string(),
            ));
        }

        if self.polling.metrics_window < self.polling.metrics_interval {
            return Err(Error::Config(
                "polling.metrics_window must be >= polling.metrics_interval".to_string(),
            ));
        }

        if self.server.tls.enabled
            && (self.server.tls.cert_path.is_none() || self.server.tls.key_path.is_none())
        {
            return Err(Error::Config(
                "TLS is enabled but cert_path/key_path are not both set".to_string(),
            ));
        }

        // Guardrail: auth=none is dev-only, must bind loopback.
        if self.auth.mode == "none" {
            let bind: SocketAddr = self.server.bind_addr.parse().map_err(|e| {
                Error::Config(format!("invalid socket addr for server.bind_addr: {e}"))
            })?;
            if !bind.ip().is_loopback() {
                return Err(Error::Config(
                    "auth.mode=none is dev-only and must bind to loopback (127.0.0.1 / ::1)"
                        .to_string(),
                ));
            }
        }

        // Local auth must have a usable RBAC path (can be created if missing, but not empty).
        if self.auth.mode == "local" {
            let rp = self.auth.rbac_path.as_deref().unwrap_or("");
            if rp.trim().is_empty() {
                return Err(Error::Config(
                    "auth.mode=local requires auth.rbac_path (or SVC_ADMIN_AUTH_RBAC_PATH)"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}

// --- nodes: env loader ------------------------------------------------------

#[derive(Debug, Default, Clone)]
struct NodePatch {
    base_url: Option<String>,
    display_name: Option<String>,
    environment: Option<String>,
    insecure_http: Option<bool>,
    forced_profile: Option<String>,
    macaroon_path: Option<PathBuf>,
    default_timeout: Option<Duration>,
}

fn apply_nodes_from_env(cfg: &mut Config) -> Result<()> {
    // If set, wipe whatever is already in cfg.nodes before applying env.
    let clear = load_bool("SVC_ADMIN_NODES_CLEAR", false)?;
    if clear {
        cfg.nodes.clear();
    }

    // Parse SVC_ADMIN_NODES__<ID>__<FIELD>=...
    let mut patches: BTreeMap<String, NodePatch> = BTreeMap::new();

    for (k, v) in env::vars() {
        let rest = match k.strip_prefix("SVC_ADMIN_NODES__") {
            Some(r) => r,
            None => continue,
        };

        // Expect "<ID>__<FIELD>"
        let mut parts = rest.splitn(2, "__");
        let id = match parts.next() {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        let field = match parts.next() {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };

        let patch = patches.entry(id).or_default();

        match field {
            "BASE_URL" => patch.base_url = Some(v),
            "DISPLAY_NAME" => patch.display_name = Some(v),
            "ENVIRONMENT" => patch.environment = Some(v),
            "INSECURE_HTTP" => patch.insecure_http = Some(parse_bool(&k, &v)?),
            "FORCED_PROFILE" => {
                // allow empty to mean None
                let vv = v.trim().to_string();
                patch.forced_profile = if vv.is_empty() { None } else { Some(vv) };
            }
            "MACAROON_PATH" => patch.macaroon_path = Some(PathBuf::from(v)),
            "DEFAULT_TIMEOUT" => {
                // seconds, integer (same convention as load_duration)
                let secs: u64 = v.parse().map_err(|e| {
                    Error::Config(format!(
                        "invalid duration seconds for {}: {} ({e})",
                        k, v
                    ))
                })?;
                patch.default_timeout = Some(Duration::from_secs(secs));
            }
            _ => {
                // ignore unknown fields for forward-compat
            }
        }
    }

    // Apply patches
    for (id, patch) in patches {
        // Merge into existing entry if present, else create a new one.
        let mut node = cfg.nodes.remove(&id).unwrap_or_else(|| NodeCfg {
            base_url: patch.base_url.clone().unwrap_or_default(),
            display_name: None,
            environment: "dev".to_string(),
            insecure_http: true,
            forced_profile: None,
            macaroon_path: None,
            default_timeout: Some(Duration::from_secs(2)),
        });

        if let Some(u) = patch.base_url {
            node.base_url = u;
        }
        if node.base_url.trim().is_empty() {
            return Err(Error::Config(format!(
                "node '{}' missing BASE_URL (set SVC_ADMIN_NODES__{}__BASE_URL)",
                id, id
            )));
        }

        if let Some(n) = patch.display_name {
            node.display_name = Some(n);
        }
        if let Some(e) = patch.environment {
            node.environment = e;
        }
        if let Some(b) = patch.insecure_http {
            node.insecure_http = b;
        }
        if let Some(p) = patch.forced_profile {
            node.forced_profile = Some(p);
        }
        if let Some(m) = patch.macaroon_path {
            node.macaroon_path = Some(m);
        }
        if let Some(t) = patch.default_timeout {
            node.default_timeout = Some(t);
        }

        cfg.nodes.insert(id, node);
    }

    // Legacy dev convenience: if set, upsert "example-node".
    if let Ok(url) = env::var("SVC_ADMIN_EXAMPLE_NODE_URL") {
        let env_tag = env::var("SVC_ADMIN_EXAMPLE_NODE_ENV")
            .ok()
            .unwrap_or_else(|| "dev".into());
        cfg.nodes.insert(
            "example-node".to_string(),
            NodeCfg {
                base_url: url,
                display_name: Some("Example Node".to_string()),
                environment: env_tag,
                insecure_http: true,
                forced_profile: Some("macronode".to_string()),
                macaroon_path: None,
                default_timeout: Some(Duration::from_secs(2)),
            },
        );
    }

    Ok(())
}

fn parse_bool(key: &str, raw: &str) -> Result<bool> {
    let v = raw.to_ascii_lowercase();
    match v.as_str() {
        "1" | "true" | "yes" | "y" | "on" => Ok(true),
        "0" | "false" | "no" | "n" | "off" => Ok(false),
        _ => Err(Error::Config(format!("invalid boolean for {}: {}", key, raw))),
    }
}

// --- helpers ---------------------------------------------------------------

fn load_addr(key: &str, default: &str) -> Result<String> {
    match env::var(key) {
        Ok(raw) => raw
            .parse::<SocketAddr>()
            .map_err(|e| Error::Config(format!("invalid socket addr for {}: {} ({e})", key, raw)))
            .map(|_| raw),
        Err(env::VarError::NotPresent) => Ok(default.to_string()),
        Err(e) => Err(Error::Config(format!("failed to read {}: {e}", key))),
    }
}

fn load_bool(key: &str, default: bool) -> Result<bool> {
    match env::var(key) {
        Ok(raw) => parse_bool(key, &raw),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(e) => Err(Error::Config(format!("failed to read {}: {e}", key))),
    }
}

fn load_usize(key: &str, default: usize) -> Result<usize> {
    match env::var(key) {
        Ok(raw) => raw
            .parse::<usize>()
            .map_err(|e| Error::Config(format!("invalid usize for {}: {} ({e})", key, raw))),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(e) => Err(Error::Config(format!("failed to read {}: {e}", key))),
    }
}

fn load_duration(key: &str, default: Duration) -> Result<Duration> {
    match env::var(key) {
        Ok(raw) => {
            // For now we interpret durations as integer seconds to
            // avoid extra deps. We can upgrade to full "5s"/"1m" parsing later.
            let secs: u64 = raw.parse().map_err(|e| {
                Error::Config(format!(
                    "invalid duration seconds for {}: {} ({e})",
                    key, raw
                ))
            })?;
            Ok(Duration::from_secs(secs))
        }
        Err(env::VarError::NotPresent) => Ok(default),
        Err(e) => Err(Error::Config(format!("failed to read {}: {e}", key))),
    }
}

fn load_opt_path(key: &str, default: Option<PathBuf>) -> Option<PathBuf> {
    match env::var_os(key) {
        Some(v) => Some(PathBuf::from(v)),
        None => default,
    }
}

fn load_auth_mode(key: &str, default: &str) -> Result<String> {
    let raw = match env::var(key) {
        Ok(v) => v,
        Err(env::VarError::NotPresent) => return Ok(default.to_string()),
        Err(e) => return Err(Error::Config(format!("failed to read {}: {e}", key))),
    };

    let normalized = raw.to_ascii_lowercase();
    match normalized.as_str() {
        "none" | "ingress" | "passport" | "local" => Ok(normalized),
        _ => Err(Error::Config(format!(
            "invalid auth mode for {}: {}",
            key, raw
        ))),
    }
}
