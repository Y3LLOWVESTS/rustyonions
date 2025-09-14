#![forbid(unsafe_code)]

//! File watching + hot-reload publisher for `config.toml`.
//! Self-heals if the watcher channel closes unexpectedly.

use std::{
    env,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use tracing::{error, info, warn};

use super::types::load_from_file;
use crate::{bus::Bus, metrics::HealthState, KernelEvent};

/// Monotonic version for successfully committed configs.
static VERSION: AtomicU64 = AtomicU64::new(0);

pub fn spawn_config_watcher<P: Into<PathBuf>>(
    path: P,
    bus: Bus,
    health: Arc<HealthState>,
) -> tokio::task::JoinHandle<()> {
    let path = path.into();
    tokio::spawn(async move {
        // Run the blocking watch loop on a dedicated threadpool thread.
        let _ = tokio::task::spawn_blocking(move || watch_forever(path, bus, health)).await;
    })
}

fn watch_forever(path: PathBuf, bus: Bus, health: Arc<HealthState>) {
    // If the inner watcher loop aborts (e.g., channel closed), sleep and try again.
    loop {
        if let Err(e) = watch_once(&path, &bus, &health) {
            error!(error = %e, "config watcher encountered an error; restarting in 1s");
            thread::sleep(Duration::from_secs(1));
        }
    }
}

fn watch_once(path: &Path, bus: &Bus, health: &Arc<HealthState>) -> anyhow::Result<()> {
    // Initial load (non-fatal if missing/invalid)
    match load_from_file(path) {
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

    // Normalize watch dir so passing just "config.toml" doesn't produce "".
    let watch_dir: PathBuf = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => {
            warn!("config file has no parent directory; watching current directory");
            env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    };

    // Channel for notify callback -> loop
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            let _ = tx.send(res);
        },
        NotifyConfig::default()
            .with_poll_interval(Duration::from_millis(750))
            .with_compare_contents(true),
    )?;

    watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;

    // Debounce simple: small sleep after an event burst.
    let debounce = Duration::from_millis(200);

    loop {
        match rx.recv() {
            Ok(Ok(ev)) => {
                // Only act on relevant events
                match ev.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        thread::sleep(debounce);
                        match load_from_file(path) {
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
                // Channel closed; return Err so the outer loop can recreate the watcher.
                return Err(anyhow::anyhow!("config watcher channel closed: {e}"));
            }
        }
    }
}
