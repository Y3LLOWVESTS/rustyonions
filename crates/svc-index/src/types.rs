//! RO:WHAT — DTOs for HTTP responses/requests and WEB3_2 manifest pointer records.
//! RO:WHY  — Interop hygiene; `#[serde(deny_unknown_fields)]`; stable asset/site pointer shape.
//! RO:INVARIANTS — b3 CIDs are canonical `b3:<64 lowercase hex>`; no raw bytes; no wallet mutation.
//! RO:SECURITY — no secrets in payloads; owner fields are references only, not spend authority.

use serde::{Deserialize, Serialize};

/// Response for resolving a name or content key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolveResponse {
    /// Resolved key, usually `name:*` or `b3:<64hex>`.
    pub key: String,
    /// Manifest CID if known.
    pub manifest: Option<String>,
    /// Ranked provider list.
    pub providers: Vec<ProviderEntry>,
    /// Optional entity tag.
    pub etag: Option<String>,
    /// Whether the response was served from cache.
    pub cached: bool,
}

/// Provider entry for a CID.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderEntry {
    /// Provider ID, node address, or route reference.
    pub id: String,
    /// Optional provider region.
    pub region: Option<String>,
    /// Ranking hint.
    pub score: f32,
}

/// Response for provider lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvidersResponse {
    /// Canonical `b3:<64 lowercase hex>` CID.
    pub cid: String,
    /// Ranked providers.
    pub providers: Vec<ProviderEntry>,
    /// Whether the result was truncated.
    pub truncated: bool,
    /// Optional entity tag.
    pub etag: Option<String>,
}

/// Stable error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    /// Stable error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Request body for writing an asset manifest pointer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PutAssetManifestPointer {
    /// Asset kind suffix, e.g. `image`, `video`, `music`, `article`.
    pub asset_kind: String,
    /// Manifest CID that describes this asset.
    pub manifest_cid: String,
    /// Optional passport subject for dev/prod owner checks.
    #[serde(default)]
    pub owner_passport_subject: Option<String>,
    /// Optional owner wallet account reference.
    #[serde(default)]
    pub owner_wallet_account: Option<String>,
    /// Optional caller-supplied timestamp in milliseconds since Unix epoch.
    #[serde(default)]
    pub updated_at_ms: Option<u64>,
}

/// Request body for writing a site manifest pointer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PutSiteManifestPointer {
    /// Manifest CID that describes the site.
    pub manifest_cid: String,
    /// Optional passport subject for dev/prod owner checks.
    #[serde(default)]
    pub owner_passport_subject: Option<String>,
    /// Optional owner wallet account reference.
    #[serde(default)]
    pub owner_wallet_account: Option<String>,
    /// Optional caller-supplied timestamp in milliseconds since Unix epoch.
    #[serde(default)]
    pub updated_at_ms: Option<u64>,
}

/// Stored asset manifest pointer.
///
/// This record is mutable. The asset bytes and manifest bytes remain immutable by CID.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetManifestPointer {
    /// Schema version for forward migration.
    pub version: u16,
    /// Canonical asset CID: `b3:<64 lowercase hex>`.
    pub asset_cid: String,
    /// Canonical asset kind suffix.
    pub asset_kind: String,
    /// Canonical manifest CID: `b3:<64 lowercase hex>`.
    pub manifest_cid: String,
    /// Optional owner passport subject.
    #[serde(default)]
    pub owner_passport_subject: Option<String>,
    /// Optional owner wallet account reference.
    #[serde(default)]
    pub owner_wallet_account: Option<String>,
    /// Last update timestamp in milliseconds since Unix epoch.
    pub updated_at_ms: u64,
}

/// Stored site manifest pointer.
///
/// This record is mutable. The site manifest bytes remain immutable by CID.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SiteManifestPointer {
    /// Schema version for forward migration.
    pub version: u16,
    /// Canonical site/name key.
    pub name: String,
    /// Canonical manifest CID: `b3:<64 lowercase hex>`.
    pub manifest_cid: String,
    /// Optional owner passport subject.
    #[serde(default)]
    pub owner_passport_subject: Option<String>,
    /// Optional owner wallet account reference.
    #[serde(default)]
    pub owner_wallet_account: Option<String>,
    /// Last update timestamp in milliseconds since Unix epoch.
    pub updated_at_ms: u64,
}

/// Normalize and validate a canonical b3 CID.
///
/// Accepts either `b3:<64 lowercase hex>` or raw `<64 lowercase hex>`.
/// Returns canonical `b3:<64 lowercase hex>`.
pub fn normalize_b3_cid(input: &str) -> Result<String, &'static str> {
    let trimmed = input.trim();
    let raw = trimmed.strip_prefix("b3:").unwrap_or(trimmed);

    if raw.len() != 64 {
        return Err("cid must be 64 lowercase hex characters, optionally prefixed with b3:");
    }

    if !raw
        .bytes()
        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err("cid must use lowercase hex characters only");
    }

    Ok(format!("b3:{raw}"))
}

/// Normalize and validate an asset kind suffix.
///
/// This intentionally mirrors the WEB3_2 beta vocabulary while staying string-based
/// so `svc-index` does not own product semantics.
pub fn normalize_asset_kind(input: &str) -> Result<String, &'static str> {
    if input.chars().any(char::is_control) {
        return Err("asset kind contains control characters");
    }

    let kind = input.trim().to_ascii_lowercase();

    if kind.is_empty() {
        return Err("asset kind is required");
    }

    let ok = matches!(
        kind.as_str(),
        "image"
            | "video"
            | "stream"
            | "music"
            | "song"
            | "podcast"
            | "article"
            | "post"
            | "comment"
            | "page"
            | "site"
            | "app"
            | "manifest"
    );

    if !ok {
        return Err("unsupported asset kind");
    }

    Ok(kind)
}

/// Normalize and validate a site/name key.
///
/// This is intentionally conservative for the beta product proof:
/// ASCII lowercase, digits, dot, dash, underscore; no slashes, credentials, or traversal.
pub fn normalize_site_name(input: &str) -> Result<String, &'static str> {
    if input.chars().any(char::is_control) {
        return Err("name contains control characters");
    }

    let name = input.trim().to_ascii_lowercase();

    if name.is_empty() {
        return Err("name is required");
    }

    if name.len() > 253 {
        return Err("name is too long");
    }

    if name.contains('/')
        || name.contains('\\')
        || name.contains('@')
        || name.contains("..")
        || name == "."
        || name == ".."
        || name.starts_with('.')
        || name.ends_with('.')
    {
        return Err("name contains unsafe path or credential characters");
    }

    if name
        .bytes()
        .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')))
    {
        return Err("name contains unsupported characters");
    }

    Ok(name)
}

/// Normalize an optional owner/passport/account reference.
pub fn normalize_optional_ref(
    field: &'static str,
    value: Option<String>,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };

    if value.chars().any(char::is_control) {
        return Err(format!("{field} contains control characters"));
    }

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if trimmed.len() > 512 {
        return Err(format!("{field} is too long"));
    }

    Ok(Some(trimmed.to_owned()))
}
