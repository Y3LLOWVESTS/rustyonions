//! RO:WHAT — Config watchers: filesystem (TOML) + env poller.
//! RO:WHY  — Hot-reload posture without blocking; keep amnesia gauge in sync.
//! RO:INVARIANTS — Non-blocking; no locks across .await; errors are logged and ignored; only emit on real change.

use super::{Config, ConfigCell};
use crate::{Bus, Metrics, events::KernelEvent};
use anyhow::Context;
use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{env, path::PathBuf, sync::Arc};
use tokio::{fs, sync::mpsc, task};

/// Spawn a file watcher on a TOML file. On write/create, parse and apply if changed.
pub fn spawn_file_watcher(
    path: PathBuf,
    cell: Arc<ConfigCell>,
    bus: Bus,
    metrics: Metrics,
    autobump: bool,
) {
    let (tx, mut rx) = mpsc::unbounded_channel::<()>();

    // Blocking thread for notify (keeps OS handle alive).
    let path_clone = path.clone();
    let _handle = task::spawn_blocking(move || {
        let tx_inner = tx.clone();
        let mut watcher: RecommendedWatcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => {
                            let _ = tx_inner.send(());
                        }
                        _ => {}
                    }
                }
            },
            NotifyConfig::default(),
        )
        .expect("create watcher");

        watcher
            .watch(&path_clone, RecursiveMode::NonRecursive)
            .expect("watch path");

        loop {
            std::thread::park();
        }
    });

    // Async side: on signal, reload and apply.
    tokio::spawn(async move {
        while let Some(()) = rx.recv().await {
            if let Err(e) = reload_from_file(&path, &cell, &bus, &metrics, autobump).await {
                eprintln!("[kernel.config] failed to reload {:?}: {e:#}", path);
            }
        }
    });
}

/// Reload the config from TOML and apply it (only if changed). May autobump version.
async fn reload_from_file(
    path: &PathBuf,
    cell: &Arc<ConfigCell>,
    bus: &Bus,
    metrics: &Metrics,
    autobump: bool,
) -> anyhow::Result<()> {
    let bytes = fs::read(path).await.with_context(|| format!("read {:?}", path))?;
    let text = String::from_utf8_lossy(&bytes);
    let mut file_cfg: Config =
        toml::from_str(&text).with_context(|| format!("parse TOML {:?}", path))?;

    let old = cell.get();

    // CONTENT-based change: in our minimal config, content == {amnesia}
    let content_changed = old.amnesia != file_cfg.amnesia;

    if !autobump {
        // Strict mode: apply only when the whole struct differs.
        if *old == file_cfg {
            return Ok(());
        }
        // Apply as-is (file controls version).
        cell.set(file_cfg.clone());
        metrics.set_amnesia(file_cfg.amnesia);
        let _ = bus.publish(KernelEvent::ConfigUpdated { version: file_cfg.version });
        return Ok(());
    }

    // Autobump mode: apply only when content changes; set version = max(file.version, old.version + 1).
    if content_changed {
        if file_cfg.version <= old.version {
            file_cfg.version = old.version.saturating_add(1);
        }
        cell.set(file_cfg.clone());
        metrics.set_amnesia(file_cfg.amnesia);
        let _ = bus.publish(KernelEvent::ConfigUpdated { version: file_cfg.version });
        return Ok(());
    }

    // No content change. Optionally adopt a higher version from file (no event).
    if file_cfg.version > old.version {
        let mut next = (*old).clone();
        next.version = file_cfg.version;
        cell.set(next);
        // No event — content unchanged; version-only bump is local bookkeeping.
    }

    Ok(())
}

/// Spawn an env poller that checks a single key and toggles amnesia on change.
/// Values: on|off|true|false|1|0 (case-insensitive). Emits only on real change; may autobump.
pub fn spawn_env_poller(
    key: &'static str,
    poll_secs: u64,
    cell: Arc<ConfigCell>,
    bus: Bus,
    metrics: Metrics,
    autobump: bool,
) {
    tokio::spawn(async move {
        // Seed from env on boot, if present.
        if let Some(v) = read_bool_env(key) {
            let old = cell.get();
            if old.amnesia != v {
                let mut next = (*old).clone();
                next.amnesia = v;
                if autobump {
                    next.version = next.version.saturating_add(1);
                }
                cell.set(next.clone());
                metrics.set_amnesia(next.amnesia);
                let _ = bus.publish(KernelEvent::ConfigUpdated { version: next.version });
            }
        }

        let mut last = read_bool_env(key);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(poll_secs));
        loop {
            interval.tick().await;
            let curr = read_bool_env(key);
            if curr != last {
                last = curr;
                if let Some(v) = curr {
                    let old = cell.get();
                    if old.amnesia != v {
                        let mut next = (*old).clone();
                        next.amnesia = v;
                        if autobump {
                            next.version = next.version.saturating_add(1);
                        }
                        cell.set(next.clone());
                        metrics.set_amnesia(next.amnesia);
                        let _ = bus.publish(KernelEvent::ConfigUpdated { version: next.version });
                    }
                }
            }
        }
    });
}

/// Parse boolean-ish env values.
fn read_bool_env(key: &str) -> Option<bool> {
    env::var(key).ok().and_then(|s| match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    })
}
