//! RO:WHAT — WEB3_2 product routes for static site prepare/create/resolve.
//! RO:WHY — Batch 8/9 crab://site foundation: paid prepare, manifest storage, site pointer write, site hydration.
//! RO:INTERACTS — svc-storage `/paid/o/estimate` and `/o`; svc-index `/v1/index/sites/:name/manifest`.
//! RO:INVARIANTS — no wallet calls; no ledger mutation; no raw bundle storage here; storage/index remain source owners.
//! RO:METRICS — covered by omnigate HTTP middleware when mounted through `App::build`.
//! RO:CONFIG — `OMNIGATE_STORAGE_BASE_URL`/`OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL`;
//!              `OMNIGATE_INDEX_BASE_URL`/`OMNIGATE_DOWNSTREAM_INDEX_BASE_URL`.
//! RO:SECURITY — strict DTOs; unsafe site names reject; hop-by-hop headers filtered.
//! RO:TEST — `tests/site_launch.rs`; live smoke: `scripts/web3_product_stack_smoke.sh`.

use axum::{
    body::Bytes,
    extract::Path,
    http::{header, HeaderMap, HeaderName, Method, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DEFAULT_STORAGE_BASE_URL: &str = "http://127.0.0.1:15303";
const DEFAULT_INDEX_BASE_URL: &str = "http://127.0.0.1:5304";
const SITE_PREPARE_SCHEMA: &str = "omnigate.site-prepare.v1";
const SITE_CREATE_SCHEMA: &str = "omnigate.site-create.v1";
const SITE_PAGE_SCHEMA: &str = "omnigate.site-page.v1";
const DEFAULT_ACTION: &str = "paid_site_launch";
const DEFAULT_ASSET: &str = "roc";
const DEFAULT_CURRENCY: &str = "ROC";

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate sites route reqwest client should build")
});

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SitePrepareRequest {
    site_name: String,
    #[serde(default)]
    total_bytes: Option<u64>,
    #[serde(default)]
    files: Vec<SiteFileSpec>,
    #[serde(default)]
    payer_account: Option<String>,
    #[serde(default)]
    owner_passport_subject: Option<String>,
    #[serde(default)]
    owner_wallet_account: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteFileSpec {
    path: String,
    bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteCreateRequest {
    site_name: String,
    root_document_cid: String,
    #[serde(default)]
    owner_passport_subject: Option<String>,
    #[serde(default)]
    owner_wallet_account: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    route_map: BTreeMap<String, String>,
    #[serde(default)]
    asset_map: BTreeMap<String, String>,
    #[serde(default)]
    receipt_refs: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteManifestPointer {
    version: u16,
    name: String,
    manifest_cid: String,
    #[serde(default)]
    owner_passport_subject: Option<String>,
    #[serde(default)]
    owner_wallet_account: Option<String>,
    updated_at_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteManifestDocument {
    version: u16,
    site_name: String,
    root_document_cid: String,
    #[serde(default)]
    asset_map: BTreeMap<String, String>,
    #[serde(default)]
    route_map: BTreeMap<String, String>,
    #[serde(default)]
    owner: Option<SiteManifestOwner>,
    #[serde(default)]
    payout: Option<SiteManifestPayout>,
    #[serde(default)]
    metadata: Option<SiteManifestMetadata>,
    #[allow(dead_code)]
    #[serde(default)]
    provenance: Option<Value>,
    #[allow(dead_code)]
    #[serde(default)]
    storage: Option<Value>,
    #[serde(default)]
    receipts: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteManifestOwner {
    #[serde(default)]
    passport_subject: Option<String>,
    #[serde(default)]
    wallet_account: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteManifestPayout {
    #[serde(default)]
    default_action: Option<String>,
    #[serde(default)]
    recipient_account: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    splits: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteManifestMetadata {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SitePrepareResponse {
    schema: &'static str,
    site_name: String,
    action: String,
    asset: String,
    total_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_passport_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_wallet_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    file_count: usize,
    paid_storage: PaidStoragePrepareSummary,
    wallet_hold: WalletHoldTemplate,
    site_manifest_preview: SiteManifestPreview,
    next: SitePrepareNext,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PaidStoragePrepareSummary {
    estimate_path: &'static str,
    submit_path: &'static str,
    estimate: Value,
}

#[derive(Debug, Serialize)]
struct WalletHoldTemplate {
    required: bool,
    action: String,
    currency: &'static str,
    amount_minor: String,
    minimum_hold_minor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payer_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    idempotency_key_hint: Option<String>,
    capability: WalletCapabilityHint,
}

#[derive(Debug, Serialize)]
struct WalletCapabilityHint {
    required_action: &'static str,
    resource: &'static str,
    audience: &'static str,
    recommended_ttl_seconds: u64,
}

#[derive(Debug, Serialize)]
struct SiteManifestPreview {
    will_create_site_manifest: bool,
    will_index_site_pointer: bool,
    name_pointer_route: String,
    owner_source: &'static str,
    note: &'static str,
}

#[derive(Debug, Serialize)]
struct SitePrepareNext {
    create_hold: &'static str,
    submit_site: &'static str,
    resolve_after_launch: String,
    required_submit_fields: Vec<&'static str>,
    optional_submit_headers: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct SiteCreateResponse {
    schema: &'static str,
    site_name: String,
    root_document_cid: String,
    manifest: SiteManifestWriteSummary,
    index_pointer: SiteIndexPointerSummary,
    owner: SiteOwnerSummary,
    payout: SitePayoutSummary,
    links: SiteLinks,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SitePageResponse {
    schema: &'static str,
    site_name: String,
    root_document_cid: String,
    manifest: SiteHydratedManifestSummary,
    owner: SiteOwnerSummary,
    payout: SitePayoutSummary,
    metadata: SiteMetadataSummary,
    route_map: BTreeMap<String, String>,
    asset_map: BTreeMap<String, String>,
    receipts: Vec<Value>,
    links: SiteLinks,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SiteManifestWriteSummary {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest_cid: Option<String>,
    storage_path: &'static str,
}

#[derive(Debug, Serialize)]
struct SiteHydratedManifestSummary {
    status: &'static str,
    hydration_status: &'static str,
    manifest_cid: String,
    updated_at_ms: u64,
    manifest_raw: String,
}

#[derive(Debug, Serialize)]
struct SiteIndexPointerSummary {
    status: &'static str,
    route: String,
    http_status: Option<u16>,
}

#[derive(Debug, Serialize)]
struct SiteOwnerSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    passport_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wallet_account: Option<String>,
}

#[derive(Debug, Serialize)]
struct SitePayoutSummary {
    default_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipient_account: Option<String>,
}

#[derive(Debug, Serialize)]
struct SiteMetadataSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SiteLinks {
    crab: String,
    resolve: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest_raw: Option<String>,
}

#[derive(Debug, Serialize)]
struct Problem<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
    reason: &'a str,
}

#[derive(Debug, Serialize)]
struct StorageEstimateRejectedProblem {
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
    storage_status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_error: Option<Value>,
}

#[derive(Debug)]
struct UpstreamBody {
    status: StatusCode,
    body: Bytes,
}

/// Prepare a static site launch.
pub async fn site_prepare(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<SitePrepareRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_prepare_request",
                "site prepare request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    let site_name = match normalize_site_name(&request.site_name) {
        Ok(name) => name,
        Err(reason) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_name",
                "site_name is not a safe beta site name",
                false,
                reason,
            );
        }
    };

    if let Err(reason) = validate_site_files(&request.files) {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_site_files",
            "site files are not valid",
            false,
            reason,
        );
    }

    let total_bytes = match effective_total_bytes(request.total_bytes, &request.files) {
        Some(bytes) if bytes > 0 => bytes,
        _ => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_prepare_request",
                "total_bytes or files[].bytes must be greater than zero",
                false,
                "invalid_total_bytes",
            );
        }
    };

    let storage_estimate = match fetch_storage_estimate(total_bytes, headers).await {
        Ok(storage_estimate) => storage_estimate,
        Err(response) => return response,
    };

    let action =
        value_string(&storage_estimate, "action").unwrap_or_else(|| DEFAULT_ACTION.to_owned());
    let asset =
        value_string(&storage_estimate, "asset").unwrap_or_else(|| DEFAULT_ASSET.to_owned());

    let Some(amount_minor) = value_string(&storage_estimate, "amount_minor")
        .or_else(|| value_string(&storage_estimate, "amount_minor_units"))
        .or_else(|| value_string(&storage_estimate, "amount"))
    else {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_estimate_missing_amount",
            "storage estimate did not include an amount",
            true,
            "storage_estimate_missing_amount",
        );
    };

    let minimum_hold_minor = value_string(&storage_estimate, "minimum_hold_minor")
        .or_else(|| value_string(&storage_estimate, "minimum_hold_minor_units"))
        .unwrap_or_else(|| amount_minor.clone());

    let idempotency_key_hint = request
        .client_idempotency_key
        .clone()
        .or_else(|| Some(format!("prepare:site:{action}:{site_name}:{total_bytes}")));

    let response = SitePrepareResponse {
        schema: SITE_PREPARE_SCHEMA,
        site_name: site_name.clone(),
        action: action.clone(),
        asset,
        total_bytes,
        owner_passport_subject: request.owner_passport_subject,
        owner_wallet_account: request.owner_wallet_account,
        title: request.title,
        description: request.description,
        file_count: request.files.len(),
        paid_storage: PaidStoragePrepareSummary {
            estimate_path: "/v1/paid/o/prepare",
            submit_path: "/v1/sites",
            estimate: storage_estimate,
        },
        wallet_hold: WalletHoldTemplate {
            required: true,
            action,
            currency: DEFAULT_CURRENCY,
            amount_minor,
            minimum_hold_minor,
            payer_account: request.payer_account,
            idempotency_key_hint,
            capability: WalletCapabilityHint {
                required_action: "wallet.hold",
                resource: "paid_site_launch",
                audience: "svc-wallet",
                recommended_ttl_seconds: 300,
            },
        },
        site_manifest_preview: SiteManifestPreview {
            will_create_site_manifest: true,
            will_index_site_pointer: true,
            name_pointer_route: format!("/v1/index/sites/{site_name}/manifest"),
            owner_source: "request.owner_or_submit_body",
            note: "site manifest creation and index pointer write happen after site submit",
        },
        next: SitePrepareNext {
            create_hold: "/v1/wallet/hold",
            submit_site: "/v1/sites",
            resolve_after_launch: format!("/v1/sites/{site_name}"),
            required_submit_fields: vec!["site_name", "root_document_cid"],
            optional_submit_headers: vec![
                "Authorization",
                "Idempotency-Key",
                "x-ron-passport",
                "x-ron-wallet-account",
                "x-ron-wallet-hold-txid",
                "x-correlation-id",
                "x-request-id",
            ],
        },
        warnings: Vec::new(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Create a dev-mode static site manifest and write the site/name pointer.
pub async fn site_create(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<SiteCreateRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_create_request",
                "site create request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    let site_name = match normalize_site_name(&request.site_name) {
        Ok(name) => name,
        Err(reason) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_name",
                "site_name is not a safe beta site name",
                false,
                reason,
            );
        }
    };

    if !is_canonical_b3_cid(&request.root_document_cid) {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_root_document_cid",
            "root_document_cid must be canonical b3:<64 lowercase hex>",
            false,
            "invalid_root_document_cid",
        );
    }

    if let Err(reason) = validate_cid_map(&request.asset_map) {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_asset_map",
            "asset_map contains invalid entries",
            false,
            reason,
        );
    }

    if let Err(reason) = validate_cid_map(&request.route_map) {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_route_map",
            "route_map contains invalid entries",
            false,
            reason,
        );
    }

    let owner = SiteOwnerSummary {
        passport_subject: request.owner_passport_subject.clone(),
        wallet_account: request.owner_wallet_account.clone(),
    };

    let manifest = build_site_manifest(
        &site_name,
        &request,
        &owner,
        grab(&headers, "x-ron-wallet-hold-txid"),
    );
    let manifest_bytes = match serde_json::to_vec(&manifest) {
        Ok(bytes) => Bytes::from(bytes),
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "manifest_encode_failed",
                "failed to encode generated site manifest",
                false,
                "manifest_encode_failed",
            );
        }
    };

    let mut warnings = Vec::new();

    let manifest_write = match store_manifest_object(headers.clone(), manifest_bytes).await {
        Ok(upstream) if upstream.status.is_success() => {
            match serde_json::from_slice::<Value>(&upstream.body)
                .ok()
                .and_then(|value| value_string(&value, "cid"))
                .filter(|cid| is_canonical_b3_cid(cid))
            {
                Some(manifest_cid) => SiteManifestWriteSummary {
                    status: "stored",
                    manifest_cid: Some(manifest_cid),
                    storage_path: "/o",
                },
                None => {
                    warnings.push("site_manifest_storage_missing_valid_cid".to_owned());
                    SiteManifestWriteSummary {
                        status: "failed",
                        manifest_cid: None,
                        storage_path: "/o",
                    }
                }
            }
        }
        Ok(upstream) => {
            warnings.push(format!(
                "site_manifest_storage_http_{}",
                upstream.status.as_u16()
            ));
            SiteManifestWriteSummary {
                status: "failed",
                manifest_cid: None,
                storage_path: "/o",
            }
        }
        Err(response) => {
            warnings.push(response_warning(&response, "site_manifest_storage_failed"));
            SiteManifestWriteSummary {
                status: "failed",
                manifest_cid: None,
                storage_path: "/o",
            }
        }
    };

    let pointer_route = format!("/v1/index/sites/{site_name}/manifest");

    let index_pointer = if let Some(manifest_cid) = &manifest_write.manifest_cid {
        match put_site_pointer(&headers, &site_name, manifest_cid, &owner).await {
            Ok(upstream) if upstream.status.is_success() => SiteIndexPointerSummary {
                status: "stored",
                route: pointer_route,
                http_status: Some(upstream.status.as_u16()),
            },
            Ok(upstream) => {
                warnings.push(format!(
                    "site_index_pointer_http_{}",
                    upstream.status.as_u16()
                ));
                SiteIndexPointerSummary {
                    status: "failed",
                    route: pointer_route,
                    http_status: Some(upstream.status.as_u16()),
                }
            }
            Err(response) => {
                warnings.push(response_warning(&response, "site_index_pointer_failed"));
                SiteIndexPointerSummary {
                    status: "failed",
                    route: pointer_route,
                    http_status: None,
                }
            }
        }
    } else {
        warnings.push("site_index_pointer_skipped_missing_manifest_cid".to_owned());
        SiteIndexPointerSummary {
            status: "skipped",
            route: pointer_route,
            http_status: None,
        }
    };

    let manifest_raw = manifest_write
        .manifest_cid
        .as_ref()
        .map(|manifest_cid| format!("/o/{manifest_cid}"));

    let response = SiteCreateResponse {
        schema: SITE_CREATE_SCHEMA,
        site_name: site_name.clone(),
        root_document_cid: request.root_document_cid,
        manifest: manifest_write,
        index_pointer,
        owner: SiteOwnerSummary {
            passport_subject: owner.passport_subject.clone(),
            wallet_account: owner.wallet_account.clone(),
        },
        payout: SitePayoutSummary {
            default_action: "site_visit".to_owned(),
            recipient_account: owner.wallet_account,
        },
        links: SiteLinks {
            crab: format!("crab://{site_name}"),
            resolve: format!("/v1/sites/{site_name}"),
            manifest_raw,
        },
        warnings,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Resolve and hydrate a site page by name.
pub async fn site_resolve(Path(name): Path<String>, headers: HeaderMap) -> Response {
    let site_name = match normalize_site_name(&name) {
        Ok(name) => name,
        Err(reason) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_site_name",
                "site_name is not a safe beta site name",
                false,
                reason,
            );
        }
    };

    let pointer = match fetch_site_pointer(&site_name, &headers).await {
        Ok(Some(pointer)) => pointer,
        Ok(None) => {
            return problem(
                StatusCode::NOT_FOUND,
                "site_not_found",
                "site manifest pointer was not found",
                false,
                "site_not_found",
            );
        }
        Err(response) => return response,
    };

    if pointer.version != 1 {
        return problem(
            StatusCode::BAD_GATEWAY,
            "site_pointer_unsupported_version",
            "site pointer version is unsupported",
            true,
            "site_pointer_unsupported_version",
        );
    }

    if pointer.name != site_name {
        return problem(
            StatusCode::BAD_GATEWAY,
            "site_pointer_mismatch",
            "site pointer name did not match request",
            true,
            "site_pointer_mismatch",
        );
    }

    if !is_canonical_b3_cid(&pointer.manifest_cid) {
        return problem(
            StatusCode::BAD_GATEWAY,
            "site_pointer_invalid_manifest_cid",
            "site pointer returned invalid manifest CID",
            true,
            "site_pointer_invalid_manifest_cid",
        );
    }

    let manifest = match fetch_site_manifest(&pointer.manifest_cid, &headers).await {
        Ok(manifest) => manifest,
        Err(response) => return response,
    };

    if manifest.version != 1 {
        return problem(
            StatusCode::BAD_GATEWAY,
            "site_manifest_unsupported_version",
            "site manifest version is unsupported",
            true,
            "site_manifest_unsupported_version",
        );
    }

    if normalize_site_name(&manifest.site_name).ok().as_deref() != Some(site_name.as_str()) {
        return problem(
            StatusCode::BAD_GATEWAY,
            "site_manifest_name_mismatch",
            "site manifest name did not match request",
            true,
            "site_manifest_name_mismatch",
        );
    }

    if !is_canonical_b3_cid(&manifest.root_document_cid) {
        return problem(
            StatusCode::BAD_GATEWAY,
            "site_manifest_invalid_root_document_cid",
            "site manifest root document CID is invalid",
            true,
            "site_manifest_invalid_root_document_cid",
        );
    }

    let owner = SiteOwnerSummary {
        passport_subject: manifest
            .owner
            .as_ref()
            .and_then(|owner| owner.passport_subject.clone())
            .or(pointer.owner_passport_subject),
        wallet_account: manifest
            .owner
            .as_ref()
            .and_then(|owner| owner.wallet_account.clone())
            .or(pointer.owner_wallet_account),
    };

    let payout = SitePayoutSummary {
        default_action: manifest
            .payout
            .as_ref()
            .and_then(|payout| payout.default_action.clone())
            .unwrap_or_else(|| "site_visit".to_owned()),
        recipient_account: manifest
            .payout
            .as_ref()
            .and_then(|payout| payout.recipient_account.clone())
            .or_else(|| owner.wallet_account.clone()),
    };

    let metadata = SiteMetadataSummary {
        title: manifest
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.title.clone()),
        description: manifest
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.description.clone()),
        tags: manifest
            .metadata
            .as_ref()
            .map(|metadata| metadata.tags.clone())
            .unwrap_or_default(),
    };

    let manifest_raw = format!("/o/{}", pointer.manifest_cid);
    let response = SitePageResponse {
        schema: SITE_PAGE_SCHEMA,
        site_name: site_name.clone(),
        root_document_cid: manifest.root_document_cid,
        manifest: SiteHydratedManifestSummary {
            status: "present",
            hydration_status: "hydrated",
            manifest_cid: pointer.manifest_cid,
            updated_at_ms: pointer.updated_at_ms,
            manifest_raw: manifest_raw.clone(),
        },
        owner,
        payout,
        metadata,
        route_map: manifest.route_map,
        asset_map: manifest.asset_map,
        receipts: manifest.receipts,
        links: SiteLinks {
            crab: format!("crab://{site_name}"),
            resolve: format!("/v1/sites/{site_name}"),
            manifest_raw: Some(manifest_raw),
        },
        warnings: Vec::new(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn fetch_site_pointer(
    site_name: &str,
    headers: &HeaderMap,
) -> Result<Option<SiteManifestPointer>, Response> {
    let route = format!("/v1/index/sites/{site_name}/manifest");
    let index_base = index_base_url();
    let upstream_url = format!("{}{}", index_base.trim_end_matches('/'), route);

    let mut req_builder = HTTP_CLIENT.get(upstream_url);

    for (name, value) in headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "index site pointer upstream unavailable",
                true,
                "index_connect",
            ));
        }
    };

    if upstream_res.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !upstream_res.status().is_success() {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "index_site_pointer_rejected",
            "index rejected site pointer lookup",
            upstream_res.status().as_u16() >= 500,
            "index_site_pointer_rejected",
        ));
    }

    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "index site pointer upstream unavailable",
                true,
                "index_read",
            ));
        }
    };

    serde_json::from_slice::<SiteManifestPointer>(&body)
        .map(Some)
        .map_err(|_| {
            problem(
                StatusCode::BAD_GATEWAY,
                "index_site_pointer_bad_json",
                "index site pointer response was not valid JSON",
                true,
                "index_site_pointer_bad_json",
            )
        })
}

async fn fetch_site_manifest(
    manifest_cid: &str,
    headers: &HeaderMap,
) -> Result<SiteManifestDocument, Response> {
    let route = format!("/o/{manifest_cid}");
    let storage_base = storage_base_url();
    let upstream_url = format!("{}{}", storage_base.trim_end_matches('/'), route);

    let mut req_builder = HTTP_CLIENT.get(upstream_url);

    for (name, value) in headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "storage site manifest upstream unavailable",
                true,
                "storage_connect",
            ));
        }
    };

    if upstream_res.status() == StatusCode::NOT_FOUND {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "site_manifest_missing",
            "site manifest object was not found in storage",
            true,
            "site_manifest_missing",
        ));
    }

    if !upstream_res.status().is_success() {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "storage_site_manifest_rejected",
            "storage rejected site manifest fetch",
            upstream_res.status().as_u16() >= 500,
            "storage_site_manifest_rejected",
        ));
    }

    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "storage site manifest upstream unavailable",
                true,
                "storage_read",
            ));
        }
    };

    serde_json::from_slice::<SiteManifestDocument>(&body).map_err(|_| {
        problem(
            StatusCode::BAD_GATEWAY,
            "site_manifest_bad_json",
            "site manifest object was not valid JSON",
            true,
            "site_manifest_bad_json",
        )
    })
}

async fn fetch_storage_estimate(bytes: u64, headers: HeaderMap) -> Result<Value, Response> {
    let upstream_path = format!("/paid/o/estimate?bytes={bytes}");
    let storage_base = storage_base_url();
    let upstream_url = format!("{}{}", storage_base.trim_end_matches('/'), upstream_path);

    let mut req_builder = HTTP_CLIENT.get(&upstream_url);

    for (name, value) in &headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "storage estimate upstream unavailable",
                true,
                "storage_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let body_bytes = match upstream_res.bytes().await {
        Ok(body_bytes) => body_bytes,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "storage estimate upstream unavailable",
                true,
                "storage_read",
            ));
        }
    };

    let parsed = serde_json::from_slice::<Value>(&body_bytes).ok();

    if !status.is_success() {
        let storage_error = parsed.or_else(|| {
            Some(Value::String(
                String::from_utf8_lossy(&body_bytes).to_string(),
            ))
        });

        return Err((
            status,
            Json(StorageEstimateRejectedProblem {
                code: "storage_estimate_rejected",
                message: "storage estimate rejected site prepare request",
                retryable: status.as_u16() >= 500,
                reason: "storage_estimate_rejected",
                storage_status: status.as_u16(),
                storage_error,
            }),
        )
            .into_response());
    }

    let Some(parsed) = parsed else {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "storage_estimate_bad_json",
            "storage estimate response was not valid JSON",
            true,
            "storage_bad_json",
        ));
    };

    Ok(parsed)
}

async fn store_manifest_object(headers: HeaderMap, body: Bytes) -> Result<UpstreamBody, Response> {
    let mut manifest_headers = headers;
    manifest_headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );

    send_to_storage(
        Method::POST,
        "/o",
        manifest_headers,
        body,
        "storage site manifest object upstream unavailable",
    )
    .await
}

async fn put_site_pointer(
    headers: &HeaderMap,
    site_name: &str,
    manifest_cid: &str,
    owner: &SiteOwnerSummary,
) -> Result<UpstreamBody, Response> {
    let route = format!("/v1/index/sites/{site_name}/manifest");
    let index_base = index_base_url();
    let upstream_url = format!("{}{}", index_base.trim_end_matches('/'), route);

    let body = json!({
        "manifest_cid": manifest_cid,
        "owner_passport_subject": owner.passport_subject,
        "owner_wallet_account": owner.wallet_account,
        "updated_at_ms": now_ms(),
    });

    let body = match serde_json::to_vec(&body) {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "site_pointer_encode_failed",
                "failed to encode site manifest pointer",
                false,
                "site_pointer_encode_failed",
            ));
        }
    };

    let mut req_builder = HTTP_CLIENT
        .put(upstream_url)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body);

    for (name, value) in headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "index site pointer upstream unavailable",
                true,
                "index_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "index site pointer upstream unavailable",
                true,
                "index_read",
            ));
        }
    };

    Ok(UpstreamBody { status, body })
}

async fn send_to_storage(
    method: Method,
    upstream_path: &str,
    headers: HeaderMap,
    body: Bytes,
    unavailable_message: &'static str,
) -> Result<UpstreamBody, Response> {
    let storage_base = storage_base_url();
    let upstream_url = format!("{}{}", storage_base.trim_end_matches('/'), upstream_path);

    let reqwest_method = match reqwest::Method::from_bytes(method.as_str().as_bytes()) {
        Ok(method) => method,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                unavailable_message,
                true,
                "bad_method",
            ));
        }
    };

    let mut req_builder = HTTP_CLIENT.request(reqwest_method, &upstream_url);

    for (name, value) in &headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.body(body).send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                unavailable_message,
                true,
                "storage_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                unavailable_message,
                true,
                "storage_read",
            ));
        }
    };

    Ok(UpstreamBody { status, body })
}

fn build_site_manifest(
    site_name: &str,
    request: &SiteCreateRequest,
    owner: &SiteOwnerSummary,
    wallet_hold_txid: Option<String>,
) -> Value {
    let mut root = Map::new();

    root.insert("version".to_owned(), json!(1));
    root.insert("site_name".to_owned(), json!(site_name));
    root.insert(
        "root_document_cid".to_owned(),
        json!(request.root_document_cid),
    );
    root.insert("asset_map".to_owned(), json!(request.asset_map));
    root.insert("route_map".to_owned(), json!(request.route_map));

    if owner.passport_subject.is_some() && owner.wallet_account.is_some() {
        root.insert(
            "owner".to_owned(),
            json!({
                "passport_subject": owner.passport_subject,
                "wallet_account": owner.wallet_account,
            }),
        );
    }

    if owner.wallet_account.is_some() {
        root.insert(
            "payout".to_owned(),
            json!({
                "default_action": "site_visit",
                "recipient_account": owner.wallet_account,
                "splits": [
                    {
                        "role": "creator",
                        "account": owner.wallet_account,
                        "bps": 10_000
                    }
                ]
            }),
        );
    }

    root.insert(
        "metadata".to_owned(),
        json!({
            "title": request
                .title
                .clone()
                .unwrap_or_else(|| site_name.to_owned()),
            "description": request.description,
            "tags": ["site"],
        }),
    );

    root.insert(
        "provenance".to_owned(),
        json!({
            "created_at_ms": now_ms(),
            "source": "omnigate.site_create",
        }),
    );

    root.insert(
        "storage".to_owned(),
        json!({
            "root_document_cid": request.root_document_cid,
            "manifest_route": "/o/<site-manifest-cid>",
        }),
    );

    let mut receipts = request.receipt_refs.clone();
    if let Some(tx_id) = wallet_hold_txid {
        receipts.push(json!({
            "tx_id": tx_id,
            "receipt_kind": "paid_site_launch",
            "account": owner.wallet_account,
            "created_at_ms": now_ms(),
        }));
    }
    root.insert("receipts".to_owned(), json!(receipts));

    Value::Object(root)
}

fn effective_total_bytes(total_bytes: Option<u64>, files: &[SiteFileSpec]) -> Option<u64> {
    if let Some(total_bytes) = total_bytes {
        return Some(total_bytes);
    }

    let mut total = 0u64;
    for file in files {
        total = total.checked_add(file.bytes)?;
    }

    Some(total)
}

fn validate_site_files(files: &[SiteFileSpec]) -> Result<(), &'static str> {
    for file in files {
        if file.bytes == 0 {
            return Err("empty_file");
        }

        validate_site_file_path(&file.path)?;
    }

    Ok(())
}

fn validate_site_file_path(path: &str) -> Result<(), &'static str> {
    let trimmed = path.trim();

    if trimmed.is_empty() {
        return Err("empty_path");
    }

    if trimmed.len() > 256 {
        return Err("path_too_long");
    }

    if path.chars().any(char::is_control) {
        return Err("path_control_character");
    }

    if trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains('\\')
        || trimmed.contains("..")
    {
        return Err("unsafe_path");
    }

    Ok(())
}

fn validate_cid_map(map: &BTreeMap<String, String>) -> Result<(), &'static str> {
    for (key, cid) in map {
        if key.trim().is_empty() || key.chars().any(char::is_control) {
            return Err("invalid_map_key");
        }

        if !is_canonical_b3_cid(cid) {
            return Err("invalid_map_cid");
        }
    }

    Ok(())
}

fn normalize_site_name(input: &str) -> Result<String, &'static str> {
    if input.chars().any(char::is_control) {
        return Err("control_character");
    }

    let name = input.trim().to_ascii_lowercase();

    if name.is_empty() {
        return Err("empty_name");
    }

    if name == "." || name == ".." {
        return Err("dot_name");
    }

    if name.contains("..") {
        return Err("double_dot");
    }

    if name.starts_with('.') || name.ends_with('.') {
        return Err("edge_dot");
    }

    if name.contains('/') || name.contains('\\') || name.contains('@') || name.contains(' ') {
        return Err("unsafe_character");
    }

    if name.len() > 253 {
        return Err("name_too_long");
    }

    if !name
        .bytes()
        .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_'))
    {
        return Err("unsupported_character");
    }

    Ok(name)
}

fn storage_base_url() -> String {
    std::env::var("OMNIGATE_STORAGE_BASE_URL")
        .or_else(|_| std::env::var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_STORAGE_BASE_URL.to_owned())
}

fn index_base_url() -> String {
    std::env::var("OMNIGATE_INDEX_BASE_URL")
        .or_else(|_| std::env::var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_INDEX_BASE_URL.to_owned())
}

fn is_canonical_b3_cid(value: &str) -> bool {
    let Some(hash) = value.strip_prefix("b3:") else {
        return false;
    };

    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn should_forward_header(name: &HeaderName) -> bool {
    if is_hop_by_hop_or_host(name) || name == header::CONTENT_LENGTH {
        return false;
    }

    name == header::AUTHORIZATION
        || name == header::ACCEPT
        || name == header::CONTENT_TYPE
        || name.as_str().starts_with("x-ron-")
        || name.as_str() == "x-correlation-id"
        || name.as_str() == "x-request-id"
        || name.as_str() == "idempotency-key"
}

fn is_hop_by_hop_or_host(name: &HeaderName) -> bool {
    name == header::HOST
        || name == header::CONNECTION
        || name == header::PROXY_AUTHORIZATION
        || name == header::TE
        || name == header::TRAILER
        || name == header::TRANSFER_ENCODING
        || name == header::UPGRADE
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    match value.get(key)? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn now_ms() -> u64 {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 1;
    };

    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn response_warning(_response: &Response, fallback: &'static str) -> String {
    fallback.to_owned()
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
        Json(Problem {
            code,
            message,
            retryable,
            reason,
        }),
    )
        .into_response()
}
