//! RO:WHAT — Facet manifest schema (TOML).
//! RO:WHY  — Declarative facets loaded from a directory.
//! RO:FORMAT —
//!   [[route]]
//!   method = "GET" | "POST"
//!   path = "/ping"
//!   # for kind="static":
//!   file = "configs/static/hello.txt"
//!
//!   [facet]
//!   id = "docs"
//!   kind = "static" | "echo" | "proxy"

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct FacetManifest {
    pub facet: FacetHeader,
    #[serde(default)]
    pub route: Vec<RouteSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FacetHeader {
    pub id: String,
    #[serde(rename = "kind")]
    pub kind: FacetKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FacetKind {
    Static,
    Echo,
    Proxy,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteSpec {
    /// "GET" or "POST" (case-insensitive accepted by our parser).
    pub method: String,
    /// Must start with "/".
    pub path: String,
    /// For Static kind: local file to serve for GET.
    #[serde(default)]
    pub file: Option<String>,
}

impl FacetManifest {
    /// Minimal validation and normalization.
    pub fn validate(&self) -> Result<(), String> {
        if self.facet.id.trim().is_empty() {
            return Err("facet.id must not be empty".into());
        }
        for r in &self.route {
            if !r.path.starts_with('/') {
                return Err(format!("route.path must start with '/': {}", r.path));
            }
            let m = r.method.to_ascii_uppercase();
            if m != "GET" && m != "POST" {
                return Err(format!("route.method must be GET or POST: {}", r.method));
            }
            if let FacetKind::Static = self.facet.kind {
                if m != "GET" {
                    return Err("static routes must be GET".into());
                }
                if r.file.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                    return Err("static route requires non-empty 'file'".into());
                }
            }
        }
        Ok(())
    }
}
