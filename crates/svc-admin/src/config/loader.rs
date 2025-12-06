// crates/svc-admin/src/config/loader.rs
//
// WHAT: Env-based configuration loading and validation for Config.
// WHY: Keeps parsing and invariants separate from the type definitions.

use crate::error::{Error, Result};
use std::{
    env,
    net::SocketAddr,
    path::PathBuf,
    time::Duration,
};

use super::Config;

impl Config {
    /// Load config from environment only (for now).
    ///
    /// File- and CLI-based config will be layered on in a later step.
    pub fn load() -> Result<Self> {
        if env::var_os("SVC_ADMIN_CONFIG").is_some() {
            // Guardrail until file loading is implemented.
            return Err(Error::Config(
                "SVC_ADMIN_CONFIG is not yet supported (file-based config TODO)"
                    .to_string(),
            ));
        }

        let mut cfg = Config::default();

        // --- Server / listener ---

        cfg.server.bind_addr =
            load_addr("SVC_ADMIN_BIND_ADDR", &cfg.server.bind_addr)?;
        cfg.server.metrics_addr =
            load_addr("SVC_ADMIN_METRICS_ADDR", &cfg.server.metrics_addr)?;

        cfg.server.max_conns =
            load_usize("SVC_ADMIN_MAX_CONNS", cfg.server.max_conns)?;

        cfg.server.read_timeout = load_duration(
            "SVC_ADMIN_READ_TIMEOUT",
            cfg.server.read_timeout,
        )?;
        cfg.server.write_timeout = load_duration(
            "SVC_ADMIN_WRITE_TIMEOUT",
            cfg.server.write_timeout,
        )?;
        cfg.server.idle_timeout = load_duration(
            "SVC_ADMIN_IDLE_TIMEOUT",
            cfg.server.idle_timeout,
        )?;

        cfg.server.tls.enabled =
            load_bool("SVC_ADMIN_TLS_ENABLED", cfg.server.tls.enabled)?;
        cfg.server.tls.cert_path =
            load_opt_path("SVC_ADMIN_TLS_CERT_PATH", cfg.server.tls.cert_path);
        cfg.server.tls.key_path =
            load_opt_path("SVC_ADMIN_TLS_KEY_PATH", cfg.server.tls.key_path);

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

        cfg.ui.read_only =
            load_bool("SVC_ADMIN_UI_READ_ONLY", cfg.ui.read_only)?;
        cfg.ui.dev.enable_app_playground = load_bool(
            "SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND",
            cfg.ui.dev.enable_app_playground,
        )?;

        // --- Actions ---

        cfg.actions.enable_reload = load_bool(
            "SVC_ADMIN_ACTIONS_ENABLE_RELOAD",
            cfg.actions.enable_reload,
        )?;
        cfg.actions.enable_shutdown = load_bool(
            "SVC_ADMIN_ACTIONS_ENABLE_SHUTDOWN",
            cfg.actions.enable_shutdown,
        )?;

        // --- Auth ---

        cfg.auth.mode =
            load_auth_mode("SVC_ADMIN_AUTH_MODE", &cfg.auth.mode)?;

        if let Ok(issuer) = env::var("SVC_ADMIN_AUTH_PASSPORT_ISSUER") {
            cfg.auth.passport_issuer = Some(issuer);
        }
        if let Ok(aud) = env::var("SVC_ADMIN_AUTH_PASSPORT_AUDIENCE") {
            cfg.auth.passport_audience = Some(aud);
        }
        if let Ok(jwks) = env::var("SVC_ADMIN_AUTH_PASSPORT_JWKS_URL") {
            cfg.auth.passport_jwks_url = Some(jwks);
        }

        // --- Node seed overrides (dev convenience) ---

        if let Some(example) = cfg.nodes.get_mut("example-node") {
            if let Ok(url) = env::var("SVC_ADMIN_EXAMPLE_NODE_URL") {
                example.base_url = url;
            }
            if let Ok(env_tag) = env::var("SVC_ADMIN_EXAMPLE_NODE_ENV") {
                example.environment = env_tag;
            }
        }

        cfg.validate()?;
        Ok(cfg)
    }

    /// Basic invariants for Config v2.
    fn validate(&self) -> Result<()> {
        if self.server.max_conns == 0 {
            return Err(Error::Config(
                "server.max_conns must be > 0".to_string(),
            ));
        }

        if self.polling.metrics_interval == Duration::from_secs(0) {
            return Err(Error::Config(
                "polling.metrics_interval must be > 0s".to_string(),
            ));
        }

        if self.polling.metrics_window < self.polling.metrics_interval {
            return Err(Error::Config(
                "polling.metrics_window must be >= polling.metrics_interval"
                    .to_string(),
            ));
        }

        if self.server.tls.enabled
            && (self.server.tls.cert_path.is_none()
                || self.server.tls.key_path.is_none())
        {
            return Err(Error::Config(
                "TLS is enabled but cert_path/key_path are not both set"
                    .to_string(),
            ));
        }

        Ok(())
    }
}

// --- helpers ---------------------------------------------------------------

fn load_addr(key: &str, default: &str) -> Result<String> {
    match env::var(key) {
        Ok(raw) => {
            raw.parse::<SocketAddr>()
                .map_err(|e| {
                    Error::Config(format!(
                        "invalid socket addr for {}: {} ({e})",
                        key, raw
                    ))
                })
                .map(|_| raw)
        }
        Err(env::VarError::NotPresent) => Ok(default.to_string()),
        Err(e) => Err(Error::Config(format!(
            "failed to read {}: {e}",
            key
        ))),
    }
}

fn load_bool(key: &str, default: bool) -> Result<bool> {
    match env::var(key) {
        Ok(raw) => {
            let v = raw.to_ascii_lowercase();
            match v.as_str() {
                "1" | "true" | "yes" | "y" | "on" => Ok(true),
                "0" | "false" | "no" | "n" | "off" => Ok(false),
                _ => Err(Error::Config(format!(
                    "invalid boolean for {}: {}",
                    key, raw
                ))),
            }
        }
        Err(env::VarError::NotPresent) => Ok(default),
        Err(e) => Err(Error::Config(format!(
            "failed to read {}: {e}",
            key
        ))),
    }
}

fn load_usize(key: &str, default: usize) -> Result<usize> {
    match env::var(key) {
        Ok(raw) => raw.parse::<usize>().map_err(|e| {
            Error::Config(format!("invalid usize for {}: {} ({e})", key, raw))
        }),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(e) => Err(Error::Config(format!(
            "failed to read {}: {e}",
            key
        ))),
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
        Err(e) => Err(Error::Config(format!(
            "failed to read {}: {e}",
            key
        ))),
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
        Err(e) => {
            return Err(Error::Config(format!(
                "failed to read {}: {e}",
                key
            )))
        }
    };

    let normalized = raw.to_ascii_lowercase();
    match normalized.as_str() {
        "none" | "ingress" | "passport" => Ok(normalized),
        _ => Err(Error::Config(format!(
            "invalid auth mode for {}: {}",
            key, raw
        ))),
    }
}
