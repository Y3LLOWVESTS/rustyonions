//! RO:WHAT — Optional manifest artifact writer for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: GOV/RES. Non-amnesia deployments can retain audit artifacts.
//! RO:INTERACTS — outputs::manifest, config::Config.
//! RO:INVARIANTS — amnesia mode writes nothing; filenames derive from sanitized epoch ids.
//! RO:METRICS — artifact write failures are counted by caller as internal/dependency errors later.
//! RO:CONFIG — rewarder.artifact_dir and amnesia.enabled.
//! RO:SECURITY — avoids path traversal by sanitizing epoch id.
//! RO:TEST — compile coverage; future artifact tests.

use std::path::PathBuf;

use crate::config::Config;
use crate::outputs::manifest::RewardManifest;
use crate::Result;

/// Write a manifest artifact if amnesia mode is disabled. Returns written path or None.
pub fn maybe_write_manifest(cfg: &Config, manifest: &RewardManifest) -> Result<Option<PathBuf>> {
    if cfg.amnesia.enabled {
        return Ok(None);
    }
    let dir = PathBuf::from(&cfg.rewarder.artifact_dir);
    std::fs::create_dir_all(&dir)?;
    let file = format!("{}.run.json", sanitize(&manifest.epoch_id));
    let path = dir.join(file);
    let bytes = serde_json::to_vec_pretty(manifest)?;
    std::fs::write(&path, bytes)?;
    Ok(Some(path))
}

fn sanitize(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
