use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::net::SocketAddr;

/// Top-level svc-admin config.
///
/// For this first slice we support:
/// - Env + defaults only (no TOML file yet).
/// - Minimal fields: server, auth, ui, nodes (config-driven registry).
///
/// Precedence (current dev-preview behavior):
///   env → defaults
///
/// Future (per docs/CONFIG.MD):
///   CLI → env → file → defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerCfg,
    pub auth: AuthCfg,
    pub ui: UiCfg,
    pub nodes: NodesCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCfg {
    /// UI/API bind address (Axum).
    /// Env: SVC_ADMIN_BIND_ADDR
    pub bind_addr: String,

    /// Metrics/health bind address.
    /// Env: SVC_ADMIN_METRICS_ADDR
    pub metrics_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCfg {
    /// "none" | "ingress" | "passport"
    /// Env: SVC_ADMIN_AUTH_MODE
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiCfg {
    /// Initial theme for new sessions ("light" | "dark" | "system" etc.).
    /// Env: SVC_ADMIN_UI_DEFAULT_THEME
    pub default_theme: String,

    /// Default language/locale (e.g. "en-US").
    /// Env: SVC_ADMIN_UI_DEFAULT_LANGUAGE
    pub default_language: String,

    /// Whether the UI is read-only (no “dangerous” actions).
    /// Env: SVC_ADMIN_UI_READ_ONLY
    pub read_only: bool,
}

/// Configuration for a single RON-CORE node that svc-admin can talk to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCfg {
    /// Base admin URL for the node (e.g. "http://127.0.0.1:9000").
    pub base_url: String,
    /// Human-friendly display name; falls back to the logical ID if None.
    pub display_name: Option<String>,
    /// Environment tag: "dev", "staging", "prod", etc.
    pub environment: String,
    /// Whether plain HTTP is allowed when talking to this node.
    pub insecure_http: bool,
}

/// Map of node_id → NodeCfg.
///
/// Later this will be hydrated from TOML (`[nodes.<id>]` blocks) and/or
/// env-var overlays. For now we seed a single example node.
pub type NodesCfg = BTreeMap<String, NodeCfg>;

impl Config {
    /// Load configuration from environment variables + built-in defaults.
    ///
    /// This is intentionally **env-only** for the first dev slice. We keep
    /// the shape compatible with future TOML/CLI loading so that we can add
    /// those later without breaking callers.
    ///
    /// Env keys (current dev behavior):
    ///
    /// - SVC_ADMIN_BIND_ADDR            (default: 127.0.0.1:5300)
    /// - SVC_ADMIN_METRICS_ADDR         (default: 127.0.0.1:5310)
    /// - SVC_ADMIN_AUTH_MODE            (default: "none")
    /// - SVC_ADMIN_UI_DEFAULT_THEME     (default: "light")
    /// - SVC_ADMIN_UI_DEFAULT_LANGUAGE  (default: "en-US")
    /// - SVC_ADMIN_UI_READ_ONLY         (default: "true")
    ///
    /// Node seed (dev-only convenience):
    /// - SVC_ADMIN_EXAMPLE_NODE_URL     (default: http://127.0.0.1:9000)
    /// - SVC_ADMIN_EXAMPLE_NODE_ENV     (default: "dev")
    pub fn load() -> Result<Self> {
        // Guardrail: we *know* we will support config files later via
        // SVC_ADMIN_CONFIG / CLI flags. For now we fail fast if someone tries
        // to use SVC_ADMIN_CONFIG so we don’t silently ignore it.
        if let Ok(path) = env::var("SVC_ADMIN_CONFIG") {
            return Err(Error::Config(format!(
                "SVC_ADMIN_CONFIG={path} is set, but config file loading \
                 is not implemented yet. Unset SVC_ADMIN_CONFIG to use \
                 env-only defaults."
            )));
        }

        let bind_addr = load_addr("SVC_ADMIN_BIND_ADDR", "127.0.0.1:5300")?;
        let metrics_addr = load_addr("SVC_ADMIN_METRICS_ADDR", "127.0.0.1:5310")?;

        let auth_mode = load_auth_mode("SVC_ADMIN_AUTH_MODE", "none")?;

        let default_theme =
            env::var("SVC_ADMIN_UI_DEFAULT_THEME").unwrap_or_else(|_| "light".to_string());
        let default_language =
            env::var("SVC_ADMIN_UI_DEFAULT_LANGUAGE").unwrap_or_else(|_| "en-US".to_string());
        let read_only = load_bool("SVC_ADMIN_UI_READ_ONLY", true)?;

        // Seed a single example node for dev so the UI/API have real data.
        let mut nodes: NodesCfg = NodesCfg::new();
        nodes.insert(
            "example-node".to_string(),
            NodeCfg {
                base_url: env::var("SVC_ADMIN_EXAMPLE_NODE_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string()),
                display_name: Some("Example Node".to_string()),
                environment: env::var("SVC_ADMIN_EXAMPLE_NODE_ENV")
                    .unwrap_or_else(|_| "dev".to_string()),
                insecure_http: true,
            },
        );

        Ok(Config {
            server: ServerCfg {
                bind_addr,
                metrics_addr,
            },
            auth: AuthCfg { mode: auth_mode },
            ui: UiCfg {
                default_theme,
                default_language,
                read_only,
            },
            nodes,
        })
    }
}

/// Load and validate a SocketAddr-ish env var.
///
/// We store it as String in Config, but we *parse* it here to fail fast on
/// obviously broken values (e.g. bad port).
fn load_addr(key: &str, default: &str) -> Result<String> {
    let raw = env::var(key).unwrap_or_else(|_| default.to_string());

    match raw.parse::<SocketAddr>() {
        Ok(_) => Ok(raw),
        Err(e) => Err(Error::Config(format!(
            "invalid {key} `{raw}`: {e}"
        ))),
    }
}

/// Load a boolean with friendly syntax:
/// - true/false
/// - 1/0
/// - yes/no, y/n
fn load_bool(key: &str, default: bool) -> Result<bool> {
    match env::var(key) {
        Ok(val) => {
            let v = val.to_lowercase();
            if matches!(v.as_str(), "1" | "true" | "yes" | "y") {
                Ok(true)
            } else if matches!(v.as_str(), "0" | "false" | "no" | "n") {
                Ok(false)
            } else {
                Err(Error::Config(format!(
                    "invalid boolean for {key}: `{val}` (expected true/false/1/0/yes/no/y/n)"
                )))
            }
        }
        Err(_) => Ok(default),
    }
}

/// Validate auth.mode from env; keeps the string but enforces the allowed set.
fn load_auth_mode(key: &str, default: &str) -> Result<String> {
    let raw = env::var(key).unwrap_or_else(|_| default.to_string());
    match raw.as_str() {
        "none" | "ingress" | "passport" => Ok(raw),
        _ => Err(Error::Config(format!(
            "invalid auth mode `{raw}` from {key}; expected one of: none, ingress, passport"
        ))),
    }
}
