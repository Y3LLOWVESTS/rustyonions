//! RO:WHAT — Facet registry loader for manifest-driven facets.
//! RO:WHY  — Reads all `*.toml` manifests from a directory and returns a registry.
//! RO:INVARIANTS —
//!   - Unique facet IDs among *loaded* manifests.
//!   - Route validation applied.
//!   - Static files must exist.
//!   - Loader is resilient: one bad manifest does NOT break all facets.
//!     (Bad manifests are skipped with warnings.)

use super::manifest::FacetManifest;
use crate::errors::{Error, Result};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct FacetRegistry {
    pub manifests: Vec<FacetManifest>,
}

pub fn load_facets(dir: &Path) -> Result<FacetRegistry> {
    if !dir.exists() {
        return Err(Error::Config(format!("facet dir {:?} does not exist", dir)));
    }
    if !dir.is_dir() {
        return Err(Error::Config(format!("facet dir {:?} is not a directory", dir)));
    }

    let mut manifests = Vec::new();
    let mut ids = HashSet::new();
    let mut skipped = 0usize;

    let rd = fs::read_dir(dir).map_err(|e| Error::Config(format!("read_dir {:?}: {e}", dir)))?;
    for entry in rd {
        let entry = entry.map_err(|e| Error::Config(format!("read_dir entry error: {e}")))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }

        let s = match fs::read_to_string(&path) {
            Ok(v) => v,
            Err(e) => {
                skipped += 1;
                tracing::warn!("facet manifest skipped (read failed): path={:?} err={}", path, e);
                continue;
            }
        };

        // IMPORTANT:
        // Use parse_toml() so we support both v1 and legacy manifests.
        let manifest = match FacetManifest::parse_toml(&s) {
            Ok(m) => m,
            Err(msg) => {
                skipped += 1;
                tracing::warn!(
                    "facet manifest skipped (parse failed): path={:?} err={}",
                    path,
                    msg
                );
                continue;
            }
        };

        if let Err(msg) = manifest.validate() {
            skipped += 1;
            tracing::warn!("facet manifest skipped (validate failed): path={:?} err={}", path, msg);
            continue;
        }

        if !ids.insert(manifest.facet.id.clone()) {
            skipped += 1;
            tracing::warn!(
                "facet manifest skipped (duplicate facet id): path={:?} id={}",
                path,
                manifest.facet.id
            );
            continue;
        }

        // Extra static checks: file must exist.
        if let super::manifest::FacetKind::Static = manifest.facet.kind {
            let mut ok = true;
            for r in &manifest.route {
                if let Some(ref f) = r.file {
                    let fp = PathBuf::from(f);
                    if !fp.exists() {
                        skipped += 1;
                        ok = false;
                        tracing::warn!(
                            "facet manifest skipped (static file not found): path={:?} missing={:?}",
                            path,
                            fp
                        );
                        break;
                    }
                }
            }
            if !ok {
                continue;
            }
        }

        manifests.push(manifest);
    }

    if manifests.is_empty() {
        return Err(Error::Config(format!(
            "no valid facet manifests loaded from {:?} (skipped={})",
            dir, skipped
        )));
    }

    if skipped > 0 {
        tracing::warn!(
            "facet loader completed with skipped manifests: dir={:?} loaded={} skipped={}",
            dir,
            manifests.len(),
            skipped
        );
    }

    Ok(FacetRegistry { manifests })
}
