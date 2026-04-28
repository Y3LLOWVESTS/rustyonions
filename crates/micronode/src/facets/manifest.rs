//! RO:WHAT — Facet manifest schema (TOML).
//! RO:WHY  — Declarative facets loaded from a directory.
//! RO:FORMAT (v1, preferred) —
//!   [facet]
//!   id = "docs"
//!   kind = "static" | "echo" | "proxy"
//!
//!   # For kind="proxy":
//!   [upstream]
//!   scheme = "http" | "https"
//!   host = "127.0.0.1"
//!   port = 5401
//!   base_path = "/optional-prefix"   # optional
//!
//!   [[route]]
//!   method = "GET" | "POST" | "PUT" | "PATCH" | "DELETE"
//!   path = "/ping"
//!   # for kind="static":
//!   file = "configs/static/hello.txt"
//!   # for kind="proxy":
//!   upstream_path = "/real-upstream-path"   # optional
//!
//! LEGACY COMPAT (v0) — supported for older demo manifests:
//!   id = "wow"
//!   kind = "static"
//!   routes = [ { path="/", file="..." }, ... ]
//!
//! RO:INVARIANTS — Enforced by `validate()`:
//!   - facet.id not empty
//!   - route.path starts with "/"
//!   - methods are from a small allowlist
//!   - static: GET only and file present
//!   - proxy: upstream present and valid

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct FacetManifest {
    pub facet: FacetHeader,
    pub upstream: Option<UpstreamSpec>,
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
pub struct UpstreamSpec {
    pub scheme: String, // "http" | "https"
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub base_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteSpec {
    /// HTTP method (case-insensitive accepted by our parser).
    pub method: String,
    /// Must start with "/".
    pub path: String,
    /// For Static kind: local file to serve for GET.
    #[serde(default)]
    pub file: Option<String>,
    /// For Proxy kind: optional upstream path override.
    #[serde(default)]
    pub upstream_path: Option<String>,
}

// ----- Parsing (v1 + legacy compat) -----

#[derive(Debug, Clone, Deserialize)]
struct V1Doc {
    pub facet: FacetHeader,
    #[serde(default)]
    pub upstream: Option<UpstreamSpec>,
    #[serde(default)]
    pub route: Vec<RouteSpec>,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyDoc {
    pub id: String,
    #[serde(rename = "kind")]
    pub kind: FacetKind,
    #[serde(default)]
    pub routes: Vec<LegacyRoute>,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyRoute {
    pub path: String,
    #[serde(default)]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum AnyDoc {
    V1(V1Doc),
    Legacy(LegacyDoc),
}

impl FacetManifest {
    /// Parse TOML into the canonical in-memory manifest, supporting legacy format.
    pub fn parse_toml(s: &str) -> Result<Self, String> {
        let doc: AnyDoc = toml::from_str(s).map_err(|e| format!("parse toml: {e}"))?;
        match doc {
            AnyDoc::V1(v1) => {
                Ok(FacetManifest { facet: v1.facet, upstream: v1.upstream, route: v1.route })
            }
            AnyDoc::Legacy(leg) => {
                // Legacy had only static-style routes. Convert into canonical.
                let route = leg
                    .routes
                    .into_iter()
                    .map(|r| RouteSpec {
                        method: "GET".to_string(),
                        path: r.path,
                        file: r.file,
                        upstream_path: None,
                    })
                    .collect();

                Ok(FacetManifest {
                    facet: FacetHeader { id: leg.id, kind: leg.kind },
                    upstream: None,
                    route,
                })
            }
        }
    }

    /// Minimal validation and normalization.
    pub fn validate(&self) -> Result<(), String> {
        if self.facet.id.trim().is_empty() {
            return Err("facet.id must not be empty".into());
        }

        // Upstream rules (only required for proxy facets).
        if let FacetKind::Proxy = self.facet.kind {
            let up = self
                .upstream
                .as_ref()
                .ok_or_else(|| "proxy facet requires [upstream]".to_string())?;

            let scheme = up.scheme.trim().to_ascii_lowercase();
            if scheme != "http" && scheme != "https" {
                return Err(format!("upstream.scheme must be http or https: {}", up.scheme));
            }
            if up.host.trim().is_empty() {
                return Err("upstream.host must not be empty".into());
            }
            if up.port == 0 {
                return Err("upstream.port must be > 0".into());
            }
            if let Some(bp) = &up.base_path {
                if !bp.is_empty() && !bp.starts_with('/') {
                    return Err(format!("upstream.base_path must start with '/': {bp}"));
                }
            }
        }

        for r in &self.route {
            if !r.path.starts_with('/') {
                return Err(format!("route.path must start with '/': {}", r.path));
            }

            let m = r.method.to_ascii_uppercase();
            let ok = matches!(m.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE");
            if !ok {
                return Err(format!(
                    "route.method must be one of GET/POST/PUT/PATCH/DELETE: {}",
                    r.method
                ));
            }

            match self.facet.kind {
                FacetKind::Static => {
                    if m != "GET" {
                        return Err("static routes must be GET".into());
                    }
                    if r.file.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                        return Err("static route requires non-empty 'file'".into());
                    }
                }
                FacetKind::Proxy => {
                    // If present, upstream_path must also start with "/"
                    if let Some(up) = &r.upstream_path {
                        if !up.starts_with('/') {
                            return Err(format!("route.upstream_path must start with '/': {up}"));
                        }
                    }
                }
                FacetKind::Echo => {
                    // Echo is intentionally minimal. (Can be extended later.)
                }
            }
        }

        Ok(())
    }
}
