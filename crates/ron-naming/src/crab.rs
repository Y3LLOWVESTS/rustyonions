//! RO:WHAT — Pure parser/normalizer for WEB3_2 `crab://` links.
//! RO:WHY  — Pillar 9, Concerns: SEC/DX/GOV; gives product routes one canonical link grammar.
//! RO:INTERACTS — asset::AssetKind, normalize::normalize_fqdn_ascii, types::{ContentId,Fqdn}.
//! RO:INVARIANTS — no IO; no async; canonical internal CID is "b3:<64 lowercase hex>"; URL omits b3/.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects credentials, control chars, traversal, malformed hashes, unknown asset kinds.
//! RO:TEST — tests/crab_links.rs.

use crate::{
    asset::AssetKind,
    normalize::normalize_fqdn_ascii,
    types::{ContentId, Fqdn},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, str::FromStr};

/// Canonical RON product link scheme.
pub const CRAB_SCHEME: &str = "crab";

/// Namespace selected by a parsed crab link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrabNamespace {
    /// `crab://<64hex>.<asset_kind>`.
    B3,
    /// `crab://site/<name>`.
    Site,
    /// `crab://name/<name>`.
    Name,
    /// `crab://<name>`.
    RootName,
}

impl CrabNamespace {
    /// Canonical namespace text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            CrabNamespace::B3 => "b3",
            CrabNamespace::Site => "site",
            CrabNamespace::Name => "name",
            CrabNamespace::RootName => "root",
        }
    }
}

/// Parsed crab route payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "route", rename_all = "snake_case", deny_unknown_fields)]
pub enum CrabRoute {
    /// Typed immutable b3 asset route.
    B3Asset {
        /// Canonical `b3:<64 lowercase hex>` content id.
        cid: ContentId,
        /// Canonical lowercase raw hash without the `b3:` prefix.
        raw_hash_hex: String,
        /// Typed WEB3_2 asset kind suffix.
        asset_kind: AssetKind,
    },
    /// Named route.
    Named {
        /// Name namespace.
        namespace: CrabNamespace,
        /// Normalized ASCII name.
        name: Fqdn,
    },
}

/// Parsed, normalized `crab://` link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrabLink {
    route: CrabRoute,
    query: BTreeMap<String, String>,
}

impl CrabLink {
    /// Parse a crab link and normalize it into a stable representation.
    pub fn parse(input: &str) -> Result<Self, CrabParseError> {
        reject_unsafe_input(input)?;

        let rest = input
            .strip_prefix("crab://")
            .ok_or(CrabParseError::InvalidScheme)?;

        if rest.is_empty() {
            return Err(CrabParseError::EmptyTarget);
        }

        if rest.contains('#') {
            return Err(CrabParseError::FragmentUnsupported);
        }

        let (target, query) = match rest.split_once('?') {
            Some((target, query)) => (target, Some(query)),
            None => (rest, None),
        };

        validate_target_hygiene(target)?;
        let query = parse_query(query)?;

        let route = parse_route(target)?;
        Ok(Self { route, query })
    }

    /// Return the canonical scheme, always `crab`.
    #[must_use]
    pub const fn scheme(&self) -> &'static str {
        CRAB_SCHEME
    }

    /// Return the parsed route.
    #[must_use]
    pub const fn route(&self) -> &CrabRoute {
        &self.route
    }

    /// Return the parsed namespace.
    #[must_use]
    pub const fn namespace(&self) -> CrabNamespace {
        match &self.route {
            CrabRoute::B3Asset { .. } => CrabNamespace::B3,
            CrabRoute::Named { namespace, .. } => *namespace,
        }
    }

    /// Return the canonical b3 CID for typed b3 asset links.
    #[must_use]
    pub fn canonical_b3_cid(&self) -> Option<&ContentId> {
        match &self.route {
            CrabRoute::B3Asset { cid, .. } => Some(cid),
            CrabRoute::Named { .. } => None,
        }
    }

    /// Return the canonical lowercase raw hash without the `b3:` prefix.
    #[must_use]
    pub fn raw_hash_hex(&self) -> Option<&str> {
        match &self.route {
            CrabRoute::B3Asset { raw_hash_hex, .. } => Some(raw_hash_hex.as_str()),
            CrabRoute::Named { .. } => None,
        }
    }

    /// Return the typed asset kind for typed b3 asset links.
    #[must_use]
    pub const fn asset_kind(&self) -> Option<AssetKind> {
        match &self.route {
            CrabRoute::B3Asset { asset_kind, .. } => Some(*asset_kind),
            CrabRoute::Named { .. } => None,
        }
    }

    /// Return the normalized name for named crab links.
    #[must_use]
    pub fn name(&self) -> Option<&Fqdn> {
        match &self.route {
            CrabRoute::Named { name, .. } => Some(name),
            CrabRoute::B3Asset { .. } => None,
        }
    }

    /// Return normalized query parameters in canonical key order.
    #[must_use]
    pub const fn query_params(&self) -> &BTreeMap<String, String> {
        &self.query
    }

    /// Render the stable canonical string form.
    #[must_use]
    pub fn canonical_string(&self) -> String {
        let mut out = match &self.route {
            CrabRoute::B3Asset {
                raw_hash_hex,
                asset_kind,
                ..
            } => {
                format!("crab://{raw_hash_hex}.{asset_kind}")
            }
            CrabRoute::Named {
                namespace: CrabNamespace::Site,
                name,
            } => {
                format!("crab://site/{}", name.0)
            }
            CrabRoute::Named {
                namespace: CrabNamespace::Name,
                name,
            } => {
                format!("crab://name/{}", name.0)
            }
            CrabRoute::Named {
                namespace: CrabNamespace::RootName,
                name,
            } => {
                format!("crab://{}", name.0)
            }
            CrabRoute::Named {
                namespace: CrabNamespace::B3,
                name,
            } => {
                format!("crab://{}", name.0)
            }
        };

        if !self.query.is_empty() {
            out.push('?');
            for (idx, (key, value)) in self.query.iter().enumerate() {
                if idx > 0 {
                    out.push('&');
                }
                out.push_str(key);
                out.push('=');
                out.push_str(value);
            }
        }

        out
    }
}

impl fmt::Display for CrabLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical_string())
    }
}

impl FromStr for CrabLink {
    type Err = CrabParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Deterministic parser errors for `crab://` links.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum CrabParseError {
    /// Scheme was not exactly `crab://`.
    #[error("invalid crab scheme")]
    InvalidScheme,
    /// Link target after `crab://` was empty.
    #[error("empty crab target")]
    EmptyTarget,
    /// Input contained ASCII or Unicode control characters.
    #[error("unsafe control character")]
    UnsafeControlCharacter,
    /// Fragment identifiers are not part of the beta crab grammar.
    #[error("fragment unsupported")]
    FragmentUnsupported,
    /// Embedded credentials or userinfo-like syntax was detected.
    #[error("embedded credentials rejected")]
    EmbeddedCredentials,
    /// Path contained traversal or unsafe path metacharacters.
    #[error("path traversal rejected")]
    PathTraversal,
    /// Namespace/path shape is not supported.
    #[error("unsupported crab namespace: {namespace}")]
    UnsupportedNamespace {
        /// Unsupported namespace.
        namespace: String,
    },
    /// Link path is malformed for the selected namespace.
    #[error("invalid crab path: {reason}")]
    InvalidPath {
        /// Stable reason string.
        reason: &'static str,
    },
    /// Typed b3 asset link is missing its asset kind suffix.
    #[error("missing asset kind")]
    MissingAssetKind,
    /// b3 hash failed validation.
    #[error("invalid b3 hash: {reason}")]
    InvalidHash {
        /// Stable reason string.
        reason: &'static str,
    },
    /// Asset suffix failed validation.
    #[error("invalid asset kind: {kind}")]
    InvalidAssetKind {
        /// Invalid normalized asset kind.
        kind: String,
    },
    /// Name failed normalization/hygiene checks.
    #[error("invalid crab name: {name}")]
    InvalidName {
        /// Rejected name.
        name: String,
    },
    /// Query string failed safe normalization.
    #[error("invalid query parameter: {reason}")]
    InvalidQuery {
        /// Stable reason string.
        reason: &'static str,
    },
    /// Query string contained the same key more than once.
    #[error("duplicate query key: {key}")]
    DuplicateQueryKey {
        /// Duplicate key.
        key: String,
    },
}

impl CrabParseError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            CrabParseError::InvalidScheme => "invalid_scheme",
            CrabParseError::EmptyTarget => "empty_target",
            CrabParseError::UnsafeControlCharacter => "unsafe_control_character",
            CrabParseError::FragmentUnsupported => "fragment_unsupported",
            CrabParseError::EmbeddedCredentials => "embedded_credentials",
            CrabParseError::PathTraversal => "path_traversal",
            CrabParseError::UnsupportedNamespace { .. } => "unsupported_namespace",
            CrabParseError::InvalidPath { .. } => "invalid_path",
            CrabParseError::MissingAssetKind => "missing_asset_kind",
            CrabParseError::InvalidHash { .. } => "invalid_hash",
            CrabParseError::InvalidAssetKind { .. } => "invalid_asset_kind",
            CrabParseError::InvalidName { .. } => "invalid_name",
            CrabParseError::InvalidQuery { .. } => "invalid_query",
            CrabParseError::DuplicateQueryKey { .. } => "duplicate_query_key",
        }
    }
}

fn reject_unsafe_input(input: &str) -> Result<(), CrabParseError> {
    if input.chars().any(char::is_control) {
        return Err(CrabParseError::UnsafeControlCharacter);
    }
    Ok(())
}

fn validate_target_hygiene(target: &str) -> Result<(), CrabParseError> {
    if target.is_empty() {
        return Err(CrabParseError::EmptyTarget);
    }

    if target.contains('@') {
        return Err(CrabParseError::EmbeddedCredentials);
    }

    if target.contains('\\') {
        return Err(CrabParseError::PathTraversal);
    }

    if target
        .split('/')
        .any(|segment| segment == "." || segment == "..")
    {
        return Err(CrabParseError::PathTraversal);
    }

    let lower = target.to_ascii_lowercase();
    if lower.contains("%2e") || lower.contains("%2f") || lower.contains("%5c") {
        return Err(CrabParseError::PathTraversal);
    }

    Ok(())
}

fn parse_route(target: &str) -> Result<CrabRoute, CrabParseError> {
    let mut parts = target.split('/');
    let first = parts.next().ok_or(CrabParseError::EmptyTarget)?;

    match first {
        "site" => {
            let name = parse_single_name_segment(parts, "site")?;
            Ok(CrabRoute::Named {
                namespace: CrabNamespace::Site,
                name,
            })
        }
        "name" => {
            let name = parse_single_name_segment(parts, "name")?;
            Ok(CrabRoute::Named {
                namespace: CrabNamespace::Name,
                name,
            })
        }
        other => {
            if target.contains('/') {
                return Err(CrabParseError::UnsupportedNamespace {
                    namespace: other.to_owned(),
                });
            }

            if let Some(route) = try_parse_root_asset_target(target)? {
                return Ok(route);
            }

            let name = normalize_name(other)?;
            Ok(CrabRoute::Named {
                namespace: CrabNamespace::RootName,
                name,
            })
        }
    }
}

fn parse_single_name_segment<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    namespace: &'static str,
) -> Result<Fqdn, CrabParseError> {
    let segment = parts.next().ok_or(CrabParseError::InvalidPath {
        reason: "missing name segment",
    })?;

    if parts.next().is_some() {
        return Err(CrabParseError::InvalidPath {
            reason: "too many name path segments",
        });
    }

    if segment.is_empty() {
        return Err(CrabParseError::InvalidPath {
            reason: "empty name segment",
        });
    }

    let name = normalize_name(segment)?;
    if namespace == "site" || namespace == "name" {
        Ok(name)
    } else {
        Err(CrabParseError::UnsupportedNamespace {
            namespace: namespace.to_owned(),
        })
    }
}

fn try_parse_root_asset_target(target: &str) -> Result<Option<CrabRoute>, CrabParseError> {
    if let Some((maybe_hash, _suffix)) = target.rsplit_once('.') {
        if maybe_hash.len() == 64 {
            return parse_b3_asset_segment(target).map(Some);
        }
    }

    if target.len() == 64 && target.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CrabParseError::MissingAssetKind);
    }

    Ok(None)
}

fn parse_b3_asset_segment(segment: &str) -> Result<CrabRoute, CrabParseError> {
    if segment.is_empty() {
        return Err(CrabParseError::InvalidPath {
            reason: "empty b3 asset segment",
        });
    }

    let (hash_hex, suffix) = segment
        .rsplit_once('.')
        .ok_or(CrabParseError::MissingAssetKind)?;

    if suffix.is_empty() {
        return Err(CrabParseError::MissingAssetKind);
    }

    let raw_hash_hex = canonicalize_hash_hex(hash_hex)?;
    let asset_kind =
        AssetKind::from_suffix(suffix).map_err(|_| CrabParseError::InvalidAssetKind {
            kind: suffix.to_ascii_lowercase(),
        })?;

    let cid = ContentId(format!("b3:{raw_hash_hex}"));
    debug_assert!(cid.validate());

    Ok(CrabRoute::B3Asset {
        cid,
        raw_hash_hex,
        asset_kind,
    })
}

fn canonicalize_hash_hex(hash_hex: &str) -> Result<String, CrabParseError> {
    if hash_hex.len() != 64 {
        return Err(CrabParseError::InvalidHash {
            reason: "hash must be 64 hex characters",
        });
    }

    let mut out = String::with_capacity(64);
    for byte in hash_hex.bytes() {
        match byte {
            b'0'..=b'9' | b'a'..=b'f' => out.push(char::from(byte)),
            b'A'..=b'F' => out.push(char::from(byte).to_ascii_lowercase()),
            _ => {
                return Err(CrabParseError::InvalidHash {
                    reason: "hash contains non-hex characters",
                });
            }
        }
    }

    Ok(out)
}

fn normalize_name(name: &str) -> Result<Fqdn, CrabParseError> {
    normalize_fqdn_ascii(name)
        .map(|normalized| normalized.0)
        .map_err(|_| CrabParseError::InvalidName {
            name: name.to_owned(),
        })
}

fn parse_query(query: Option<&str>) -> Result<BTreeMap<String, String>, CrabParseError> {
    let Some(query) = query else {
        return Ok(BTreeMap::new());
    };

    if query.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut out = BTreeMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            return Err(CrabParseError::InvalidQuery {
                reason: "empty query pair",
            });
        }

        let (raw_key, raw_value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };

        if raw_key.is_empty() {
            return Err(CrabParseError::InvalidQuery {
                reason: "empty query key",
            });
        }

        let key = normalize_query_component(raw_key)?;
        let value = normalize_query_component(raw_value)?;

        if out.insert(key.clone(), value).is_some() {
            return Err(CrabParseError::DuplicateQueryKey { key });
        }
    }

    Ok(out)
}

fn normalize_query_component(input: &str) -> Result<String, CrabParseError> {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut idx = 0;

    while idx < bytes.len() {
        match bytes[idx] {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b':' => {
                out.push(char::from(bytes[idx]));
                idx += 1;
            }
            b'%' => {
                if idx + 2 >= bytes.len() {
                    return Err(CrabParseError::InvalidQuery {
                        reason: "bad percent encoding",
                    });
                }

                let hi = bytes[idx + 1];
                let lo = bytes[idx + 2];
                if !hi.is_ascii_hexdigit() || !lo.is_ascii_hexdigit() {
                    return Err(CrabParseError::InvalidQuery {
                        reason: "bad percent encoding",
                    });
                }

                out.push('%');
                out.push(char::from(hi).to_ascii_uppercase());
                out.push(char::from(lo).to_ascii_uppercase());
                idx += 3;
            }
            _ => {
                return Err(CrabParseError::InvalidQuery {
                    reason: "unsafe query character",
                });
            }
        }
    }

    Ok(out)
}
