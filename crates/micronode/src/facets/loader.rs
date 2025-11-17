//! RO:WHAT — Facet registry loader for manifest-driven facets.
//! RO:WHY  — Reads all `*.toml` manifests from a directory and returns a registry.
//! RO:INVARIANTS — Unique facet IDs; route validation applied; static files must exist.

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

    for entry in fs::read_dir(dir).map_err(|e| Error::Config(format!("read_dir {:?}: {e}", dir)))? {
        let entry = entry.map_err(|e| Error::Config(format!("read_dir entry error: {e}")))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }

        let s = fs::read_to_string(&path)
            .map_err(|e| Error::Config(format!("read {:?}: {e}", path)))?;
        let manifest: FacetManifest =
            toml::from_str(&s).map_err(|e| Error::Config(format!("parse {:?}: {e}", path)))?;
        manifest.validate().map_err(|msg| Error::Config(format!("validate {:?}: {msg}", path)))?;

        if !ids.insert(manifest.facet.id.clone()) {
            return Err(Error::Config(format!("duplicate facet id: {}", manifest.facet.id)));
        }

        // Extra static checks: file must exist.
        if let super::manifest::FacetKind::Static = manifest.facet.kind {
            for r in &manifest.route {
                if let Some(ref f) = r.file {
                    let fp = PathBuf::from(f);
                    if !fp.exists() {
                        return Err(Error::Config(format!("static file not found: {:?}", fp)));
                    }
                }
            }
        }

        manifests.push(manifest);
    }

    Ok(FacetRegistry { manifests })
}
