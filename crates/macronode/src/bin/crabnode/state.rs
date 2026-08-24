//! RO:WHAT — Owns CrabNode bootstrap operator state, node ID, and strict bootstrap config inspection.
//! RO:WHY — CN-1 requires idempotent init plus truthful config show/validate before daemon lifecycle.
//! RO:INTERACTS — crabnode.rs; future CN-2 canonical runtime profile and CN-7 identity wiring.
//! RO:INVARIANTS — never overwrite config/identity; unknown config fails closed; no secret creation.
//! RO:CONFIG — CRABNODE_HOME may override the platform-local state root.
//! RO:SECURITY — Unix dirs 0700/files 0600; invalid config contents are never echoed; node ID is public.
//! RO:TEST — crabnode_init_cli.rs and crabnode_config_cli.rs.

use std::{
    env, fs,
    fs::OpenOptions,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{CrabnodeError, Options};

const CONFIG_FILE: &str = "crabnode.toml";
const NODE_ID_FILE: &str = "node-id";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapConfig {
    schema_version: u32,
    profile: String,
    operator: BootstrapOperatorConfig,
    security: BootstrapSecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapOperatorConfig {
    headless: bool,
    admin_ui_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapSecurityConfig {
    admin_loopback_only: bool,
}

const SAFE_DEFAULT_CONFIG: &str = r#"# CrabNode private-beta operator configuration.
# CN-2 will make this file the canonical runtime service/profile configuration.

schema_version = 1
profile = "private_beta"

[operator]
headless = true
admin_ui_required = false

[security]
admin_loopback_only = true
"#;

pub(super) fn init_or_dry_run(opts: &Options) -> Result<(), CrabnodeError> {
    let layout = StateLayout::resolve()?;

    if opts.dry_run {
        println!(
            "crabnode init: would initialize local operator state at {}; \
             no directories or files created",
            layout.root.display()
        );
        return Ok(());
    }

    layout.create_dirs()?;

    let config_created =
        create_file_if_absent(&layout.config_file, SAFE_DEFAULT_CONFIG.as_bytes())?;

    let (node_id, node_id_created) = ensure_node_id(&layout.node_id_file)?;

    println!("CrabNode initialized at {}", layout.root.display());
    println!(
        "config={} ({})",
        layout.config_file.display(),
        if config_created {
            "created"
        } else {
            "preserved"
        }
    );
    println!(
        "node_id={} ({})",
        node_id,
        if node_id_created {
            "created"
        } else {
            "preserved"
        }
    );
    println!("data={}", layout.data_dir.display());
    println!("logs={}", layout.log_dir.display());
    println!("run={}", layout.run_dir.display());

    Ok(())
}

pub(super) fn show_config() -> Result<(), CrabnodeError> {
    let layout = StateLayout::resolve()?;
    let cfg = load_bootstrap_config(&layout.config_file)?;
    validate_bootstrap_config(&cfg)?;

    let rendered = toml::to_string_pretty(&cfg).map_err(|_| {
        CrabnodeError::config("validated CrabNode bootstrap config could not be rendered")
    })?;

    println!("crabnode config show: {}", layout.config_file.display());
    print!("{rendered}");

    Ok(())
}

pub(super) fn validate_config_file() -> Result<(), CrabnodeError> {
    let layout = StateLayout::resolve()?;
    let cfg = load_bootstrap_config(&layout.config_file)?;
    validate_bootstrap_config(&cfg)?;

    println!(
        "crabnode config validate: OK ({})",
        layout.config_file.display()
    );

    Ok(())
}

fn load_bootstrap_config(path: &Path) -> Result<BootstrapConfig, CrabnodeError> {
    let raw = fs::read_to_string(path).map_err(|err| {
        if err.kind() == ErrorKind::NotFound {
            CrabnodeError::config(format!(
                "CrabNode config is missing at {}; run `crabnode init` first",
                path.display()
            ))
        } else {
            CrabnodeError::io(format!(
                "cannot read CrabNode config {}: {err}",
                path.display()
            ))
        }
    })?;

    toml::from_str::<BootstrapConfig>(&raw).map_err(|_| {
        CrabnodeError::config(format!(
            "CrabNode config {} is invalid or contains unsupported fields",
            path.display()
        ))
    })
}

fn validate_bootstrap_config(cfg: &BootstrapConfig) -> Result<(), CrabnodeError> {
    if cfg.schema_version != 1 {
        return Err(CrabnodeError::config(
            "CrabNode bootstrap config schema_version must be 1",
        ));
    }

    if cfg.profile != "private_beta" {
        return Err(CrabnodeError::config(
            "CrabNode bootstrap profile must remain private_beta until CN-2 establishes canonical profiles",
        ));
    }

    if !cfg.operator.headless {
        return Err(CrabnodeError::config(
            "CrabNode bootstrap operator.headless must remain true",
        ));
    }

    if cfg.operator.admin_ui_required {
        return Err(CrabnodeError::config(
            "CrabNode bootstrap operator.admin_ui_required must remain false",
        ));
    }

    if !cfg.security.admin_loopback_only {
        return Err(CrabnodeError::config(
            "CrabNode bootstrap security.admin_loopback_only must remain true",
        ));
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub(super) struct RuntimeState {
    pub(super) node_id: String,
    pub(super) config_file: PathBuf,
    pub(super) node_id_file: PathBuf,
    pub(super) data_dir: PathBuf,
    pub(super) log_dir: PathBuf,
    pub(super) run_dir: PathBuf,
    pub(super) log_file: PathBuf,
    pub(super) process_state_file: PathBuf,
}

pub(super) fn load_runtime_state() -> Result<RuntimeState, CrabnodeError> {
    let layout = StateLayout::resolve()?;

    require_runtime_directory(&layout.log_dir)?;
    require_runtime_directory(&layout.run_dir)?;
    refuse_runtime_symlink(&layout.config_file)?;
    refuse_runtime_symlink(&layout.node_id_file)?;

    let cfg = load_bootstrap_config(&layout.config_file)?;
    validate_bootstrap_config(&cfg)?;

    let node_id_raw = fs::read_to_string(&layout.node_id_file).map_err(|err| {
        if err.kind() == ErrorKind::NotFound {
            CrabnodeError::config("CrabNode node identity is missing; run `crabnode init` first")
        } else {
            CrabnodeError::io(format!(
                "cannot read CrabNode node identity {}: {err}",
                layout.node_id_file.display()
            ))
        }
    })?;

    let node_id = node_id_raw.trim();
    validate_node_id(node_id)?;

    Ok(RuntimeState {
        node_id: node_id.to_string(),
        config_file: layout.config_file.clone(),
        node_id_file: layout.node_id_file.clone(),
        data_dir: layout.data_dir.clone(),
        log_dir: layout.log_dir.clone(),
        run_dir: layout.run_dir.clone(),
        log_file: layout.log_dir.join("crabnode.log"),
        process_state_file: layout.run_dir.join("crabnode-process.json"),
    })
}

fn require_runtime_directory(path: &Path) -> Result<(), CrabnodeError> {
    let metadata = fs::symlink_metadata(path).map_err(|err| {
        if err.kind() == ErrorKind::NotFound {
            CrabnodeError::config(format!(
                "CrabNode state directory {} is missing; run `crabnode init` first",
                path.display()
            ))
        } else {
            CrabnodeError::io(format!(
                "cannot inspect CrabNode state directory {}: {err}",
                path.display()
            ))
        }
    })?;

    if metadata.file_type().is_symlink() {
        return Err(CrabnodeError::config(format!(
            "CrabNode state directory {} must not be a symlink",
            path.display()
        )));
    }

    if !metadata.is_dir() {
        return Err(CrabnodeError::config(format!(
            "CrabNode state path {} must be a directory",
            path.display()
        )));
    }

    Ok(())
}

fn refuse_runtime_symlink(path: &Path) -> Result<(), CrabnodeError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(CrabnodeError::config(format!(
            "CrabNode state file {} must not be a symlink",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(CrabnodeError::io(format!(
            "cannot inspect CrabNode state file {}: {err}",
            path.display()
        ))),
    }
}

#[derive(Debug)]
struct StateLayout {
    root: PathBuf,
    config_dir: PathBuf,
    data_dir: PathBuf,
    log_dir: PathBuf,
    run_dir: PathBuf,
    config_file: PathBuf,
    node_id_file: PathBuf,
}

impl StateLayout {
    fn resolve() -> Result<Self, CrabnodeError> {
        let root = crabnode_home()?;
        let config_dir = root.join("config");
        let data_dir = root.join("data");
        let log_dir = root.join("log");
        let run_dir = root.join("run");

        Ok(Self {
            config_file: config_dir.join(CONFIG_FILE),
            node_id_file: data_dir.join(NODE_ID_FILE),
            root,
            config_dir,
            data_dir,
            log_dir,
            run_dir,
        })
    }

    fn create_dirs(&self) -> Result<(), CrabnodeError> {
        for dir in [
            &self.root,
            &self.config_dir,
            &self.data_dir,
            &self.log_dir,
            &self.run_dir,
        ] {
            fs::create_dir_all(dir).map_err(|err| {
                CrabnodeError::io(format!(
                    "cannot create CrabNode state directory {}: {err}",
                    dir.display()
                ))
            })?;

            secure_directory(dir)?;
        }

        Ok(())
    }
}

fn crabnode_home() -> Result<PathBuf, CrabnodeError> {
    if let Some(root) = nonempty_env("CRABNODE_HOME") {
        return Ok(PathBuf::from(root));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(base) = nonempty_env("LOCALAPPDATA").or_else(|| nonempty_env("APPDATA")) {
            return Ok(PathBuf::from(base).join("CrabNode"));
        }

        return Err(CrabnodeError::config(
            "cannot resolve CrabNode state root: set CRABNODE_HOME or LOCALAPPDATA",
        ));
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = nonempty_env("HOME") {
            return Ok(PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("CrabNode"));
        }

        return Err(CrabnodeError::config(
            "cannot resolve CrabNode state root: set CRABNODE_HOME or HOME",
        ));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(state_home) = nonempty_env("XDG_STATE_HOME") {
            return Ok(PathBuf::from(state_home).join("crabnode"));
        }

        if let Some(home) = nonempty_env("HOME") {
            return Ok(PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("crabnode"));
        }

        return Err(CrabnodeError::config(
            "cannot resolve CrabNode state root: set CRABNODE_HOME, XDG_STATE_HOME, or HOME",
        ));
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    {
        Err(CrabnodeError::config(
            "cannot resolve CrabNode state root on this platform; set CRABNODE_HOME",
        ))
    }
}

fn nonempty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn ensure_node_id(path: &Path) -> Result<(String, bool), CrabnodeError> {
    match fs::read_to_string(path) {
        Ok(existing) => {
            let node_id = existing.trim();

            validate_node_id(node_id)?;

            return Ok((node_id.to_string(), false));
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => {
            return Err(CrabnodeError::io(format!(
                "cannot read existing CrabNode node identity {}: {err}",
                path.display()
            )));
        }
    }

    let node_id = generate_node_id();
    let payload = format!("{node_id}\n");

    match create_file_if_absent(path, payload.as_bytes()) {
        Ok(true) => Ok((node_id, true)),
        Ok(false) => {
            let existing = fs::read_to_string(path).map_err(|err| {
                CrabnodeError::io(format!(
                    "cannot read concurrently created CrabNode node identity {}: {err}",
                    path.display()
                ))
            })?;

            let existing = existing.trim();
            validate_node_id(existing)?;

            Ok((existing.to_string(), false))
        }
        Err(err) => Err(err),
    }
}

fn generate_node_id() -> String {
    let first = Uuid::new_v4().simple().to_string();
    let second = Uuid::new_v4().simple().to_string();

    format!("{first}{second}")
}

fn validate_node_id(node_id: &str) -> Result<(), CrabnodeError> {
    let valid = node_id.len() == 64
        && node_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));

    if valid {
        Ok(())
    } else {
        Err(CrabnodeError::config(
            "existing CrabNode node identity is malformed; refusing to replace it",
        ))
    }
}

fn create_file_if_absent(path: &Path, contents: &[u8]) -> Result<bool, CrabnodeError> {
    let mut file = match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
            secure_file(path)?;
            return Ok(false);
        }
        Err(err) => {
            return Err(CrabnodeError::io(format!(
                "cannot create CrabNode state file {}: {err}",
                path.display()
            )));
        }
    };

    file.write_all(contents).map_err(|err| {
        CrabnodeError::io(format!(
            "cannot write CrabNode state file {}: {err}",
            path.display()
        ))
    })?;

    file.sync_all().map_err(|err| {
        CrabnodeError::io(format!(
            "cannot sync CrabNode state file {}: {err}",
            path.display()
        ))
    })?;

    drop(file);
    secure_file(path)?;

    Ok(true)
}

#[cfg(unix)]
fn secure_directory(path: &Path) -> Result<(), CrabnodeError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|err| {
        CrabnodeError::io(format!(
            "cannot secure CrabNode state directory {}: {err}",
            path.display()
        ))
    })
}

#[cfg(not(unix))]
fn secure_directory(_path: &Path) -> Result<(), CrabnodeError> {
    Ok(())
}

#[cfg(unix)]
fn secure_file(path: &Path) -> Result<(), CrabnodeError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| {
        CrabnodeError::io(format!(
            "cannot secure CrabNode state file {}: {err}",
            path.display()
        ))
    })
}

#[cfg(not(unix))]
fn secure_file(_path: &Path) -> Result<(), CrabnodeError> {
    Ok(())
}
