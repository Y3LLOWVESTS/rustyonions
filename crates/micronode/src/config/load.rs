//! RO:WHAT — Load config from file and env overlays (foundation).
//! RO:WHY  — Keep `main.rs` tiny; centralize TOML + env behavior and
//!           provide a single `load_config()` entrypoint.
//! RO:INTERACTS —
//!   - Reads TOML from `MICRONODE_CONFIG` or well-known paths.
//!   - Applies env overrides for bind/dev-routes (storage is TOML-only
//!     for now; env wiring can be added later).
//!     RO:INVARIANTS —
//!   - Always returns a validated `Config` or a typed `Error::Config`.
//!   - Defaults are safe (amnesia-first, local bind).
//!   - File overlays are applied in a deterministic order.

use std::{env, fs, path::PathBuf};

use crate::errors::{Error, Result};

use super::schema::Config;
use super::validate::validate;

/// Entry point used by `main.rs` to construct the runtime config.
///
/// Order of precedence:
///
/// 1. Start from `Config::default()` (safe defaults).
/// 2. If `MICRONODE_CONFIG` is set, load that TOML and merge.
/// 3. Else, try `./configs/micronode.toml`.
/// 4. Else, try `./crates/micronode/configs/micronode.toml`.
/// 5. Apply env overrides (bind address, dev routes).
/// 6. Run `validate` to enforce invariants.
///
/// Any failure in file IO, TOML parse, or validation is surfaced as
/// `Error::Config` and causes startup to fail-fast.
pub fn load_config() -> Result<Config> {
    let mut cfg = Config::default();

    if let Some(path) = discover_config_path() {
        overlay_file(&mut cfg, path)?;
    }

    overlay_env(&mut cfg)?;
    validate(&cfg)?;
    Ok(cfg)
}

/// Try to locate a config file on disk.
///
/// This intentionally accepts multiple fallbacks to allow:
/// - `MICRONODE_CONFIG` when run under systemd/k8s.
/// - `./configs/micronode.toml` for workspace-local dev.
/// - `./crates/micronode/configs/micronode.toml` for crate-local dev.
fn discover_config_path() -> Option<PathBuf> {
    if let Ok(path) = env::var("MICRONODE_CONFIG") {
        return Some(PathBuf::from(path));
    }

    let workspace_local = PathBuf::from("configs/micronode.toml");
    if workspace_local.exists() {
        return Some(workspace_local);
    }

    let crate_local = PathBuf::from("crates/micronode/configs/micronode.toml");
    if crate_local.exists() {
        return Some(crate_local);
    }

    None
}

/// Overlay TOML config from the given path onto the existing `cfg`.
fn overlay_file(cfg: &mut Config, path: PathBuf) -> Result<()> {
    let s =
        fs::read_to_string(&path).map_err(|e| Error::Config(format!("read {:?}: {e}", path)))?;
    let from: Config =
        toml::from_str(&s).map_err(|e| Error::Config(format!("parse {:?}: {e}", path)))?;

    // Take the current cfg by value, merge, then write back.
    let base = std::mem::take(cfg);
    *cfg = merge(base, from);

    Ok(())
}

/// Apply env-var overrides on top of the parsed config.
///
/// Current surface:
/// - `BIND_ADDR` — override `server.bind`
/// - `MICRONODE_DEV_ROUTES=1` — enable dev routes
///
/// Storage is intentionally configured via TOML only in beta; env-based
/// overrides can be added later when the surface stabilizes.
fn overlay_env(cfg: &mut Config) -> Result<()> {
    if let Ok(addr) = env::var("BIND_ADDR") {
        let parsed = addr
            .parse()
            .map_err(|e| Error::Config(format!("BIND_ADDR parse error ({addr}): {e}")))?;
        cfg.server.bind = parsed;
    }

    if let Ok(flag) = env::var("MICRONODE_DEV_ROUTES") {
        let enabled = flag == "1" || flag.eq_ignore_ascii_case("true");
        cfg.server.dev_routes = enabled;
    }

    Ok(())
}

/// Merge two config instances, with `from` overriding `base`.
///
/// For now this is a simple "last writer wins" overlay; if/when we add
/// partial inheritance we can make this more granular.
fn merge(mut base: Config, from: Config) -> Config {
    // Server section: file/env wins completely.
    base.server = from.server;

    // Storage section: file/env wins completely.
    //
    // This keeps the default (amnesia-first) profile unless the TOML
    // explicitly opts into a different engine or path.
    base.storage = from.storage;

    base
}
