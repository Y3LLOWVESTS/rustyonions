/*!
Config — load + hot-reload + event emission.

Rules:
- Precedence: ENV > FILE > DEFAULTS.
- Autobump version if a semantic toggle (e.g., amnesia) changes without explicit version.
- No-op writes do not emit `ConfigUpdated`.

Surface (kept minimal/by-canon):
- `Config` — current kernel config (version, amnesia).
- `ConfigUpdated` — DTO used by reload/apply decision paths.
- `ConfigCell` — tiny Arc<RwLock<Config>> for shared access (watcher/HTTP/etc.).
- `validation` — guardrails for field sanity.
- `watcher` — polling file-watcher that applies hot-reloads and emits events.
*/

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path};

// Submodules (exported)
pub mod validation;
pub mod watcher;
pub mod cell;

pub use cell::ConfigCell;

/// Kernel configuration loaded from file/env with sane defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    /// Monotonically increasing config version used to order updates.
    pub version: u64,
    /// When `true`, run in amnesia mode (RAM-only posture surfaced to metrics).
    pub amnesia: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            amnesia: true, // keep your current default; demo flips this live via watcher/ENV
        }
    }
}

/// DTO published on the bus when a new configuration becomes active.
#[derive(Debug, Clone)]
pub struct ConfigUpdated {
    /// Version that became active after reload/autobump.
    pub version: u64,
}

/// Load configuration from `path` (if it exists), then apply ENV overrides.
///
/// ENV precedence:
/// - `RON_AMNESIA` — truthy strings: `1|true|on|yes` (case-insensitive)
/// - `RON_VERSION` — `u64` parse
pub fn load_from(path: impl AsRef<Path>) -> Result<Config> {
    // Defaults
    let mut cfg = Config::default();

    // File
    if path.as_ref().exists() {
        let raw = fs::read_to_string(path.as_ref()).with_context(|| "read config file")?;
        let from_file: Config = toml::from_str(&raw).with_context(|| "parse toml")?;
        cfg = from_file;
    }

    // ENV overrides
    if let Ok(s) = env::var("RON_AMNESIA") {
        cfg.amnesia = is_truthy(&s);
    }

    if let Ok(v) = env::var("RON_VERSION") {
        cfg.version = v.parse::<u64>().with_context(|| "RON_VERSION parse")?;
    }

    // Final guardrails
    validation::validate(&cfg)?;

    Ok(cfg)
}

/// Compare `old` vs `new` and decide if we should emit `ConfigUpdated`.
///
/// Logic:
/// - No-op (identical) → `None`
/// - Only `amnesia` flipped and version unchanged → **autobump** and emit
/// - Any change with version increase → emit
pub fn apply_reload(old: &Config, mut new: Config) -> Option<ConfigUpdated> {
    if new == *old {
        return None;
    }

    // Guardrails (defensive): keep decisions on validated data.
    if validation::validate(&new).is_err() {
        // Invalid new config ⇒ do not emit; caller may log/telemetry an error.
        return None;
    }

    let amnesia_changed = new.amnesia != old.amnesia;
    let version_increased = new.version > old.version;

    if amnesia_changed && !version_increased {
        // Autobump: preserve monotonic versioning on semantic-only flips.
        new.version = old.version.saturating_add(1);
        return Some(ConfigUpdated { version: new.version });
    }

    if version_increased || amnesia_changed {
        return Some(ConfigUpdated { version: new.version });
    }

    None
}

/// Accepts "1|true|on|yes" (case-insensitive) as truthy; everything else is false.
#[inline]
fn is_truthy(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "on" | "yes"
    )
}
