//! RO:WHAT — Execution helpers for non-server Micronode CLI commands.
//! RO:WHY  — Keep `main.rs` small while making `check` and `status` real.
//! RO:INVARIANTS —
//!   - `check` runs the real config loader and validator.
//!   - `status` uses the same admin status builder as `/api/v1/status`.
//!   - No command claims wallet/ledger mutation or confirmed ROC.

use std::env;

use crate::{
    cli::{usage, Cli, Command, ServeOpts},
    config::{load::load_config, schema::Config},
    http::admin_api::build_status_response,
    state::AppState,
};

pub const EXIT_OK: i32 = 0;
pub const EXIT_INTERNAL_ERROR: i32 = 1;
pub const EXIT_CONFIG_ERROR: i32 = 2;
pub const EXIT_USAGE_ERROR: i32 = 64;

/// Run a CLI command that does not start the HTTP server.
///
/// Returns:
/// - `None` when the caller should continue into server startup.
/// - `Some(code)` when the caller should exit with `code`.
pub fn run_control_command(cli: &Cli) -> Option<i32> {
    match cli.command() {
        Command::Serve(_) => None,
        Command::Help => {
            print!("{}", usage());
            Some(EXIT_OK)
        }
        Command::UsageError { message } => {
            eprintln!("micronode: {message}");
            eprintln!();
            eprint!("{}", usage());
            Some(EXIT_USAGE_ERROR)
        }
        Command::Check { config_path } => Some(run_check(config_path.as_deref())),
        Command::Status { config_path } => Some(run_status(config_path.as_deref())),
    }
}

/// Apply serve-time overrides before `load_config()`.
///
/// Config file and bind overrides are translated into the existing env overlay
/// path so the runtime continues to use one validated config loader.
pub fn apply_serve_env_overrides(opts: &ServeOpts) {
    if let Some(path) = opts.config_path.as_deref() {
        env::set_var("MICRONODE_CONFIG", path);
    }

    if let Some(bind_addr) = opts.bind_addr.as_deref() {
        env::set_var("MICRONODE_BIND", bind_addr);
    }

    if let Some(dev_routes) = opts.dev_routes_override {
        env::set_var("MICRONODE_DEV_ROUTES", if dev_routes { "1" } else { "0" });
    }
}

fn apply_config_path_override(config_path: Option<&str>) {
    if let Some(path) = config_path {
        env::set_var("MICRONODE_CONFIG", path);
    }
}

fn run_check(config_path: Option<&str>) -> i32 {
    apply_config_path_override(config_path);

    match load_config() {
        Ok(cfg) => {
            println!("micronode config ok");
            println!("bind={}", cfg.server.bind);
            println!("user_node.passive_runtime_enabled={}", cfg.user_node.passive_runtime_enabled);
            println!(
                "user_node.verification_queue_enabled={}",
                cfg.user_node.verification_queue_enabled
            );
            println!(
                "user_node.economic_replay_worker_enabled={}",
                cfg.user_node.economic_replay_worker_enabled
            );
            println!("user_node.resource_mode={}", cfg.user_node.resource_mode.as_str());
            EXIT_OK
        }
        Err(err) => {
            eprintln!("micronode config error: {err}");
            EXIT_CONFIG_ERROR
        }
    }
}

fn run_status(config_path: Option<&str>) -> i32 {
    apply_config_path_override(config_path);

    let cfg = match load_config() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("micronode config error: {err}");
            return EXIT_CONFIG_ERROR;
        }
    };

    match render_local_status_pretty(cfg) {
        Ok(status) => {
            println!("{status}");
            EXIT_OK
        }
        Err(err) => {
            eprintln!("micronode status render error: {err}");
            EXIT_INTERNAL_ERROR
        }
    }
}

/// Render local status using the same builder used by the HTTP admin API.
pub fn render_local_status_json(cfg: Config) -> serde_json::Value {
    let state = AppState::new(cfg);
    serde_json::to_value(build_status_response(&state))
        .expect("StatusResponse serialization must be infallible")
}

fn render_local_status_pretty(cfg: Config) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&render_local_status_json(cfg))
}
