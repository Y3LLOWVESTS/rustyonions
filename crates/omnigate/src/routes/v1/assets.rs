//! RO:WHAT — WEB3_2 product routes for image asset prepare/upload.
//! RO:WHY — First visible creator flow needs image-specific prepare DTOs and upload coordination.
//! RO:INTERACTS — svc-storage `/paid/o/estimate`, `/paid/o`, `/o`; svc-index asset manifest pointer routes.
//! RO:INVARIANTS — no wallet calls; no ledger mutation; no direct byte storage; storage/index remain source owners.
//! RO:METRICS — covered by omnigate HTTP middleware when mounted through `App::build`.
//! RO:CONFIG — `OMNIGATE_STORAGE_BASE_URL`/`OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL`;
//!              `OMNIGATE_INDEX_BASE_URL`/`OMNIGATE_DOWNSTREAM_INDEX_BASE_URL`.
//! RO:SECURITY — strict JSON prepare DTO; image content-type validation; hop-by-hop headers filtered.
//! RO:TEST — `tests/image_asset_prepare.rs`.

use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderName, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_STORAGE_BASE_URL: &str = "http://127.0.0.1:15303";
const DEFAULT_INDEX_BASE_URL: &str = "http://127.0.0.1:5304";
const IMAGE_PREPARE_SCHEMA: &str = "omnigate.image-asset-prepare.v1";
const IMAGE_UPLOAD_SCHEMA: &str = "omnigate.image-asset-upload.v1";
const DEFAULT_ACTION: &str = "paid_storage_put";
const DEFAULT_ASSET: &str = "roc";
const DEFAULT_CURRENCY: &str = "ROC";

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate assets route reqwest client should build")
});

/// Router for `/v1/assets/*`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/image/prepare", post(image_prepare))
        .route("/image", post(image_upload))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageAssetPrepareRequest {
    bytes: u64,
    #[serde(default)]
    payer_account: Option<String>,
    #[serde(default)]
    owner_passport_subject: Option<String>,
    #[serde(default)]
    content_type: Option<String>,
    #[serde(default)]
    expected_asset_cid: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct ImageAssetPrepareResponse {
    schema: &'static str,
    asset_kind: &'static str,
    action: String,
    asset: String,
    bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_asset_cid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_passport_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    tags: Vec<String>,
    paid_storage: PaidStoragePrepareSummary,
    wallet_hold: WalletHoldTemplate,
    manifest_preview: ImageManifestPreview,
    next: ImagePrepareNext,
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
struct ImageManifestPreview {
    will_create_manifest: bool,
    will_index_asset_pointer: bool,
    owner_source: &'static str,
    note: &'static str,
}

#[derive(Debug, Serialize)]
struct ImagePrepareNext {
    create_hold: &'static str,
    submit_upload: &'static str,
    resolve_after_upload: &'static str,
    required_upload_headers: Vec<&'static str>,
    optional_upload_headers: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct ImageAssetUploadResponse {
    schema: &'static str,
    asset_kind: &'static str,
    asset_cid: String,
    crab_url: String,
    storage_upload: Value,
    manifest: ManifestWriteSummary,
    index_pointer: IndexPointerSummary,
    owner: OwnerSummary,
    payout: PayoutSummary,
    links: ImageUploadLinks,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ManifestWriteSummary {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest_cid: Option<String>,
    storage_path: &'static str,
}

#[derive(Debug, Serialize)]
struct IndexPointerSummary {
    status: &'static str,
    route: String,
    http_status: Option<u16>,
}

#[derive(Debug, Serialize)]
struct OwnerSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    passport_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wallet_account: Option<String>,
}

#[derive(Debug, Serialize)]
struct PayoutSummary {
    default_action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipient_account: Option<String>,
}

#[derive(Debug, Serialize)]
struct ImageUploadLinks {
    raw: String,
    crab: String,
    http_b3: String,
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
    headers: HeaderMap,
    body: Bytes,
}

/// Prepare an image asset upload.
///
/// This is image-specific UX glue over paid storage estimate. It intentionally
/// does not create a wallet hold, write bytes, create manifests, index asset
/// pointers, or mutate ledger/accounting.
pub async fn image_prepare(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<ImageAssetPrepareRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_image_prepare_request",
                "image prepare request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    if request.bytes == 0 {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_image_prepare_request",
            "bytes must be greater than zero",
            false,
            "invalid_bytes",
        );
    }

    if let Some(content_type) = &request.content_type {
        if !is_valid_image_content_type(content_type) {
            return problem(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "invalid_image_content_type",
                "content_type must be an image/* media type",
                false,
                "invalid_content_type",
            );
        }
    }

    let storage_estimate = match fetch_storage_estimate(request.bytes, headers).await {
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
        .or_else(|| Some(format!("prepare:image:{action}:{}", request.bytes)));

    let response = ImageAssetPrepareResponse {
        schema: IMAGE_PREPARE_SCHEMA,
        asset_kind: "image",
        action: action.clone(),
        asset,
        bytes: request.bytes,
        content_type: request.content_type,
        expected_asset_cid: request.expected_asset_cid,
        owner_passport_subject: request.owner_passport_subject,
        title: request.title,
        description: request.description,
        tags: request.tags,
        paid_storage: PaidStoragePrepareSummary {
            estimate_path: "/v1/paid/o/prepare",
            submit_path: "/v1/paid/o",
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
                resource: "paid_storage_put",
                audience: "svc-wallet",
                recommended_ttl_seconds: 300,
            },
        },
        manifest_preview: ImageManifestPreview {
            will_create_manifest: true,
            will_index_asset_pointer: true,
            owner_source: "request.owner_passport_subject_or_upload_headers",
            note: "manifest creation and index pointer write happen after the paid image upload succeeds",
        },
        next: ImagePrepareNext {
            create_hold: "/v1/wallet/hold",
            submit_upload: "/v1/assets/image",
            resolve_after_upload: "/v1/crab/resolve?url=crab://<hash>.image",
            required_upload_headers: vec![
                "Authorization",
                "Idempotency-Key",
                "x-ron-wallet-hold-txid",
            ],
            optional_upload_headers: vec![
                "x-ron-passport",
                "x-ron-wallet-account",
                "x-ron-asset-title",
                "x-ron-asset-description",
                "x-ron-asset-tags",
                "x-ron-permission",
                "x-ron-spend-limit",
                "x-correlation-id",
                "x-request-id",
            ],
        },
        warnings: Vec::new(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Upload image bytes through paid storage, then write an asset manifest object
/// and `svc-index` asset→manifest pointer.
///
/// Storage remains the paid-write enforcement owner. Index remains the pointer
/// owner. Omnigate only coordinates the product-facing flow after storage has
/// accepted the paid write.
pub async fn image_upload(headers: HeaderMap, body: Bytes) -> Response {
    let storage_upload = match send_to_storage(
        Method::POST,
        "/paid/o",
        headers.clone(),
        body,
        "storage paid image upload upstream unavailable",
    )
    .await
    {
        Ok(storage_upload) => storage_upload,
        Err(response) => return response,
    };

    if !storage_upload.status.is_success() {
        return response_from_upstream(storage_upload);
    }

    let storage_upload_json = match serde_json::from_slice::<Value>(&storage_upload.body) {
        Ok(value) => value,
        Err(_) => {
            return problem(
                StatusCode::BAD_GATEWAY,
                "storage_upload_bad_json",
                "storage paid image upload response was not valid JSON",
                true,
                "storage_bad_json",
            );
        }
    };

    let Some(asset_cid) = value_string(&storage_upload_json, "cid") else {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_missing_cid",
            "storage paid image upload response did not include cid",
            true,
            "storage_missing_cid",
        );
    };

    if !is_canonical_b3_cid(&asset_cid) {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_invalid_cid",
            "storage paid image upload response included invalid cid",
            true,
            "storage_invalid_cid",
        );
    }

    let owner = owner_from_headers(&headers);
    let payout = PayoutSummary {
        default_action: "content_view",
        recipient_account: owner.wallet_account.clone(),
    };

    let manifest = build_image_manifest(&headers, &storage_upload_json, &asset_cid, &owner);
    let manifest_bytes = match serde_json::to_vec(&manifest) {
        Ok(bytes) => Bytes::from(bytes),
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "manifest_encode_failed",
                "failed to encode generated image manifest",
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
                Some(manifest_cid) => ManifestWriteSummary {
                    status: "stored",
                    manifest_cid: Some(manifest_cid),
                    storage_path: "/o",
                },
                None => {
                    warnings.push("manifest_storage_missing_valid_cid".to_owned());
                    ManifestWriteSummary {
                        status: "failed",
                        manifest_cid: None,
                        storage_path: "/o",
                    }
                }
            }
        }
        Ok(upstream) => {
            warnings.push(format!(
                "manifest_storage_http_{}",
                upstream.status.as_u16()
            ));
            ManifestWriteSummary {
                status: "failed",
                manifest_cid: None,
                storage_path: "/o",
            }
        }
        Err(response) => {
            warnings.push(response_warning(&response, "manifest_storage_failed"));
            ManifestWriteSummary {
                status: "failed",
                manifest_cid: None,
                storage_path: "/o",
            }
        }
    };

    let pointer_route = format!(
        "/v1/index/assets/{}/manifest",
        asset_cid.trim_start_matches("b3:")
    );

    let index_pointer = if let Some(manifest_cid) = &manifest_write.manifest_cid {
        match put_index_pointer(&headers, &asset_cid, manifest_cid, &owner).await {
            Ok(upstream) if upstream.status.is_success() => IndexPointerSummary {
                status: "stored",
                route: pointer_route,
                http_status: Some(upstream.status.as_u16()),
            },
            Ok(upstream) => {
                warnings.push(format!("index_pointer_http_{}", upstream.status.as_u16()));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route,
                    http_status: Some(upstream.status.as_u16()),
                }
            }
            Err(response) => {
                warnings.push(response_warning(&response, "index_pointer_failed"));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route,
                    http_status: None,
                }
            }
        }
    } else {
        warnings.push("index_pointer_skipped_missing_manifest_cid".to_owned());
        IndexPointerSummary {
            status: "skipped",
            route: pointer_route,
            http_status: None,
        }
    };

    let raw_hash = asset_cid.trim_start_matches("b3:");
    let crab_url = format!("crab://{raw_hash}.image");

    let manifest_raw = manifest_write
        .manifest_cid
        .as_ref()
        .map(|manifest_cid| format!("/o/{manifest_cid}"));

    let response = ImageAssetUploadResponse {
        schema: IMAGE_UPLOAD_SCHEMA,
        asset_kind: "image",
        asset_cid: asset_cid.clone(),
        crab_url: crab_url.clone(),
        storage_upload: storage_upload_json,
        manifest: manifest_write,
        index_pointer,
        owner,
        payout,
        links: ImageUploadLinks {
            raw: format!("/o/{asset_cid}"),
            crab: crab_url.clone(),
            http_b3: format!("/v1/b3/{raw_hash}.image"),
            resolve: format!("/v1/crab/resolve?url={crab_url}"),
            manifest_raw,
        },
        warnings,
    };

    (StatusCode::OK, Json(response)).into_response()
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
                message: "storage estimate rejected image prepare request",
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
        "storage manifest object upstream unavailable",
    )
    .await
}

async fn put_index_pointer(
    headers: &HeaderMap,
    asset_cid: &str,
    manifest_cid: &str,
    owner: &OwnerSummary,
) -> Result<UpstreamBody, Response> {
    let raw_hash = asset_cid.trim_start_matches("b3:");
    let route = format!("/v1/index/assets/{raw_hash}/manifest");
    let index_base = index_base_url();
    let upstream_url = format!("{}{}", index_base.trim_end_matches('/'), route);

    let body = json!({
        "asset_kind": "image",
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
                "index_pointer_encode_failed",
                "failed to encode image manifest pointer",
                false,
                "index_pointer_encode_failed",
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
                "index manifest pointer upstream unavailable",
                true,
                "index_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let headers = upstream_res.headers().clone();
    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "index manifest pointer upstream unavailable",
                true,
                "index_read",
            ));
        }
    };

    Ok(UpstreamBody {
        status,
        headers,
        body,
    })
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
    let headers = upstream_res.headers().clone();

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

    Ok(UpstreamBody {
        status,
        headers,
        body,
    })
}

fn response_from_upstream(upstream: UpstreamBody) -> Response {
    let mut response = Response::new(Body::from(upstream.body));
    *response.status_mut() = upstream.status;

    for (name, value) in &upstream.headers {
        if should_copy_response_header(name) {
            response.headers_mut().insert(name.clone(), value.clone());
        }
    }

    response
}

fn response_warning(_response: &Response, fallback: &'static str) -> String {
    fallback.to_owned()
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

fn build_image_manifest(
    headers: &HeaderMap,
    storage_upload: &Value,
    asset_cid: &str,
    owner: &OwnerSummary,
) -> Value {
    let title = grab(headers, "x-ron-asset-title").unwrap_or_else(|| "Untitled image".to_owned());
    let description = grab(headers, "x-ron-asset-description");
    let tags = tags_from_headers(headers);
    let content_type = grab(headers, "content-type");

    let mut root = Map::new();
    root.insert("version".to_owned(), json!(1));
    root.insert("asset_cid".to_owned(), json!(asset_cid));
    root.insert("asset_kind".to_owned(), json!("image"));

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
                "default_action": "content_view",
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
            "title": title,
            "description": description,
            "tags": tags,
            "license": grab(headers, "x-ron-asset-license"),
            "content_type": content_type,
        }),
    );

    root.insert(
        "provenance".to_owned(),
        json!({
            "created_at_ms": now_ms(),
            "source": "omnigate.image_upload",
            "parent_cids": [],
        }),
    );

    root.insert(
        "storage".to_owned(),
        json!({
            "available": true,
            "content_type": grab(headers, "content-type"),
            "raw_url": format!("/o/{asset_cid}"),
            "paid": true,
        }),
    );

    let receipts = receipt_refs_from_storage_upload(storage_upload, owner);
    root.insert("receipts".to_owned(), json!(receipts));

    Value::Object(root)
}

fn receipt_refs_from_storage_upload(storage_upload: &Value, owner: &OwnerSummary) -> Vec<Value> {
    let Some(tx_id) = value_string(storage_upload, "wallet_txid")
        .or_else(|| value_string(storage_upload, "tx_id"))
        .or_else(|| value_string(storage_upload, "wallet_hold_txid"))
    else {
        return Vec::new();
    };

    vec![json!({
        "tx_id": tx_id,
        "receipt_kind": "paid_storage",
        "amount_minor_units": value_string(storage_upload, "estimate_minor")
            .and_then(|value| value.parse::<u64>().ok()),
        "account": owner.wallet_account,
        "created_at_ms": now_ms(),
    })]
}

fn owner_from_headers(headers: &HeaderMap) -> OwnerSummary {
    OwnerSummary {
        passport_subject: grab(headers, "x-ron-passport"),
        wallet_account: grab(headers, "x-ron-wallet-account"),
    }
}

fn tags_from_headers(headers: &HeaderMap) -> Vec<String> {
    grab(headers, "x-ron-asset-tags")
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .take(32)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn is_valid_image_content_type(content_type: &str) -> bool {
    let trimmed = content_type.trim().to_ascii_lowercase();
    !trimmed.is_empty() && !trimmed.chars().any(char::is_control) && trimmed.starts_with("image/")
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

fn should_copy_response_header(name: &HeaderName) -> bool {
    name != header::TRANSFER_ENCODING
        && name != header::CONTENT_LENGTH
        && name != header::CONNECTION
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
