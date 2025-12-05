// crates/svc-admin/src/config/mod.rs
//
// WHAT: Top-level configuration module for svc-admin (Config v2).
// WHY: Keeps the public shape stable while splitting into smaller,
//      focused submodules (server, ui, auth, nodes, etc.).
// API: External code continues to use `crate::config::Config` and
//      related types; this file re-exports them.

use serde::{Deserialize, Serialize};

pub mod actions;
pub mod auth;
pub mod loader;
pub mod log;
pub mod nodes;
pub mod polling;
pub mod server;
pub mod ui;

pub use actions::ActionsCfg;
pub use auth::AuthCfg;
pub use log::LogCfg;
pub use nodes::{NodeCfg, NodesCfg};
pub use polling::PollingCfg;
pub use server::{ServerCfg, TlsCfg};
pub use ui::{UiCfg, UiDevCfg};

/// Top-level service config.
///
/// Shape is intentionally simple and env-driven for now. File- and
/// CLI-based config are future work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server / listener config.
    pub server: ServerCfg,

    /// Auth mode and passport/JWT settings.
    pub auth: AuthCfg,

    /// UI defaults and dev flags.
    pub ui: UiCfg,

    /// Config-driven node registry.
    pub nodes: NodesCfg,

    /// Metrics / sampling config.
    pub polling: PollingCfg,

    /// Logging style.
    pub log: LogCfg,

    /// Admin actions feature flags.
    pub actions: ActionsCfg,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerCfg::default(),
            auth: AuthCfg::default(),
            ui: UiCfg::default(),
            nodes: nodes::default_nodes(),
            polling: PollingCfg::default(),
            log: LogCfg::default(),
            actions: ActionsCfg::default(),
        }
    }
}
