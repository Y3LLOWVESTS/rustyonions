//! RO:WHAT — WEB3_2 read-only crab/b3 asset-page and built-in page resolver routes.
//! RO:WHY — Product BFF layer hydrates typed asset pages and reserved RustyOnions product pages.
//! RO:INTERACTS — svc-index manifest pointer route, svc-storage object HEAD/GET routes, browser extension later.
//! RO:INVARIANTS — read-only; no wallet/ledger mutation; no byte storage; public URL is crab://<64hex>.<kind>.
//! RO:METRICS — increments local resolver counters for requests/errors.
//! RO:CONFIG — OMNIGATE_INDEX_BASE_URL / OMNIGATE_DOWNSTREAM_INDEX_BASE_URL;
//!             OMNIGATE_STORAGE_BASE_URL / OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL.
//! RO:SECURITY — rejects old b3/ path prefix, traversal, controls, malformed hashes, unknown kinds.
//! RO:TEST — tests/asset_page_resolver.rs; tests/builtin_page_resolver.rs.

use super::sites;
use axum::{
    extract::{Path, Query},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

const DEFAULT_INDEX_BASE_URL: &str = "http://127.0.0.1:5304";
const DEFAULT_STORAGE_BASE_URL: &str = "http://127.0.0.1:15303";
const ASSET_PAGE_SCHEMA: &str = "omnigate.asset-page.v1";
const BUILTIN_PAGE_SCHEMA: &str = "omnigate.builtin-page.v1";
const MAX_MANIFEST_FETCH_BYTES: usize = 1_048_576;

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate crab route reqwest client should build")
});

static CRAB_RESOLVE_TOTAL: AtomicU64 = AtomicU64::new(0);
static CRAB_RESOLVE_ERROR_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Router for `/v1/crab/*`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/resolve", get(resolve_crab_url))
}

/// GET /v1/crab/resolve?url=crab://<64hex>.<kind>
///
/// Read-only resolver for the browser-extension/dashboard path. Also resolves
/// reserved built-in RustyOnions product pages such as `crab://site`,
/// `crab://image`, `crab://music`, and `crab://article` into safe metadata DTOs.
pub async fn resolve_crab_url(
    headers: HeaderMap,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    CRAB_RESOLVE_TOTAL.fetch_add(1, Ordering::Relaxed);

    let Some(url) = query.get("url") else {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_crab_url",
            "missing query parameter: url",
            false,
            "missing_url",
        );
    };

    if let Some(page_kind) = parse_builtin_page_url(url) {
        return Json(builtin_page_response(page_kind)).into_response();
    }

    match parse_crab_asset_url(url) {
        Ok(parsed) => hydrate_asset_page(parsed).await.into_response(),
        Err(err) => {
            if should_try_named_site_fallback(err) {
                if let Some(site_name) = parse_named_site_crab_url(url) {
                    return sites::site_resolve(Path(site_name), headers).await;
                }
            }

            CRAB_RESOLVE_ERROR_TOTAL.fetch_add(1, Ordering::Relaxed);
            problem(
                StatusCode::BAD_REQUEST,
                "invalid_crab_url",
                err.message(),
                false,
                err.code(),
            )
        }
    }
}

/// GET /v1/b3/:asset, where `:asset` is `<64hex>.<kind>`.
///
/// This is the HTTP equivalent of `crab://<64hex>.<kind>`.
pub async fn resolve_b3_asset(Path(asset): Path<String>) -> Response {
    CRAB_RESOLVE_TOTAL.fetch_add(1, Ordering::Relaxed);

    let parsed = match parse_b3_asset_segment(&asset) {
        Ok(parsed) => parsed,
        Err(err) => {
            CRAB_RESOLVE_ERROR_TOTAL.fetch_add(1, Ordering::Relaxed);
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_b3_asset",
                err.message(),
                false,
                err.code(),
            );
        }
    };

    hydrate_asset_page(parsed).await.into_response()
}

/// Parsed canonical asset target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAssetTarget {
    /// Raw lowercase 64-character hash.
    pub raw_hash_hex: String,
    /// Canonical internal CID.
    pub asset_cid: String,
    /// Canonical lowercase asset kind.
    pub asset_kind: String,
    /// Canonical public crab URL.
    pub canonical_crab: String,
}

/// Stored pointer record returned by svc-index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetManifestPointer {
    /// Schema version.
    pub version: u16,
    /// Canonical asset CID.
    pub asset_cid: String,
    /// Canonical asset kind.
    pub asset_kind: String,
    /// Canonical manifest CID.
    pub manifest_cid: String,
    /// Optional owner passport subject.
    #[serde(default)]
    pub owner_passport_subject: Option<String>,
    /// Optional owner wallet account.
    #[serde(default)]
    pub owner_wallet_account: Option<String>,
    /// Updated timestamp in milliseconds since Unix epoch.
    pub updated_at_ms: u64,
}

/// Beta asset page response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetPageResponse {
    /// Stable response schema.
    pub schema: &'static str,
    /// Canonical internal asset CID.
    pub asset_cid: String,
    /// Canonical asset kind.
    pub asset_kind: String,
    /// Manifest pointer/hydration status.
    pub manifest: ManifestSummary,
    /// Storage availability summary for the raw asset object.
    pub storage: StorageSummary,
    /// Optional owner summary from hydrated manifest first, index pointer second.
    #[serde(default)]
    pub owner: Option<OwnerSummary>,
    /// Optional payout summary from hydrated manifest first, index pointer second.
    #[serde(default)]
    pub payout: Option<PayoutSummary>,
    /// Optional product metadata from the manifest object.
    #[serde(default)]
    pub metadata: Option<ManifestMetadataSummary>,
    /// Receipt references discovered in the manifest object.
    #[serde(default)]
    pub receipts: Vec<ReceiptSummary>,
    /// Product links.
    pub links: AssetPageLinks,
    /// Non-fatal hydration warnings.
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Manifest pointer and manifest-object hydration summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManifestSummary {
    /// `present` or `missing` pointer status.
    pub status: &'static str,
    /// `missing`, `pointer_only`, or `hydrated` manifest object status.
    pub hydration_status: &'static str,
    /// Manifest CID if present.
    #[serde(default)]
    pub manifest_cid: Option<String>,
    /// Pointer update timestamp if present.
    #[serde(default)]
    pub updated_at_ms: Option<u64>,
    /// Raw manifest object route if present.
    #[serde(default)]
    pub manifest_url: Option<String>,
}

/// Storage availability summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StorageSummary {
    /// Whether storage HEAD indicated available bytes.
    pub available: bool,
    /// Object size if known.
    #[serde(default)]
    pub size_bytes: Option<u64>,
    /// Content type if known.
    #[serde(default)]
    pub content_type: Option<String>,
    /// Storage/provider ref if surfaced by upstream.
    #[serde(default)]
    pub provider_ref: Option<String>,
}

/// Owner summary from manifest or index pointer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OwnerSummary {
    /// Passport subject.
    pub passport_subject: String,
    /// Wallet account.
    pub wallet_account: String,
}

/// Payout summary for display only.
///
/// This is not wallet authority and does not cause spending.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PayoutSummary {
    /// Default action the frontend can display.
    pub default_action: String,
    /// Recipient account reference.
    pub recipient_account: String,
}

/// Metadata summary extracted from an asset manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManifestMetadataSummary {
    /// Human title.
    #[serde(default)]
    pub title: Option<String>,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Deterministic tag list.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional license string.
    #[serde(default)]
    pub license: Option<String>,
    /// Optional content type.
    #[serde(default)]
    pub content_type: Option<String>,
}

/// Receipt reference extracted from an asset manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptSummary {
    /// Wallet transaction or receipt ID.
    pub tx_id: String,
    /// Receipt kind.
    pub receipt_kind: String,
    /// Optional ROC amount in integer minor units.
    #[serde(default)]
    pub amount_minor_units: Option<u64>,
    /// Optional account associated with the receipt.
    #[serde(default)]
    pub account: Option<String>,
    /// Optional timestamp in milliseconds since Unix epoch.
    #[serde(default)]
    pub created_at_ms: Option<u64>,
}

/// Product links for asset page response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetPageLinks {
    /// Raw object route.
    pub raw: String,
    /// Canonical crab URL.
    pub crab: String,
    /// HTTP equivalent route.
    pub http_b3: String,
    /// Raw manifest object route when a manifest pointer exists.
    #[serde(default)]
    pub manifest: Option<String>,
}

/// Reserved built-in RustyOnions product page kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinPageKind {
    /// `crab://site` — register/launch a manifest-backed RON site.
    Site,
    /// `crab://image` — upload/create a b3 image asset page.
    Image,
    /// `crab://music` — reserved placeholder for future music/song asset workflow.
    Music,
    /// `crab://article` — reserved placeholder for future article/post workflow.
    Article,
}

impl BuiltinPageKind {
    /// Canonical lowercase page-kind string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Site => "site",
            Self::Image => "image",
            Self::Music => "music",
            Self::Article => "article",
        }
    }
}

/// Built-in RustyOnions product page metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuiltinPageResponse {
    /// Stable response schema.
    pub schema: &'static str,
    /// Canonical crab URL.
    pub url: String,
    /// Built-in page kind.
    pub page_kind: String,
    /// Page availability status: `active` or `coming_soon`.
    pub status: String,
    /// Human title for client rendering.
    pub title: String,
    /// Human description for client rendering.
    pub description: String,
    /// Whether the page requires a passport before mutation flows.
    pub requires_passport: bool,
    /// Whether the page requires a wallet before paid mutation flows.
    pub requires_wallet: bool,
    /// Actions clients can render. Mutating actions still require explicit confirmation.
    pub actions: Vec<BuiltinPageAction>,
    /// Fields clients can render. Omnigate describes; downstream routes validate.
    pub fields: Vec<BuiltinPageField>,
    /// Non-fatal warnings for client display.
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Built-in page action metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuiltinPageAction {
    /// Stable action ID.
    pub id: String,
    /// Human label.
    pub label: String,
    /// HTTP method clients should use.
    pub method: String,
    /// Public gateway route for the action.
    pub route: String,
    /// Whether the action mutates backend state.
    pub mutates: bool,
    /// Whether UI must require explicit confirmation before invoking.
    pub requires_confirmation: bool,
}

/// Built-in page field metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuiltinPageField {
    /// Stable field name.
    pub name: String,
    /// Human label.
    pub label: String,
    /// UI field type.
    #[serde(rename = "type")]
    pub field_type: String,
    /// Optional browser file accept hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept: Option<String>,
    /// Whether the field is required.
    pub required: bool,
}

/// Manifest details extracted from a manifest object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManifestDetails {
    /// Owner declared by the manifest.
    #[serde(default)]
    pub owner: Option<OwnerSummary>,
    /// Payout target declared by the manifest.
    #[serde(default)]
    pub payout: Option<PayoutSummary>,
    /// Product metadata declared by the manifest.
    #[serde(default)]
    pub metadata: Option<ManifestMetadataSummary>,
    /// Receipt references declared by the manifest.
    #[serde(default)]
    pub receipts: Vec<ReceiptSummary>,
}

/// Problem response shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResolverProblem<'a> {
    /// Stable problem code.
    pub code: &'a str,
    /// Human-readable message.
    pub message: &'a str,
    /// Whether retry may help.
    pub retryable: bool,
    /// Stable reason.
    pub reason: &'a str,
}

/// Parser error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetParseError {
    /// Missing `crab://`.
    InvalidScheme,
    /// Empty target.
    EmptyTarget,
    /// Old/noncanonical b3 slash prefix.
    B3SlashPrefixRejected,
    /// Unsupported path.
    UnsupportedPath,
    /// Fragment or query in path-equivalent route.
    QueryOrFragmentRejected,
    /// Missing asset kind suffix.
    MissingAssetKind,
    /// Invalid hash length.
    InvalidHashLength,
    /// Invalid hash characters.
    InvalidHashCharacters,
    /// Unsupported asset kind.
    UnsupportedAssetKind,
    /// Unsafe control character.
    UnsafeControlCharacter,
    /// Unsafe path or credential characters.
    UnsafePath,
}

impl AssetParseError {
    /// Stable machine-readable code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidScheme => "invalid_scheme",
            Self::EmptyTarget => "empty_target",
            Self::B3SlashPrefixRejected => "b3_slash_prefix_rejected",
            Self::UnsupportedPath => "unsupported_path",
            Self::QueryOrFragmentRejected => "query_or_fragment_rejected",
            Self::MissingAssetKind => "missing_asset_kind",
            Self::InvalidHashLength => "invalid_hash_length",
            Self::InvalidHashCharacters => "invalid_hash_characters",
            Self::UnsupportedAssetKind => "unsupported_asset_kind",
            Self::UnsafeControlCharacter => "unsafe_control_character",
            Self::UnsafePath => "unsafe_path",
        }
    }

    /// Human-readable message.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidScheme => "expected crab:// URL",
            Self::EmptyTarget => "empty crab target",
            Self::B3SlashPrefixRejected => "old crab://b3/<hash>.<kind> form is noncanonical",
            Self::UnsupportedPath => "unsupported crab path",
            Self::QueryOrFragmentRejected => {
                "query and fragment are not supported in asset segment"
            }
            Self::MissingAssetKind => "missing asset kind suffix",
            Self::InvalidHashLength => "hash must be 64 lowercase hex characters",
            Self::InvalidHashCharacters => "hash must use lowercase hex characters only",
            Self::UnsupportedAssetKind => "unsupported asset kind",
            Self::UnsafeControlCharacter => "input contains control characters",
            Self::UnsafePath => "input contains unsafe path or credential characters",
        }
    }
}

/// Manifest hydration error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestHydrationError {
    /// Manifest was not valid JSON.
    BadJson,
    /// Manifest JSON root was not an object.
    NotObject,
    /// Manifest version is missing or unsupported.
    InvalidVersion,
    /// Manifest asset CID does not match the requested asset.
    AssetCidMismatch,
    /// Manifest asset kind does not match the requested suffix.
    AssetKindMismatch,
    /// Manifest CID field does not match the index pointer.
    ManifestCidMismatch,
}

impl ManifestHydrationError {
    /// Stable warning code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BadJson => "manifest_bad_json",
            Self::NotObject => "manifest_not_object",
            Self::InvalidVersion => "manifest_invalid_version",
            Self::AssetCidMismatch => "manifest_asset_cid_mismatch",
            Self::AssetKindMismatch => "manifest_asset_kind_mismatch",
            Self::ManifestCidMismatch => "manifest_cid_mismatch",
        }
    }
}

/// Parse a reserved built-in product page URL.
///
/// This parser is intentionally tiny and pure. It does not hydrate, query
/// indexes, or grant authority. Business logic stays in downstream routes.
#[must_use]
pub fn parse_builtin_page_url(input: &str) -> Option<BuiltinPageKind> {
    if reject_control_chars(input).is_err() {
        return None;
    }

    let target = input.strip_prefix("crab://")?;

    if target.contains('/')
        || target.contains('\\')
        || target.contains('@')
        || target.contains('?')
        || target.contains('#')
        || target.contains('.')
        || target.contains("..")
    {
        return None;
    }

    match target {
        "site" => Some(BuiltinPageKind::Site),
        "image" => Some(BuiltinPageKind::Image),
        "music" => Some(BuiltinPageKind::Music),
        "article" => Some(BuiltinPageKind::Article),
        _ => None,
    }
}

/// Build the safe JSON metadata DTO for a reserved built-in product page.
#[must_use]
pub fn builtin_page_response(page_kind: BuiltinPageKind) -> BuiltinPageResponse {
    match page_kind {
        BuiltinPageKind::Site => BuiltinPageResponse {
            schema: BUILTIN_PAGE_SCHEMA,
            url: "crab://site".to_owned(),
            page_kind: "site".to_owned(),
            status: "active".to_owned(),
            title: "Create a RON Site".to_owned(),
            description: "Register a name and launch a manifest-backed RustyOnions site."
                .to_owned(),
            requires_passport: true,
            requires_wallet: true,
            actions: vec![
                BuiltinPageAction {
                    id: "site.prepare".to_owned(),
                    label: "Prepare site launch".to_owned(),
                    method: "POST".to_owned(),
                    route: "/sites/prepare".to_owned(),
                    mutates: false,
                    requires_confirmation: false,
                },
                BuiltinPageAction {
                    id: "site.create".to_owned(),
                    label: "Create site".to_owned(),
                    method: "POST".to_owned(),
                    route: "/sites".to_owned(),
                    mutates: true,
                    requires_confirmation: true,
                },
            ],
            fields: vec![
                BuiltinPageField {
                    name: "site_name".to_owned(),
                    label: "Site name".to_owned(),
                    field_type: "text".to_owned(),
                    accept: None,
                    required: true,
                },
                BuiltinPageField {
                    name: "title".to_owned(),
                    label: "Title".to_owned(),
                    field_type: "text".to_owned(),
                    accept: None,
                    required: false,
                },
                BuiltinPageField {
                    name: "description".to_owned(),
                    label: "Description".to_owned(),
                    field_type: "textarea".to_owned(),
                    accept: None,
                    required: false,
                },
            ],
            warnings: Vec::new(),
        },
        BuiltinPageKind::Image => BuiltinPageResponse {
            schema: BUILTIN_PAGE_SCHEMA,
            url: "crab://image".to_owned(),
            page_kind: "image".to_owned(),
            status: "active".to_owned(),
            title: "Upload a RON Image".to_owned(),
            description:
                "Upload an image, create a b3 image asset page, and attach ownership/payout metadata."
                    .to_owned(),
            requires_passport: true,
            requires_wallet: true,
            actions: vec![
                BuiltinPageAction {
                    id: "image.prepare".to_owned(),
                    label: "Prepare image upload".to_owned(),
                    method: "POST".to_owned(),
                    route: "/assets/image/prepare".to_owned(),
                    mutates: false,
                    requires_confirmation: false,
                },
                BuiltinPageAction {
                    id: "image.create".to_owned(),
                    label: "Create image asset".to_owned(),
                    method: "POST".to_owned(),
                    route: "/assets/image".to_owned(),
                    mutates: true,
                    requires_confirmation: true,
                },
            ],
            fields: vec![
                BuiltinPageField {
                    name: "file".to_owned(),
                    label: "Image file".to_owned(),
                    field_type: "file".to_owned(),
                    accept: Some("image/*".to_owned()),
                    required: true,
                },
                BuiltinPageField {
                    name: "title".to_owned(),
                    label: "Title".to_owned(),
                    field_type: "text".to_owned(),
                    accept: None,
                    required: false,
                },
                BuiltinPageField {
                    name: "description".to_owned(),
                    label: "Description".to_owned(),
                    field_type: "textarea".to_owned(),
                    accept: None,
                    required: false,
                },
                BuiltinPageField {
                    name: "tags".to_owned(),
                    label: "Tags".to_owned(),
                    field_type: "tags".to_owned(),
                    accept: None,
                    required: false,
                },
            ],
            warnings: Vec::new(),
        },
        BuiltinPageKind::Music => BuiltinPageResponse {
            schema: BUILTIN_PAGE_SCHEMA,
            url: "crab://music".to_owned(),
            page_kind: "music".to_owned(),
            status: "coming_soon".to_owned(),
            title: "RON Music Is Coming Soon".to_owned(),
            description:
                "The RustyOnions music workflow is reserved, but upload/manifest routes are not enabled yet."
                    .to_owned(),
            requires_passport: false,
            requires_wallet: false,
            actions: Vec::new(),
            fields: vec![
                BuiltinPageField {
                    name: "title".to_owned(),
                    label: "Song title".to_owned(),
                    field_type: "text".to_owned(),
                    accept: None,
                    required: false,
                },
                BuiltinPageField {
                    name: "artist".to_owned(),
                    label: "Artist".to_owned(),
                    field_type: "text".to_owned(),
                    accept: None,
                    required: false,
                },
            ],
            warnings: vec!["music_page_coming_soon".to_owned()],
        },
        BuiltinPageKind::Article => BuiltinPageResponse {
            schema: BUILTIN_PAGE_SCHEMA,
            url: "crab://article".to_owned(),
            page_kind: "article".to_owned(),
            status: "coming_soon".to_owned(),
            title: "RON Articles Are Coming Soon".to_owned(),
            description:
                "The RustyOnions article workflow is reserved, but article manifest routes are not enabled yet."
                    .to_owned(),
            requires_passport: false,
            requires_wallet: false,
            actions: Vec::new(),
            fields: vec![
                BuiltinPageField {
                    name: "title".to_owned(),
                    label: "Article title".to_owned(),
                    field_type: "text".to_owned(),
                    accept: None,
                    required: false,
                },
                BuiltinPageField {
                    name: "body".to_owned(),
                    label: "Body".to_owned(),
                    field_type: "textarea".to_owned(),
                    accept: None,
                    required: false,
                },
            ],
            warnings: vec!["article_page_coming_soon".to_owned()],
        },
    }
}

/// Parse canonical `crab://<64hex>.<kind>`.
pub fn parse_crab_asset_url(input: &str) -> Result<ParsedAssetTarget, AssetParseError> {
    reject_control_chars(input)?;

    let target = input
        .strip_prefix("crab://")
        .ok_or(AssetParseError::InvalidScheme)?;

    parse_b3_asset_segment(target)
}

fn should_try_named_site_fallback(err: AssetParseError) -> bool {
    matches!(
        err,
        AssetParseError::MissingAssetKind | AssetParseError::InvalidHashLength
    )
}

fn parse_named_site_crab_url(input: &str) -> Option<String> {
    if reject_control_chars(input).is_err() {
        return None;
    }

    let target = input.strip_prefix("crab://")?.trim();

    if target.is_empty()
        || target == "."
        || target == ".."
        || target.len() > 253
        || target.starts_with('.')
        || target.ends_with('.')
        || target.contains("..")
        || target.contains('/')
        || target.contains('\\')
        || target.contains('@')
        || target.contains(' ')
        || target.contains('?')
        || target.contains('#')
        || is_exact_64_lower_hex(target)
    {
        return None;
    }

    let normalized = target.to_ascii_lowercase();

    if normalized
        .bytes()
        .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_'))
    {
        Some(normalized)
    } else {
        None
    }
}

fn is_exact_64_lower_hex(input: &str) -> bool {
    input.len() == 64
        && input
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Parse `<64hex>.<kind>`.
pub fn parse_b3_asset_segment(input: &str) -> Result<ParsedAssetTarget, AssetParseError> {
    reject_control_chars(input)?;

    if input.is_empty() {
        return Err(AssetParseError::EmptyTarget);
    }

    if input.starts_with("b3/") {
        return Err(AssetParseError::B3SlashPrefixRejected);
    }

    if input.contains('/') || input.contains('\\') || input.contains('@') {
        return Err(AssetParseError::UnsupportedPath);
    }

    if input.contains('?') || input.contains('#') {
        return Err(AssetParseError::QueryOrFragmentRejected);
    }

    if input.contains("..") {
        return Err(AssetParseError::UnsafePath);
    }

    let (hash, kind) = input
        .rsplit_once('.')
        .ok_or(AssetParseError::MissingAssetKind)?;

    let raw_hash_hex = canonicalize_hash(hash)?;
    let asset_kind = normalize_asset_kind(kind)?;

    Ok(ParsedAssetTarget {
        asset_cid: format!("b3:{raw_hash_hex}"),
        canonical_crab: format!("crab://{raw_hash_hex}.{asset_kind}"),
        raw_hash_hex,
        asset_kind,
    })
}

/// Extract display-safe manifest details from a JSON asset manifest object.
///
/// This helper intentionally performs no wallet, ledger, storage, or ownership mutation.
pub fn manifest_details_from_json(
    parsed: &ParsedAssetTarget,
    expected_manifest_cid: &str,
    bytes: &[u8],
) -> Result<ManifestDetails, ManifestHydrationError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| ManifestHydrationError::BadJson)?;
    let object = value.as_object().ok_or(ManifestHydrationError::NotObject)?;

    let version_ok = object.get("version").and_then(Value::as_u64) == Some(1);
    if !version_ok {
        return Err(ManifestHydrationError::InvalidVersion);
    }

    let asset_cid = object
        .get("asset_cid")
        .and_then(Value::as_str)
        .ok_or(ManifestHydrationError::AssetCidMismatch)?;
    if asset_cid != parsed.asset_cid {
        return Err(ManifestHydrationError::AssetCidMismatch);
    }

    let asset_kind = object
        .get("asset_kind")
        .and_then(Value::as_str)
        .ok_or(ManifestHydrationError::AssetKindMismatch)?;
    if asset_kind.trim().to_ascii_lowercase() != parsed.asset_kind {
        return Err(ManifestHydrationError::AssetKindMismatch);
    }

    if let Some(manifest_cid) = object.get("manifest_cid").and_then(Value::as_str) {
        if manifest_cid != expected_manifest_cid {
            return Err(ManifestHydrationError::ManifestCidMismatch);
        }
    }

    Ok(ManifestDetails {
        owner: object.get("owner").and_then(owner_from_value),
        payout: object.get("payout").and_then(payout_from_value),
        metadata: object.get("metadata").and_then(metadata_from_value),
        receipts: object
            .get("receipts")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(receipt_from_value).collect())
            .unwrap_or_default(),
    })
}

/// Compose an asset page from already-fetched index/storage summaries.
#[must_use]
pub fn compose_asset_page(
    parsed: ParsedAssetTarget,
    pointer: Option<AssetManifestPointer>,
    storage: StorageSummary,
    warnings: Vec<String>,
) -> AssetPageResponse {
    compose_asset_page_with_manifest(parsed, pointer, storage, None, warnings)
}

/// Compose an asset page from pointer, storage, and optional manifest details.
#[must_use]
pub fn compose_asset_page_with_manifest(
    parsed: ParsedAssetTarget,
    pointer: Option<AssetManifestPointer>,
    storage: StorageSummary,
    manifest_details: Option<ManifestDetails>,
    mut warnings: Vec<String>,
) -> AssetPageResponse {
    let (manifest, pointer_owner, pointer_payout, manifest_link) = match pointer {
        Some(pointer) => {
            let manifest_url = format!("/o/{}", pointer.manifest_cid);
            let pointer_owner = match (
                pointer.owner_passport_subject.clone(),
                pointer.owner_wallet_account.clone(),
            ) {
                (Some(passport_subject), Some(wallet_account)) => Some(OwnerSummary {
                    passport_subject,
                    wallet_account,
                }),
                _ => None,
            };

            let pointer_payout =
                pointer
                    .owner_wallet_account
                    .map(|recipient_account| PayoutSummary {
                        default_action: "content_view".to_owned(),
                        recipient_account,
                    });

            (
                ManifestSummary {
                    status: "present",
                    hydration_status: if manifest_details.is_some() {
                        "hydrated"
                    } else {
                        "pointer_only"
                    },
                    manifest_cid: Some(pointer.manifest_cid),
                    updated_at_ms: Some(pointer.updated_at_ms),
                    manifest_url: Some(manifest_url.clone()),
                },
                pointer_owner,
                pointer_payout,
                Some(manifest_url),
            )
        }
        None => {
            warnings.push("manifest_pointer_missing".to_owned());
            (
                ManifestSummary {
                    status: "missing",
                    hydration_status: "missing",
                    manifest_cid: None,
                    updated_at_ms: None,
                    manifest_url: None,
                },
                None,
                None,
                None,
            )
        }
    };

    let (owner, payout, metadata, receipts) = match manifest_details {
        Some(details) => (
            details.owner.or(pointer_owner),
            details.payout.or(pointer_payout),
            details.metadata,
            details.receipts,
        ),
        None => (pointer_owner, pointer_payout, None, Vec::new()),
    };

    AssetPageResponse {
        schema: ASSET_PAGE_SCHEMA,
        asset_cid: parsed.asset_cid.clone(),
        asset_kind: parsed.asset_kind.clone(),
        manifest,
        storage,
        owner,
        payout,
        metadata,
        receipts,
        links: AssetPageLinks {
            raw: format!("/o/{}", parsed.asset_cid),
            crab: parsed.canonical_crab,
            http_b3: format!("/v1/b3/{}.{}", parsed.raw_hash_hex, parsed.asset_kind),
            manifest: manifest_link,
        },
        warnings,
    }
}

/// Current local resolver counters.
#[must_use]
pub fn resolver_counters() -> (u64, u64) {
    (
        CRAB_RESOLVE_TOTAL.load(Ordering::Relaxed),
        CRAB_RESOLVE_ERROR_TOTAL.load(Ordering::Relaxed),
    )
}

async fn hydrate_asset_page(parsed: ParsedAssetTarget) -> Response {
    let pointer = match fetch_manifest_pointer(&parsed).await {
        Ok(pointer) => pointer,
        Err(err) => {
            CRAB_RESOLVE_ERROR_TOTAL.fetch_add(1, Ordering::Relaxed);
            return problem(
                StatusCode::BAD_GATEWAY,
                "index_upstream_error",
                "index manifest pointer lookup failed",
                err.retryable,
                err.reason,
            );
        }
    };

    if let Some(pointer) = &pointer {
        if pointer.asset_cid != parsed.asset_cid || pointer.asset_kind != parsed.asset_kind {
            CRAB_RESOLVE_ERROR_TOTAL.fetch_add(1, Ordering::Relaxed);
            return problem(
                StatusCode::BAD_GATEWAY,
                "index_pointer_mismatch",
                "index manifest pointer does not match requested asset",
                false,
                "pointer_mismatch",
            );
        }
    }

    let (storage, storage_warning) = fetch_storage_availability(&parsed).await;
    let mut warnings = Vec::new();
    if let Some(warning) = storage_warning {
        warnings.push(warning);
    }

    let manifest_details = if let Some(pointer) = &pointer {
        let (details, warning) = fetch_manifest_details(&parsed, pointer).await;
        if let Some(warning) = warning {
            warnings.push(warning);
        }
        details
    } else {
        None
    };

    let page =
        compose_asset_page_with_manifest(parsed, pointer, storage, manifest_details, warnings);
    (StatusCode::OK, Json(page)).into_response()
}

async fn fetch_manifest_pointer(
    parsed: &ParsedAssetTarget,
) -> Result<Option<AssetManifestPointer>, UpstreamError> {
    let base = index_base_url();
    let url = format!(
        "{}/v1/index/assets/{}/manifest",
        base.trim_end_matches('/'),
        parsed.raw_hash_hex
    );

    let res = HTTP_CLIENT
        .get(url)
        .send()
        .await
        .map_err(|_| UpstreamError::retryable("index_connect"))?;

    if res.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !res.status().is_success() {
        return Err(UpstreamError::from_status(
            "index_http",
            res.status().as_u16(),
        ));
    }

    let text = res
        .text()
        .await
        .map_err(|_| UpstreamError::retryable("index_read"))?;

    serde_json::from_str::<AssetManifestPointer>(&text)
        .map(Some)
        .map_err(|_| UpstreamError::not_retryable("index_bad_json"))
}

async fn fetch_manifest_details(
    parsed: &ParsedAssetTarget,
    pointer: &AssetManifestPointer,
) -> (Option<ManifestDetails>, Option<String>) {
    let base = storage_base_url();
    let url = format!("{}/o/{}", base.trim_end_matches('/'), pointer.manifest_cid);

    let res = HTTP_CLIENT.get(url).send().await;
    let Ok(res) = res else {
        return (None, Some("manifest_upstream_unavailable".to_owned()));
    };

    if res.status() == StatusCode::NOT_FOUND {
        return (None, Some("manifest_object_missing".to_owned()));
    }

    if !res.status().is_success() {
        return (
            None,
            Some(format!("manifest_http_{}", res.status().as_u16())),
        );
    }

    if res
        .content_length()
        .is_some_and(|len| len > MAX_MANIFEST_FETCH_BYTES as u64)
    {
        return (None, Some("manifest_too_large".to_owned()));
    }

    let Ok(bytes) = res.bytes().await else {
        return (None, Some("manifest_read_failed".to_owned()));
    };

    if bytes.len() > MAX_MANIFEST_FETCH_BYTES {
        return (None, Some("manifest_too_large".to_owned()));
    }

    match manifest_details_from_json(parsed, &pointer.manifest_cid, &bytes) {
        Ok(details) => (Some(details), None),
        Err(err) => (None, Some(err.code().to_owned())),
    }
}

async fn fetch_storage_availability(
    parsed: &ParsedAssetTarget,
) -> (StorageSummary, Option<String>) {
    let base = storage_base_url();
    let url = format!("{}/o/{}", base.trim_end_matches('/'), parsed.asset_cid);

    let res = HTTP_CLIENT.head(url).send().await;
    let Ok(res) = res else {
        return (
            StorageSummary {
                available: false,
                size_bytes: None,
                content_type: None,
                provider_ref: None,
            },
            Some("storage_upstream_unavailable".to_owned()),
        );
    };

    if res.status() == StatusCode::NOT_FOUND {
        return (
            StorageSummary {
                available: false,
                size_bytes: None,
                content_type: None,
                provider_ref: None,
            },
            Some("storage_object_missing".to_owned()),
        );
    }

    if !res.status().is_success() {
        return (
            StorageSummary {
                available: false,
                size_bytes: None,
                content_type: None,
                provider_ref: None,
            },
            Some(format!("storage_http_{}", res.status().as_u16())),
        );
    }

    let headers = res.headers();
    let size_bytes = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);

    let provider_ref = headers
        .get("x-ron-provider")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);

    (
        StorageSummary {
            available: true,
            size_bytes,
            content_type,
            provider_ref,
        },
        None,
    )
}

#[derive(Debug, Clone, Copy)]
struct UpstreamError {
    reason: &'static str,
    retryable: bool,
}

impl UpstreamError {
    const fn retryable(reason: &'static str) -> Self {
        Self {
            reason,
            retryable: true,
        }
    }

    const fn not_retryable(reason: &'static str) -> Self {
        Self {
            reason,
            retryable: false,
        }
    }

    const fn from_status(reason: &'static str, status: u16) -> Self {
        Self {
            reason: if status >= 500 {
                reason
            } else {
                "index_non_retryable_http"
            },
            retryable: status >= 500,
        }
    }
}

fn index_base_url() -> String {
    std::env::var("OMNIGATE_INDEX_BASE_URL")
        .or_else(|_| std::env::var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_INDEX_BASE_URL.to_owned())
}

fn storage_base_url() -> String {
    std::env::var("OMNIGATE_STORAGE_BASE_URL")
        .or_else(|_| std::env::var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_STORAGE_BASE_URL.to_owned())
}

fn reject_control_chars(input: &str) -> Result<(), AssetParseError> {
    if input.chars().any(char::is_control) {
        return Err(AssetParseError::UnsafeControlCharacter);
    }
    Ok(())
}

fn canonicalize_hash(hash: &str) -> Result<String, AssetParseError> {
    if hash.len() != 64 {
        return Err(AssetParseError::InvalidHashLength);
    }

    if !hash
        .bytes()
        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(AssetParseError::InvalidHashCharacters);
    }

    Ok(hash.to_owned())
}

fn normalize_asset_kind(kind: &str) -> Result<String, AssetParseError> {
    let kind = kind.trim().to_ascii_lowercase();

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

    if ok {
        Ok(kind)
    } else {
        Err(AssetParseError::UnsupportedAssetKind)
    }
}

fn owner_from_value(value: &Value) -> Option<OwnerSummary> {
    let passport_subject = string_field(value, "passport_subject")?;
    let wallet_account = string_field(value, "wallet_account")?;
    Some(OwnerSummary {
        passport_subject,
        wallet_account,
    })
}

fn payout_from_value(value: &Value) -> Option<PayoutSummary> {
    let default_action = string_field(value, "default_action")?;
    let recipient_account = string_field(value, "recipient_account")?;
    Some(PayoutSummary {
        default_action,
        recipient_account,
    })
}

fn metadata_from_value(value: &Value) -> Option<ManifestMetadataSummary> {
    let object = value.as_object()?;
    let tags = object
        .get("tags")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default();

    Some(ManifestMetadataSummary {
        title: string_field(value, "title"),
        description: string_field(value, "description"),
        tags,
        license: string_field(value, "license"),
        content_type: string_field(value, "content_type"),
    })
}

fn receipt_from_value(value: &Value) -> Option<ReceiptSummary> {
    let tx_id = string_field(value, "tx_id")?;
    let receipt_kind = string_field(value, "receipt_kind")?;
    Some(ReceiptSummary {
        tx_id,
        receipt_kind,
        amount_minor_units: value.get("amount_minor_units").and_then(Value::as_u64),
        account: string_field(value, "account"),
        created_at_ms: value.get("created_at_ms").and_then(Value::as_u64),
    })
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
) -> Response {
    (
        status,
        Json(ResolverProblem {
            code,
            message,
            retryable,
            reason,
        }),
    )
        .into_response()
}
