#![forbid(unsafe_code)]

use serde::Deserialize;
use std::{
    env,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{error, info, warn};

use crate::{bus::Bus, metrics::HealthState, KernelEvent};

/// Optional nested transport section.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct TransportConfig {
    pub max_conns: Option<u64>,
    pub idle_timeout_ms: Option<u64>,
    pub read_timeout_ms: Option<u64>,
    pub write_timeout_ms: Option<u64>,
}

/// Workspace-wide config with **typed fields** for legacy callsites,
/// plus `raw` for anything else (keep API flexible).
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Entire parsed TOML (for ad-hoc lookups).
    pub raw: toml::Table,

    /// Common top-level fields (back-compat with existing bins).
    pub admin_addr: String,       // e.g., "127.0.0.1:9096"
    pub overlay_addr: String,     // e.g., "127.0.0.1:1777"
    pub dev_inbox_addr: String,   // e.g., "127.0.0.1:2888"
    pub socks5_addr: String,      // e.g., "127.0.0.1:9050"
    pub tor_ctrl_addr: String,    // e.g., "127.0.0.1:9051"
    pub data_dir: String,         // e.g., ".data"
    pub chunk_size: u64,          // e.g., 65536
    pub connect_timeout_ms: u64,  // e.g., 5000

    /// Optional nested section.
    pub transport: TransportConfig,
}

impl Config {
    /// Build from a TOML table, filling typed fields with defaults when absent.
    fn from_table(t: toml::Table) -> Self {
        // Helper getters that DO NOT mutate the table (no borrow conflicts)
        fn get_string(tbl: &toml::Table, key: &str) -> Option<String> {
            tbl.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
        }
        fn get_u64(tbl: &toml::Table, key: &str) -> Option<u64> {
            tbl.get(key).and_then(|v| v.as_integer()).map(|n| n as u64)
        }

        // Top-level typed fields (fallback to sensible defaults)
        let admin_addr         = get_string(&t, "admin_addr").unwrap_or_else(|| "127.0.0.1:9096".to_string());
        let overlay_addr       = get_string(&t, "overlay_addr").unwrap_or_else(|| "127.0.0.1:1777".to_string());
        let dev_inbox_addr     = get_string(&t, "dev_inbox_addr").unwrap_or_else(|| "127.0.0.1:2888".to_string());
        let socks5_addr        = get_string(&t, "socks5_addr").unwrap_or_else(|| "127.0.0.1:9050".to_string());
        let tor_ctrl_addr      = get_string(&t, "tor_ctrl_addr").unwrap_or_else(|| "127.0.0.1:9051".to_string());
        let data_dir           = get_string(&t, "data_dir").unwrap_or_else(|| ".data".to_string());
        let chunk_size         = get_u64(&t, "chunk_size").unwrap_or(65536);
        let connect_timeout_ms = get_u64(&t, "connect_timeout_ms").unwrap_or(5000);

        // Nested [transport] section (optional)
        let transport = t
            .get("transport")
            .and_then(|v| v.clone().try_into().ok())
            .unwrap_or_default();

        // Keep the full table for ad-hoc consumers.
        Self {
            raw: t,
            admin_addr,
            overlay_addr,
            dev_inbox_addr,
            socks5_addr,
            tor_ctrl_addr,
            data_dir,
            chunk_size,
            connect_timeout_ms,
            transport,
        }
    }
}

static VERSION: AtomicU64 = AtomicU64::new(0);

/// Synchronously load and parse a TOML config file.
pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let txt = fs::read_to_string(path)?;
    let table: toml::Table = toml::from_str(&txt)?;
    Ok(Config::from_table(table))
}

/// Spawn a background watcher for `path`. On successful (re)loads:
///   - flip health "config" true
///   - publish KernelEvent::ConfigUpdated { version }
/// On parse/read errors: flip health false (does not exit).
///
/// This task runs until the process exits; fire-and-forget is fine.
pub fn spawn_config_watcher<P: Into<PathBuf>>(
    path: P,
    bus: Bus,
    health: Arc<HealthState>,
) -> tokio::task::JoinHandle<()> {
    let path = path.into();

    tokio::spawn(async move {
        // Run the blocking watcher loop on a dedicated thread pool thread.
        let _ = tokio::task::spawn_blocking(move || watch_loop(path, bus, health)).await;
    })
}

fn watch_loop(path: PathBuf, bus: Bus, health: Arc<HealthState>) {
    // Initial load (non-fatal if missing)
    match load_from_file(&path) {
        Ok(_) => {
            health.set("config", true);
            let v = VERSION.fetch_add(1, Ordering::SeqCst) + 1;
            let _ = bus.publish(KernelEvent::ConfigUpdated { version: v });
            info!(version = v, file = ?path, "config loaded");
        }
        Err(e) => {
            health.set("config", false);
            warn!(error = %e, file = ?path, "config initial load failed");
        }
    }

    // ---- Normalize watch dir so passing just "config.toml" doesn't produce "" ----
    let watch_dir: PathBuf = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    // Channel for notify callback -> loop
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            let _ = tx.send(res);
        },
        NotifyConfig::default()
            .with_poll_interval(Duration::from_millis(750)) // portable
            .with_compare_contents(true),
    )
    .expect("create config watcher");

    // Watch the file’s parent to catch atomic renames/writes.
    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .expect("watch config directory");

    // Debounce simple: small sleep after an event burst.
    let debounce = Duration::from_millis(200);
    loop {
        match rx.recv() {
            Ok(Ok(ev)) => {
                // Only act on relevant events
                match ev.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        std::thread::sleep(debounce);
                        match load_from_file(&path) {
                            Ok(_) => {
                                health.set("config", true);
                                let v = VERSION.fetch_add(1, Ordering::SeqCst) + 1;
                                let _ = bus.publish(KernelEvent::ConfigUpdated { version: v });
                                info!(version = v, file = ?path, "config reloaded");
                            }
                            Err(e) => {
                                health.set("config", false);
                                warn!(error = %e, file = ?path, "config reload failed");
                            }
                        }
                    }
                    _ => { /* ignore other event kinds */ }
                }
            }
            Ok(Err(e)) => {
                warn!(error = %e, "config watcher error event");
            }
            Err(e) => {
                error!(error = %e, "config watcher channel closed; exiting watcher loop");
                break;
            }
        }
    }
}
