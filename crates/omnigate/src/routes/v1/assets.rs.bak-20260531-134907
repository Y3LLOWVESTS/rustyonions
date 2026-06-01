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
    extract::DefaultBodyLimit,
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
const VIDEO_PREPARE_SCHEMA: &str = "omnigate.video-asset-prepare.v1";
const VIDEO_UPLOAD_SCHEMA: &str = "omnigate.video-asset-upload.v1";
const MUSIC_PREPARE_SCHEMA: &str = "omnigate.music-asset-prepare.v1";
const MUSIC_UPLOAD_SCHEMA: &str = "omnigate.music-asset-upload.v1";
const PODCAST_PREPARE_SCHEMA: &str = "omnigate.podcast-asset-prepare.v1";
const PODCAST_UPLOAD_SCHEMA: &str = "omnigate.podcast-asset-upload.v1";
const STREAM_PREPARE_SCHEMA: &str = "omnigate.stream-asset-prepare.v1";
const STREAM_PUBLISH_SCHEMA: &str = "omnigate.stream-asset-publish.v1";
const DEFAULT_ACTION: &str = "paid_storage_put";
const DEFAULT_ASSET: &str = "roc";
const DEFAULT_CURRENCY: &str = "ROC";

/// HTTP body cap for image/media asset upload routes.
/// OAP frame caps remain separate and stay at 1 MiB.
const IMAGE_UPLOAD_BODY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const IMAGE_RENDITION_GROUP_HEADER_LIMIT_BYTES: usize = 12 * 1024;

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
        .route("/video/prepare", post(video_prepare))
        .route("/video", post(video_upload))
        .route("/music/prepare", post(music_prepare))
        .route("/music", post(music_upload))
        .route("/podcast/prepare", post(podcast_prepare))
        .route("/podcast", post(podcast_upload))
        .route("/stream/prepare", post(stream_prepare))
        .route("/stream", post(stream_publish))
        .layer(DefaultBodyLimit::max(IMAGE_UPLOAD_BODY_LIMIT_BYTES))
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoAssetPrepareRequest {
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
    video_kind: Option<String>,
    #[serde(default)]
    duration: Option<String>,
    #[serde(default)]
    resolution: Option<String>,
    #[serde(default)]
    aspect_ratio: Option<String>,
    #[serde(default)]
    codec_format: Option<String>,
    #[serde(default)]
    frame_rate: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MusicAssetPrepareRequest {
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
    artist_display: Option<String>,
    #[serde(default)]
    album_title: Option<String>,
    #[serde(default)]
    release_type: Option<String>,
    #[serde(default)]
    duration: Option<String>,
    #[serde(default)]
    genre: Option<String>,
    #[serde(default)]
    mood: Option<String>,
    #[serde(default)]
    bpm: Option<String>,
    #[serde(default)]
    key_signature: Option<String>,
    #[serde(default)]
    explicit_rating: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    cover_image_crab_url: Option<String>,
    #[serde(default)]
    lyrics_crab_url: Option<String>,
    #[serde(default)]
    rights_mode: Option<String>,
    #[serde(default)]
    license_mode: Option<String>,
    #[serde(default)]
    legal_attestation_accepted: bool,
    #[serde(default)]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PodcastAssetPrepareRequest {
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
    file_name: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    show_title: Option<String>,
    #[serde(default)]
    host_display: Option<String>,
    #[serde(default)]
    guest_display: Option<String>,
    #[serde(default)]
    season_number: Option<String>,
    #[serde(default)]
    episode_number: Option<String>,
    #[serde(default)]
    duration: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    explicit_rating: Option<String>,
    #[serde(default)]
    cover_image_crab_url: Option<String>,
    #[serde(default)]
    transcript_crab_url: Option<String>,
    #[serde(default)]
    chapters_crab_url: Option<String>,
    #[serde(default)]
    show_page_crab_url: Option<String>,
    #[serde(default)]
    rights_mode: Option<String>,
    #[serde(default)]
    license_mode: Option<String>,
    #[serde(default)]
    guest_permission_attested: bool,
    #[serde(default)]
    legal_attestation_accepted: bool,
    #[serde(default)]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StreamAssetRequest {
    #[serde(default)]
    schema: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    stream_kind: Option<String>,
    #[serde(default)]
    status_hint: Option<String>,
    #[serde(default)]
    creator: Value,
    #[serde(default)]
    source: Value,
    #[serde(default)]
    access_policy: Value,
    #[serde(default)]
    linked_assets: Value,
    #[serde(default)]
    chat: Value,
    #[serde(default)]
    moderation: Value,
    #[serde(default)]
    rights: Value,
    #[serde(default)]
    payout: Value,
    #[serde(default)]
    live_delivery: Value,
    #[serde(default)]
    local_manifest_preview: Option<Value>,
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

    let storage_estimate = match fetch_storage_estimate(
        request.bytes,
        headers,
        "storage estimate rejected image prepare request",
    )
    .await
    {
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
pub async fn image_upload(headers: HeaderMap, body: Body) -> Response {
    let body = match axum::body::to_bytes(body, IMAGE_UPLOAD_BODY_LIMIT_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return problem(
                StatusCode::PAYLOAD_TOO_LARGE,
                "image_upload_body_too_large",
                "image upload body exceeded the configured image upload cap",
                false,
                "image_upload_body_too_large",
            );
        }
    };

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

/// Prepare a bounded video-lite asset upload.
///
/// This mirrors image prepare but validates `video/*` and keeps the route honest:
/// it quotes paid storage only. It does not create a wallet hold, upload bytes,
/// transcode, stream, create manifests, index pointers, or mutate ledger truth.
pub async fn video_prepare(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<VideoAssetPrepareRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_video_prepare_request",
                "video prepare request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    if request.bytes == 0 {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_video_prepare_request",
            "bytes must be greater than zero",
            false,
            "invalid_bytes",
        );
    }

    if let Some(content_type) = &request.content_type {
        if !is_valid_video_content_type(content_type) {
            return problem(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "invalid_video_content_type",
                "content_type must be a video/* media type",
                false,
                "invalid_content_type",
            );
        }
    }

    let storage_estimate = match fetch_storage_estimate(
        request.bytes,
        headers,
        "storage estimate rejected asset prepare request",
    )
    .await
    {
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
        .or_else(|| Some(format!("prepare:video:{action}:{}", request.bytes)));

    let response = json!({
        "schema": VIDEO_PREPARE_SCHEMA,
        "asset_kind": "video",
        "action": action,
        "asset": asset,
        "bytes": request.bytes,
        "content_type": request.content_type,
        "expected_asset_cid": request.expected_asset_cid,
        "owner_passport_subject": request.owner_passport_subject,
        "title": request.title,
        "description": request.description,
        "tags": request.tags,
        "video": {
            "video_kind": request.video_kind,
            "duration": request.duration,
            "resolution": request.resolution,
            "aspect_ratio": request.aspect_ratio,
            "codec_format": request.codec_format,
            "frame_rate": request.frame_rate,
            "language": request.language,
        },
        "paid_storage": {
            "estimate_path": "/v1/paid/o/prepare",
            "submit_path": "/v1/paid/o",
            "estimate": storage_estimate,
        },
        "wallet_hold": {
            "required": true,
            "action": DEFAULT_ACTION,
            "currency": DEFAULT_CURRENCY,
            "amount_minor": amount_minor,
            "minimum_hold_minor": minimum_hold_minor,
            "payer_account": request.payer_account,
            "idempotency_key_hint": idempotency_key_hint,
            "capability": {
                "required_action": "wallet.hold",
                "resource": "paid_storage_put",
                "audience": "svc-wallet",
                "recommended_ttl_seconds": 300,
            }
        },
        "manifest_preview": {
            "will_create_manifest": true,
            "will_index_asset_pointer": true,
            "owner_source": "request.owner_passport_subject_or_upload_headers",
            "note": "video-lite manifest creation and index pointer write happen after the paid video upload succeeds",
        },
        "next": {
            "create_hold": "/v1/wallet/hold",
            "submit_upload": "/v1/assets/video",
            "resolve_after_upload": "/v1/crab/resolve?url=crab://<hash>.video",
            "required_upload_headers": [
                "Authorization",
                "Idempotency-Key",
                "x-ron-paid-op",
                "x-ron-paid-asset",
                "x-ron-paid-estimate-minor",
                "x-ron-wallet-txid",
                "x-ron-wallet-receipt-hash",
                "x-ron-wallet-from",
                "x-ron-wallet-to"
            ],
            "optional_upload_headers": [
                "x-ron-passport",
                "x-ron-wallet-account",
                "x-ron-asset-title",
                "x-ron-asset-description",
                "x-ron-asset-tags",
                "x-ron-video-duration",
                "x-ron-video-resolution",
                "x-ron-video-aspect-ratio",
                "x-ron-video-kind",
                "x-ron-video-language",
                "x-ron-permission",
                "x-ron-spend-limit",
                "x-correlation-id",
                "x-request-id"
            ]
        },
        "warnings": [
            "video_lite_only_no_transcoding_no_range_streaming"
        ],
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Upload bounded video bytes through paid storage, then write a video manifest
/// object and `svc-index` asset→manifest pointer.
///
/// This is intentionally video-lite. Storage enforces the paid write. Index owns
/// the pointer. Omnigate only coordinates after storage accepts the paid upload.
pub async fn video_upload(headers: HeaderMap, body: Body) -> Response {
    let body = match axum::body::to_bytes(body, IMAGE_UPLOAD_BODY_LIMIT_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return problem(
                StatusCode::PAYLOAD_TOO_LARGE,
                "video_upload_body_too_large",
                "video upload body exceeded the configured media upload cap",
                false,
                "video_upload_body_too_large",
            );
        }
    };

    let storage_upload = match send_to_storage(
        Method::POST,
        "/paid/o",
        headers.clone(),
        body,
        "storage paid video upload upstream unavailable",
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
                "storage paid video upload response was not valid JSON",
                true,
                "storage_bad_json",
            );
        }
    };

    let Some(asset_cid) = value_string(&storage_upload_json, "cid") else {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_missing_cid",
            "storage paid video upload response did not include cid",
            true,
            "storage_missing_cid",
        );
    };

    if !is_canonical_b3_cid(&asset_cid) {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_invalid_cid",
            "storage paid video upload response included invalid cid",
            true,
            "storage_invalid_cid",
        );
    }

    let owner = owner_from_headers(&headers);
    let payout = PayoutSummary {
        default_action: "content_view",
        recipient_account: owner.wallet_account.clone(),
    };

    let manifest = build_video_manifest(&headers, &storage_upload_json, &asset_cid, &owner);
    let manifest_bytes = match serde_json::to_vec(&manifest) {
        Ok(bytes) => Bytes::from(bytes),
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "manifest_encode_failed",
                "failed to encode generated video manifest",
                false,
                "manifest_encode_failed",
            );
        }
    };

    let mut warnings = Vec::new();
    warnings.push("video_lite_only_no_transcoding_no_range_streaming".to_owned());

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

    let raw_hash = asset_cid.trim_start_matches("b3:").to_owned();
    let pointer_route = format!("/v1/index/assets/{raw_hash}/manifest");

    let index_pointer = if let Some(manifest_cid) = &manifest_write.manifest_cid {
        match put_video_index_pointer(&headers, &asset_cid, manifest_cid, &owner).await {
            Ok(upstream) if upstream.status.is_success() => IndexPointerSummary {
                status: "stored",
                route: pointer_route.clone(),
                http_status: Some(upstream.status.as_u16()),
            },
            Ok(upstream) => {
                warnings.push(format!("index_pointer_http_{}", upstream.status.as_u16()));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route.clone(),
                    http_status: Some(upstream.status.as_u16()),
                }
            }
            Err(response) => {
                warnings.push(response_warning(&response, "index_pointer_failed"));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route.clone(),
                    http_status: None,
                }
            }
        }
    } else {
        warnings.push("index_pointer_skipped_missing_manifest_cid".to_owned());
        IndexPointerSummary {
            status: "skipped",
            route: pointer_route.clone(),
            http_status: None,
        }
    };

    let crab_url = format!("crab://{raw_hash}.video");
    let manifest_raw = manifest_write
        .manifest_cid
        .as_ref()
        .map(|manifest_cid| format!("/o/{manifest_cid}"));

    let response = json!({
        "schema": VIDEO_UPLOAD_SCHEMA,
        "asset_kind": "video",
        "asset_cid": asset_cid,
        "crab_url": crab_url,
        "storage_upload": storage_upload_json,
        "manifest": {
            "status": manifest_write.status,
            "manifest_cid": manifest_write.manifest_cid,
            "storage_path": manifest_write.storage_path,
        },
        "index_pointer": index_pointer,
        "owner": owner,
        "payout": payout,
        "links": {
            "raw": format!("/o/b3:{raw_hash}"),
            "crab": crab_url,
            "http_b3": format!("/v1/b3/{raw_hash}.video"),
            "resolve": format!("/v1/crab/resolve?url=crab://{raw_hash}.video"),
            "manifest_raw": manifest_raw,
        },
        "warnings": warnings,
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Prepare a music/audio asset upload.
///
/// This is music-specific UX glue over paid storage estimate. It intentionally
/// does not create a wallet hold, store bytes, mint receipts, verify legal
/// ownership, or mutate ledger/accounting.
pub async fn music_prepare(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<MusicAssetPrepareRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_music_prepare_request",
                "music prepare request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    if request.bytes == 0 {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_music_prepare_request",
            "bytes must be greater than zero",
            false,
            "invalid_bytes",
        );
    }

    if let Some(content_type) = &request.content_type {
        if !is_valid_audio_content_type(content_type) {
            return problem(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "invalid_music_content_type",
                "content_type must be an audio/* media type",
                false,
                "invalid_content_type",
            );
        }
    }

    if !request.legal_attestation_accepted {
        return problem(
            StatusCode::BAD_REQUEST,
            "music_rights_attestation_required",
            "music prepare requires creator rights attestation",
            false,
            "missing_music_rights_attestation",
        );
    }

    let storage_estimate = match fetch_storage_estimate(
        request.bytes,
        headers,
        "storage estimate rejected asset prepare request",
    )
    .await
    {
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
        .or_else(|| Some(format!("prepare:music:{action}:{}", request.bytes)));

    let response = json!({
        "schema": MUSIC_PREPARE_SCHEMA,
        "asset_kind": "music",
        "action": action,
        "asset": asset,
        "bytes": request.bytes,
        "content_type": request.content_type,
        "expected_asset_cid": request.expected_asset_cid,
        "owner_passport_subject": request.owner_passport_subject,
        "title": request.title,
        "description": request.description,
        "tags": request.tags,
        "music": {
            "artist_display": request.artist_display,
            "album_title": request.album_title,
            "release_type": request.release_type,
            "duration": request.duration,
            "genre": request.genre,
            "mood": request.mood,
            "bpm": request.bpm,
            "key_signature": request.key_signature,
            "explicit_rating": request.explicit_rating,
            "language": request.language,
            "cover_image_crab_url": request.cover_image_crab_url,
            "cover_image_upload_from_music_page": false,
            "lyrics_crab_url": request.lyrics_crab_url,
            "lyrics_are_separate_asset": true,
            "rights_mode": request.rights_mode,
            "license_mode": request.license_mode,
            "legal_attestation_accepted": request.legal_attestation_accepted,
        },
        "paid_storage": {
            "estimate_path": "/v1/paid/o/prepare",
            "submit_path": "/v1/paid/o",
            "estimate": storage_estimate,
        },
        "wallet_hold": {
            "required": true,
            "action": DEFAULT_ACTION,
            "currency": DEFAULT_CURRENCY,
            "amount_minor": amount_minor,
            "minimum_hold_minor": minimum_hold_minor,
            "payer_account": request.payer_account,
            "idempotency_key_hint": idempotency_key_hint,
            "capability": {
                "required_action": "wallet.hold",
                "resource": "paid_storage_put",
                "audience": "svc-wallet",
                "recommended_ttl_seconds": 300,
            }
        },
        "manifest_preview": {
            "will_create_manifest": true,
            "will_index_asset_pointer": true,
            "owner_source": "request.owner_passport_subject_or_upload_headers",
            "note": "music-lite manifest creation and index pointer write happen after the paid audio upload succeeds",
        },
        "next": {
            "create_hold": "/v1/wallet/hold",
            "submit_upload": "/v1/assets/music",
            "resolve_after_upload": "/v1/crab/resolve?url=crab://<hash>.music",
            "required_upload_headers": [
                "Authorization",
                "Idempotency-Key",
                "x-ron-paid-op",
                "x-ron-paid-asset",
                "x-ron-paid-estimate-minor",
                "x-ron-wallet-txid",
                "x-ron-wallet-receipt-hash",
                "x-ron-wallet-from",
                "x-ron-wallet-to",
                "x-ron-music-rights-attested"
            ],
            "optional_upload_headers": [
                "x-ron-passport",
                "x-ron-wallet-account",
                "x-ron-asset-title",
                "x-ron-asset-description",
                "x-ron-asset-tags",
                "x-ron-music-artist",
                "x-ron-music-album",
                "x-ron-music-release-type",
                "x-ron-music-duration",
                "x-ron-music-genre",
                "x-ron-music-language",
                "x-ron-cover-image-crab-url",
                "x-ron-lyrics-crab-url",
                "x-ron-music-bpm",
                "x-ron-music-key",
                "x-ron-music-explicit-rating",
                "x-ron-permission",
                "x-ron-spend-limit",
                "x-correlation-id",
                "x-request-id"
            ]
        },
        "warnings": [
            "music_lite_only_no_transcoding_no_drm_no_cover_art_upload",
            "legal_attestation_is_creator_confirmation_not_backend_ownership_proof"
        ],
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Upload bounded audio bytes through paid storage, then write a music manifest
/// object and `svc-index` asset→manifest pointer.
///
/// This is intentionally music-lite. Storage enforces the paid write. Index owns
/// the pointer. Cover art and lyrics are references only and are never uploaded
/// by this route.
pub async fn music_upload(headers: HeaderMap, body: Body) -> Response {
    let body = match axum::body::to_bytes(body, IMAGE_UPLOAD_BODY_LIMIT_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return problem(
                StatusCode::PAYLOAD_TOO_LARGE,
                "music_upload_body_too_large",
                "music upload body exceeded the configured media upload cap",
                false,
                "music_upload_body_too_large",
            );
        }
    };

    let Some(content_type) = grab(&headers, "content-type") else {
        return problem(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "missing_music_content_type",
            "music upload requires an audio/* content type",
            false,
            "missing_content_type",
        );
    };

    if !is_valid_audio_content_type(&content_type) {
        return problem(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "invalid_music_content_type",
            "music upload content-type must be audio/*",
            false,
            "invalid_content_type",
        );
    }

    if !truthy_header(&headers, "x-ron-music-rights-attested") {
        return problem(
            StatusCode::BAD_REQUEST,
            "music_rights_attestation_required",
            "music upload requires x-ron-music-rights-attested=true",
            false,
            "missing_music_rights_attestation",
        );
    }

    let storage_upload = match send_to_storage(
        Method::POST,
        "/paid/o",
        headers.clone(),
        body,
        "storage paid music upload upstream unavailable",
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
                "storage paid music upload response was not valid JSON",
                true,
                "storage_bad_json",
            );
        }
    };

    let Some(asset_cid) = value_string(&storage_upload_json, "cid") else {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_missing_cid",
            "storage paid music upload response did not include cid",
            true,
            "storage_missing_cid",
        );
    };

    if !is_canonical_b3_cid(&asset_cid) {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_invalid_cid",
            "storage paid music upload response included invalid cid",
            true,
            "storage_invalid_cid",
        );
    }

    let owner = owner_from_headers(&headers);
    let payout = PayoutSummary {
        default_action: "content_view",
        recipient_account: owner.wallet_account.clone(),
    };

    let manifest = build_music_manifest(&headers, &storage_upload_json, &asset_cid, &owner);
    let manifest_bytes = match serde_json::to_vec(&manifest) {
        Ok(bytes) => Bytes::from(bytes),
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "manifest_encode_failed",
                "failed to encode generated music manifest",
                false,
                "manifest_encode_failed",
            );
        }
    };

    let mut warnings = Vec::new();
    warnings.push("music_lite_only_no_transcoding_no_drm".to_owned());
    warnings.push("cover_art_reference_only_no_music_page_image_upload".to_owned());
    warnings
        .push("legal_attestation_is_creator_confirmation_not_backend_ownership_proof".to_owned());

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

    let raw_hash = asset_cid.trim_start_matches("b3:").to_owned();
    let pointer_route = format!("/v1/index/assets/{raw_hash}/manifest");

    let index_pointer = if let Some(manifest_cid) = manifest_write.manifest_cid.as_deref() {
        match put_music_index_pointer(&headers, &asset_cid, manifest_cid, &owner).await {
            Ok(upstream) if upstream.status.is_success() => IndexPointerSummary {
                status: "stored",
                route: pointer_route.clone(),
                http_status: Some(upstream.status.as_u16()),
            },
            Ok(upstream) => {
                warnings.push(format!("index_pointer_http_{}", upstream.status.as_u16()));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route.clone(),
                    http_status: Some(upstream.status.as_u16()),
                }
            }
            Err(response) => {
                warnings.push(response_warning(&response, "index_pointer_failed"));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route.clone(),
                    http_status: None,
                }
            }
        }
    } else {
        warnings.push("index_pointer_skipped_missing_manifest_cid".to_owned());
        IndexPointerSummary {
            status: "skipped",
            route: pointer_route.clone(),
            http_status: None,
        }
    };

    let crab_url = format!("crab://{raw_hash}.music");
    let manifest_raw = manifest_write
        .manifest_cid
        .as_ref()
        .map(|manifest_cid| format!("/o/{manifest_cid}"));

    let response = json!({
        "schema": MUSIC_UPLOAD_SCHEMA,
        "asset_kind": "music",
        "asset_cid": &asset_cid,
        "cid": &asset_cid,
        "crab_url": &crab_url,
        "storage_upload": storage_upload_json,
        "manifest": {
            "status": manifest_write.status,
            "manifest_cid": manifest_write.manifest_cid,
            "storage_path": manifest_write.storage_path,
        },
        "index_pointer": index_pointer,
        "owner": owner,
        "payout": payout,
        "links": {
            "raw": format!("/o/b3:{raw_hash}"),
            "crab": &crab_url,
            "http_b3": format!("/v1/b3/{raw_hash}.music"),
            "resolve": format!("/v1/crab/resolve?url=crab://{raw_hash}.music"),
            "manifest_raw": manifest_raw,
        },
        "warnings": warnings,
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Prepare a podcast/audio asset upload.
///
/// This is podcast-specific UX glue over paid storage estimate. It intentionally
/// does not create a wallet hold, store bytes, mint receipts, verify legal
/// ownership, verify guest releases, or mutate ledger/accounting.
pub async fn podcast_prepare(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<PodcastAssetPrepareRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_podcast_prepare_request",
                "podcast prepare request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    if request.bytes == 0 {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_podcast_prepare_request",
            "bytes must be greater than zero",
            false,
            "invalid_bytes",
        );
    }

    if let Some(content_type) = &request.content_type {
        if !is_valid_audio_content_type(content_type) {
            return problem(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "invalid_podcast_content_type",
                "content_type must be an audio/* media type",
                false,
                "invalid_content_type",
            );
        }
    }

    if !request.legal_attestation_accepted {
        return problem(
            StatusCode::BAD_REQUEST,
            "podcast_rights_attestation_required",
            "podcast prepare requires creator rights attestation",
            false,
            "missing_podcast_rights_attestation",
        );
    }

    if !request.guest_permission_attested {
        return problem(
            StatusCode::BAD_REQUEST,
            "podcast_guest_permission_required",
            "podcast prepare requires guest/voice permission attestation",
            false,
            "missing_podcast_guest_permission",
        );
    }

    if let Some(expected_asset_cid) = &request.expected_asset_cid {
        if !is_canonical_b3_cid(expected_asset_cid) {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_expected_asset_cid",
                "expected_asset_cid must be canonical b3:<64 lowercase hex>",
                false,
                "invalid_expected_asset_cid",
            );
        }
    }

    let storage_estimate = match fetch_storage_estimate(
        request.bytes,
        headers,
        "storage estimate rejected asset prepare request",
    )
    .await
    {
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

    let payer_account = request
        .payer_account
        .clone()
        .unwrap_or_else(|| "acct_dev".to_owned());

    let idempotency_key_hint = request
        .client_idempotency_key
        .clone()
        .or_else(|| Some(format!("prepare:podcast:{action}:{}", request.bytes)));

    let response = json!({
        "schema": PODCAST_PREPARE_SCHEMA,
        "asset_kind": "podcast",
        "action": action,
        "asset": asset,
        "bytes": request.bytes,
        "content_type": request.content_type,
        "expected_asset_cid": request.expected_asset_cid,
        "file_name": request.file_name,
        "owner_passport_subject": request.owner_passport_subject,
        "title": request.title,
        "description": request.description,
        "tags": request.tags,
        "podcast": {
            "show_title": request.show_title,
            "host_display": request.host_display,
            "guest_display": request.guest_display,
            "season_number": request.season_number,
            "episode_number": request.episode_number,
            "duration": request.duration,
            "category": request.category,
            "language": request.language,
            "explicit_rating": request.explicit_rating,
            "cover_image_crab_url": request.cover_image_crab_url,
            "transcript_crab_url": request.transcript_crab_url,
            "chapters_crab_url": request.chapters_crab_url,
            "show_page_crab_url": request.show_page_crab_url,
            "rights_mode": request.rights_mode,
            "license_mode": request.license_mode,
            "guest_permission_attested": request.guest_permission_attested,
            "legal_attestation_accepted": request.legal_attestation_accepted
        },
        "paid_storage": {
            "estimate_path": "/paid/o/estimate",
            "submit_path": "/paid/o",
            "estimate": storage_estimate,
        },
        "estimate": {
            "amount_minor": amount_minor,
            "amount": amount_minor,
            "minimum_hold_minor": minimum_hold_minor,
            "asset": DEFAULT_ASSET,
            "currency": DEFAULT_CURRENCY,
            "action": DEFAULT_ACTION,
            "bytes": request.bytes,
        },
        "wallet_hold": {
            "required": true,
            "from": payer_account,
            "to": "escrow_paid_write",
            "asset": DEFAULT_ASSET,
            "amount_minor": minimum_hold_minor,
            "amount": minimum_hold_minor,
            "idempotency_key_hint": idempotency_key_hint,
            "memo": "CrabLink podcast upload hold"
        },
        "next": {
            "create_hold": "/v1/wallet/hold",
            "submit_upload": "/v1/assets/podcast",
            "requires_paid_headers": true,
            "requires_rights_header": "x-ron-podcast-rights-attested=true",
            "requires_guest_permission_header": "x-ron-podcast-guest-permission-attested=true"
        },
        "accepted_headers": {
            "upload": [
                "content-type",
                "idempotency-key",
                "x-ron-passport",
                "x-ron-wallet-account",
                "x-ron-asset-title",
                "x-ron-asset-description",
                "x-ron-asset-tags",
                "x-ron-podcast-host",
                "x-ron-podcast-show",
                "x-ron-podcast-guest",
                "x-ron-podcast-episode",
                "x-ron-podcast-season",
                "x-ron-podcast-duration",
                "x-ron-podcast-language",
                "x-ron-podcast-category",
                "x-ron-podcast-explicit-rating",
                "x-ron-podcast-cover-image-crab-url",
                "x-ron-podcast-transcript-crab-url",
                "x-ron-podcast-chapters-crab-url",
                "x-ron-podcast-show-page-crab-url",
                "x-ron-podcast-rights-attested",
                "x-ron-podcast-guest-permission-attested",
                "x-ron-permission",
                "x-ron-spend-limit",
                "x-correlation-id",
                "x-request-id"
            ]
        },
        "warnings": [
            "podcast_lite_only_no_transcoding_no_drm_no_cover_art_upload",
            "cover_art_and_transcripts_are_reference_only",
            "legal_attestation_is_creator_confirmation_not_backend_ownership_proof",
            "podcast_lite_is_recorded_audio_not_live_stream_delivery"
        ],
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Upload bounded audio bytes through paid storage, then write a podcast manifest
/// object and `svc-index` asset→manifest pointer.
///
/// This is intentionally podcast-lite. Storage enforces the paid write. Index
/// owns the pointer. Cover art, transcript, chapters, and show pages are
/// references only and are never uploaded by this route.
pub async fn podcast_upload(headers: HeaderMap, body: Body) -> Response {
    let body = match axum::body::to_bytes(body, IMAGE_UPLOAD_BODY_LIMIT_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return problem(
                StatusCode::PAYLOAD_TOO_LARGE,
                "podcast_upload_body_too_large",
                "podcast upload body exceeded the configured media upload cap",
                false,
                "podcast_upload_body_too_large",
            );
        }
    };

    let Some(content_type) = grab(&headers, "content-type") else {
        return problem(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "missing_podcast_content_type",
            "podcast upload requires an audio/* content type",
            false,
            "missing_content_type",
        );
    };

    if !is_valid_audio_content_type(&content_type) {
        return problem(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "invalid_podcast_content_type",
            "podcast upload content-type must be audio/*",
            false,
            "invalid_content_type",
        );
    }

    if !truthy_header(&headers, "x-ron-podcast-rights-attested") {
        return problem(
            StatusCode::BAD_REQUEST,
            "podcast_rights_attestation_required",
            "podcast upload requires x-ron-podcast-rights-attested=true",
            false,
            "missing_podcast_rights_attestation",
        );
    }

    if !truthy_header(&headers, "x-ron-podcast-guest-permission-attested") {
        return problem(
            StatusCode::BAD_REQUEST,
            "podcast_guest_permission_required",
            "podcast upload requires x-ron-podcast-guest-permission-attested=true",
            false,
            "missing_podcast_guest_permission",
        );
    }

    let storage_upload = match send_to_storage(
        Method::POST,
        "/paid/o",
        headers.clone(),
        body,
        "storage paid podcast upload upstream unavailable",
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
                "storage paid podcast upload response was not valid JSON",
                true,
                "storage_bad_json",
            );
        }
    };

    let Some(asset_cid) = value_string(&storage_upload_json, "cid") else {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_missing_cid",
            "storage paid podcast upload response did not include cid",
            true,
            "storage_missing_cid",
        );
    };

    if !is_canonical_b3_cid(&asset_cid) {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_invalid_cid",
            "storage paid podcast upload response included invalid cid",
            true,
            "storage_invalid_cid",
        );
    }

    let owner = owner_from_headers(&headers);
    let payout = PayoutSummary {
        default_action: "content_view",
        recipient_account: owner.wallet_account.clone(),
    };

    let manifest = build_podcast_manifest(&headers, &storage_upload_json, &asset_cid, &owner);
    let manifest_bytes = match serde_json::to_vec(&manifest) {
        Ok(bytes) => Bytes::from(bytes),
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "manifest_encode_failed",
                "failed to encode generated podcast manifest",
                false,
                "manifest_encode_failed",
            );
        }
    };

    let mut warnings = Vec::new();
    warnings.push("podcast_lite_only_no_transcoding_no_drm".to_owned());
    warnings.push("cover_art_reference_only_no_podcast_page_image_upload".to_owned());
    warnings.push("transcript_reference_only_no_podcast_page_transcript_upload".to_owned());
    warnings
        .push("legal_attestation_is_creator_confirmation_not_backend_ownership_proof".to_owned());
    warnings.push("podcast_lite_is_recorded_audio_not_live_stream_delivery".to_owned());

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

    let raw_hash = asset_cid.trim_start_matches("b3:").to_owned();
    let pointer_route = format!("/v1/index/assets/{raw_hash}/manifest");

    let index_pointer = if let Some(manifest_cid) = manifest_write.manifest_cid.as_deref() {
        match put_podcast_index_pointer(&headers, &asset_cid, manifest_cid, &owner).await {
            Ok(upstream) if upstream.status.is_success() => IndexPointerSummary {
                status: "stored",
                route: pointer_route.clone(),
                http_status: Some(upstream.status.as_u16()),
            },
            Ok(upstream) => {
                warnings.push(format!("index_pointer_http_{}", upstream.status.as_u16()));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route.clone(),
                    http_status: Some(upstream.status.as_u16()),
                }
            }
            Err(response) => {
                warnings.push(response_warning(&response, "index_pointer_failed"));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route.clone(),
                    http_status: None,
                }
            }
        }
    } else {
        warnings.push("index_pointer_skipped_missing_manifest_cid".to_owned());
        IndexPointerSummary {
            status: "skipped",
            route: pointer_route.clone(),
            http_status: None,
        }
    };

    if manifest_write.status != "stored" || manifest_write.manifest_cid.is_none() {
        return problem(
            StatusCode::BAD_GATEWAY,
            "podcast_manifest_write_failed",
            "podcast upload stored audio bytes but failed to store the required manifest object",
            true,
            "podcast_manifest_write_failed",
        );
    }

    if index_pointer.status != "stored" {
        return problem(
            StatusCode::BAD_GATEWAY,
            "podcast_index_pointer_write_failed",
            "podcast upload stored audio bytes but failed to write the required asset manifest pointer",
            true,
            "podcast_index_pointer_write_failed",
        );
    }

    let crab_url = format!("crab://{raw_hash}.podcast");
    let manifest_raw = manifest_write
        .manifest_cid
        .as_ref()
        .map(|manifest_cid| format!("/o/{manifest_cid}"));

    let response = json!({
        "schema": PODCAST_UPLOAD_SCHEMA,
        "asset_kind": "podcast",
        "kind": "podcast",
        "asset_cid": &asset_cid,
        "cid": &asset_cid,
        "crab_url": &crab_url,
        "url": &crab_url,
        "storage_upload": storage_upload_json,
        "manifest": {
            "status": manifest_write.status,
            "manifest_cid": manifest_write.manifest_cid,
            "storage_path": manifest_write.storage_path,
        },
        "index_pointer": index_pointer,
        "owner": owner,
        "payout": payout,
        "links": {
            "raw": format!("/o/b3:{raw_hash}"),
            "crab": &crab_url,
            "http_b3": format!("/v1/b3/{raw_hash}.podcast"),
            "resolve": format!("/v1/crab/resolve?url=crab://{raw_hash}.podcast"),
            "manifest_raw": manifest_raw,
        },
        "warnings": warnings,
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Prepare a stream descriptor publication.
///
/// This quotes paid storage for the descriptor JSON only. It does not create a
/// live session, ingest media, grant viewer access, or mutate wallet/ledger truth.
pub async fn stream_prepare(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<StreamAssetRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_stream_prepare_request",
                "stream prepare request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    if request.title.trim().is_empty() {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_stream_prepare_request",
            "title is required",
            false,
            "missing_title",
        );
    }

    let descriptor_bytes = match serde_json::to_vec(&request) {
        Ok(bytes) => bytes.len() as u64,
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "stream_descriptor_encode_failed",
                "failed to encode stream descriptor preview",
                false,
                "stream_descriptor_encode_failed",
            );
        }
    };

    let storage_estimate = match fetch_storage_estimate(
        descriptor_bytes,
        headers,
        "storage estimate rejected stream prepare request",
    )
    .await
    {
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

    let payer_account =
        stream_creator_wallet(&request).or_else(|| stream_policy_recipient(&request));
    let idempotency_key_hint = request
        .client_idempotency_key
        .clone()
        .or_else(|| Some(format!("prepare:stream:{action}:{descriptor_bytes}")));

    let response = json!({
        "schema": STREAM_PREPARE_SCHEMA,
        "asset_kind": "stream",
        "action": action,
        "asset": asset,
        "bytes": descriptor_bytes,
        "title": request.title,
        "description": request.description,
        "tags": request.tags,
        "stream_kind": request.stream_kind.clone().unwrap_or_else(|| "live_video".to_owned()),
        "status_hint": request.status_hint.unwrap_or_else(|| "scheduled".to_owned()),
        "access_policy": request.access_policy,
        "paid_storage": {
            "estimate_path": "/v1/paid/o/prepare",
            "submit_path": "/v1/paid/o",
            "estimate": storage_estimate,
        },
        "wallet_hold": {
            "required": true,
            "action": DEFAULT_ACTION,
            "currency": DEFAULT_CURRENCY,
            "amount_minor": amount_minor,
            "minimum_hold_minor": minimum_hold_minor,
            "payer_account": payer_account,
            "idempotency_key_hint": idempotency_key_hint,
            "capability": {
                "required_action": "wallet.hold",
                "resource": "paid_storage_put",
                "audience": "svc-wallet",
                "recommended_ttl_seconds": 300
            }
        },
        "manifest_preview": {
            "will_create_manifest": true,
            "will_index_asset_pointer": true,
            "descriptor_only": true,
            "note": "stream descriptor publication only; live ingest and viewer paid access are separate future routes"
        },
        "next": {
            "create_hold": "/v1/wallet/hold",
            "submit_publish": "/v1/assets/stream",
            "resolve_after_publish": "/v1/crab/resolve?url=crab://<hash>.stream",
            "required_publish_headers": [
                "Authorization",
                "Idempotency-Key",
                "x-ron-paid-op",
                "x-ron-paid-asset",
                "x-ron-paid-estimate-minor",
                "x-ron-wallet-txid",
                "x-ron-wallet-receipt-hash",
                "x-ron-wallet-from",
                "x-ron-wallet-to"
            ],
            "optional_publish_headers": [
                "x-ron-passport",
                "x-ron-wallet-account",
                "x-ron-asset-title",
                "x-ron-asset-description",
                "x-ron-asset-tags",
                "x-ron-permission",
                "x-ron-spend-limit"
            ]
        },
        "warnings": [
            "stream_descriptor_only",
            "live_ingest_not_started_by_prepare",
            "viewer_access_routes_not_part_of_descriptor_publish"
        ]
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Publish a paid stream descriptor.
///
/// The descriptor itself becomes the canonical b3-backed `.stream` asset. Live
/// media chunks/segments are not accepted here and must use bounded future
/// stream-session routes.
pub async fn stream_publish(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<StreamAssetRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_stream_publish_request",
                "stream publish request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    if request.title.trim().is_empty() {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_stream_publish_request",
            "title is required",
            false,
            "missing_title",
        );
    }

    let owner = stream_owner_from_request(&headers, &request);
    let stream_id_seed = request
        .client_idempotency_key
        .clone()
        .unwrap_or_else(|| format!("{}:{}", request.title, now_ms()));
    let stream_id = format!("stream_{}", short_stable_id(&stream_id_seed));

    /*
     * Important:
     * The stream descriptor is the canonical .stream asset object.
     * The content_view route, however, pays through an asset manifest pointer.
     * Therefore stream publish must use the same two-object model as video:
     *
     *   descriptor JSON  -> paid storage -> asset_cid -> crab://<hash>.stream
     *   manifest JSON    -> storage      -> manifest_cid
     *   svc-index pointer asset_cid -> manifest_cid
     *
     * Do not point asset_cid at itself as the manifest. A manifest must contain
     * the asset_cid/payout/owner fields content_view validates.
     */
    let descriptor = build_stream_descriptor(&request, &owner, &stream_id);
    let descriptor_bytes = match serde_json::to_vec(&descriptor) {
        Ok(bytes) => Bytes::from(bytes),
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "stream_descriptor_encode_failed",
                "failed to encode stream descriptor",
                false,
                "stream_descriptor_encode_failed",
            );
        }
    };

    let mut paid_headers = headers.clone();
    paid_headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );

    let storage_upload = match send_to_storage(
        Method::POST,
        "/paid/o",
        paid_headers,
        descriptor_bytes,
        "storage paid stream descriptor upstream unavailable",
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
                "storage paid stream descriptor response was not valid JSON",
                true,
                "storage_bad_json",
            );
        }
    };

    let Some(asset_cid) = value_string(&storage_upload_json, "cid") else {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_missing_cid",
            "storage paid stream descriptor response did not include cid",
            true,
            "storage_missing_cid",
        );
    };

    if !is_canonical_b3_cid(&asset_cid) {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_invalid_cid",
            "storage paid stream descriptor response included invalid cid",
            true,
            "storage_invalid_cid",
        );
    }

    let payout = PayoutSummary {
        default_action: "content_view",
        recipient_account: owner
            .wallet_account
            .clone()
            .or_else(|| stream_policy_recipient(&request)),
    };

    let manifest = build_stream_manifest(
        &request,
        &storage_upload_json,
        &asset_cid,
        &owner,
        &stream_id,
    );
    let manifest_bytes = match serde_json::to_vec(&manifest) {
        Ok(bytes) => Bytes::from(bytes),
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "stream_manifest_encode_failed",
                "failed to encode stream manifest",
                false,
                "stream_manifest_encode_failed",
            );
        }
    };

    let mut warnings = vec![
        "stream_descriptor_only_no_live_ingest".to_owned(),
        "viewer_access_routes_future".to_owned(),
    ];

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
                    warnings.push("stream_manifest_storage_missing_valid_cid".to_owned());
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
                "stream_manifest_storage_http_{}",
                upstream.status.as_u16()
            ));
            ManifestWriteSummary {
                status: "failed",
                manifest_cid: None,
                storage_path: "/o",
            }
        }
        Err(response) => {
            warnings.push(response_warning(
                &response,
                "stream_manifest_storage_failed",
            ));
            ManifestWriteSummary {
                status: "failed",
                manifest_cid: None,
                storage_path: "/o",
            }
        }
    };

    let raw_hash = asset_cid.trim_start_matches("b3:").to_owned();
    let pointer_route = format!("/v1/index/assets/{raw_hash}/manifest");

    let index_pointer = if let Some(manifest_cid) = &manifest_write.manifest_cid {
        match put_stream_index_pointer(&headers, &asset_cid, manifest_cid, &owner).await {
            Ok(upstream) if upstream.status.is_success() => IndexPointerSummary {
                status: "stored",
                route: pointer_route.clone(),
                http_status: Some(upstream.status.as_u16()),
            },
            Ok(upstream) => {
                warnings.push(format!(
                    "stream_index_pointer_http_{}",
                    upstream.status.as_u16()
                ));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route.clone(),
                    http_status: Some(upstream.status.as_u16()),
                }
            }
            Err(response) => {
                warnings.push(response_warning(&response, "stream_index_pointer_failed"));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route.clone(),
                    http_status: None,
                }
            }
        }
    } else {
        warnings.push("stream_index_pointer_skipped_missing_manifest_cid".to_owned());
        IndexPointerSummary {
            status: "skipped",
            route: pointer_route.clone(),
            http_status: None,
        }
    };

    let crab_url = format!("crab://{raw_hash}.stream");
    let manifest_raw = manifest_write
        .manifest_cid
        .as_ref()
        .map(|manifest_cid| format!("/o/{manifest_cid}"));

    let response = json!({
        "schema": STREAM_PUBLISH_SCHEMA,
        "asset_kind": "stream",
        "asset_cid": &asset_cid,
        "cid": &asset_cid,
        "crab_url": &crab_url,
        "stream_id": &stream_id,
        "status": request
            .status_hint
            .clone()
            .unwrap_or_else(|| "scheduled".to_owned()),
        "descriptor": descriptor,
        "storage_upload": storage_upload_json,
        "manifest": {
            "status": manifest_write.status,
            "manifest_cid": &manifest_write.manifest_cid,
            "storage_path": manifest_write.storage_path,
        },
        "index_pointer": index_pointer,
        "owner": owner,
        "payout": payout,
        "links": {
            "raw": format!("/o/b3:{raw_hash}"),
            "descriptor_raw": format!("/o/b3:{raw_hash}"),
            "crab": &crab_url,
            "http_b3": format!("/v1/b3/{raw_hash}.stream"),
            "resolve": format!("/v1/crab/resolve?url=crab://{raw_hash}.stream"),
            "manifest_raw": manifest_raw,
        },
        "warnings": warnings
    });

    (StatusCode::OK, Json(response)).into_response()
}

async fn fetch_storage_estimate(
    bytes: u64,
    headers: HeaderMap,
    rejection_message: &'static str,
) -> Result<Value, Response> {
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
                message: rejection_message,
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

async fn put_music_index_pointer(
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
        "asset_kind": "music",
        "manifest_cid": manifest_cid,
        "owner_passport_subject": owner.passport_subject.clone(),
        "owner_wallet_account": owner.wallet_account.clone(),
        "updated_at_ms": now_ms(),
    });

    let body = match serde_json::to_vec(&body) {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "index_pointer_encode_failed",
                "failed to encode music manifest pointer",
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
                "index music manifest pointer upstream unavailable",
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
                "index music manifest pointer upstream unavailable",
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

async fn put_podcast_index_pointer(
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
        "asset_kind": "podcast",
        "manifest_cid": manifest_cid,
        "owner_passport_subject": owner.passport_subject.clone(),
        "owner_wallet_account": owner.wallet_account.clone(),
        "updated_at_ms": now_ms(),
    });

    let body = match serde_json::to_vec(&body) {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "index_pointer_encode_failed",
                "failed to encode podcast manifest pointer",
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
                "index podcast manifest pointer upstream unavailable",
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
                "index podcast manifest pointer upstream unavailable",
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

async fn put_video_index_pointer(
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
        "asset_kind": "video",
        "manifest_cid": manifest_cid,
        "owner_passport_subject": owner.passport_subject.clone(),
        "owner_wallet_account": owner.wallet_account.clone(),
        "updated_at_ms": now_ms(),
    });

    let body = match serde_json::to_vec(&body) {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "index_pointer_encode_failed",
                "failed to encode video manifest pointer",
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

    if let Some(rendition_group) = image_rendition_group_from_headers(headers, asset_cid) {
        root.insert("rendition_group".to_owned(), rendition_group);
    }

    if grab(headers, "x-ron-image-rendition-role").is_some()
        || grab(headers, "x-ron-image-rendition-label").is_some()
    {
        root.insert(
            "rendition".to_owned(),
            json!({
                "role": grab(headers, "x-ron-image-rendition-role"),
                "label": grab(headers, "x-ron-image-rendition-label"),
                "cid": asset_cid,
                "crab_url": format!("crab://{}.image", asset_cid.trim_start_matches("b3:")),
            }),
        );
    }

    let receipts = receipt_refs_from_storage_upload(storage_upload, owner);
    root.insert("receipts".to_owned(), json!(receipts));

    Value::Object(root)
}

fn build_video_manifest(
    headers: &HeaderMap,
    storage_upload: &Value,
    asset_cid: &str,
    owner: &OwnerSummary,
) -> Value {
    let title = grab(headers, "x-ron-asset-title").unwrap_or_else(|| "Untitled video".to_owned());
    let description = grab(headers, "x-ron-asset-description");
    let tags = tags_from_headers(headers);
    let content_type = grab(headers, "content-type");

    let mut root = Map::new();
    root.insert("version".to_owned(), json!(1));
    root.insert("asset_cid".to_owned(), json!(asset_cid));
    root.insert("asset_kind".to_owned(), json!("video"));

    if owner.passport_subject.is_some() && owner.wallet_account.is_some() {
        root.insert(
            "owner".to_owned(),
            json!({
                "passport_subject": owner.passport_subject.clone(),
                "wallet_account": owner.wallet_account.clone(),
            }),
        );
    }

    if owner.wallet_account.is_some() {
        root.insert(
            "payout".to_owned(),
            json!({
                "default_action": "content_view",
                "recipient_account": owner.wallet_account.clone(),
                "splits": [
                    {
                        "role": "creator",
                        "account": owner.wallet_account.clone(),
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
            "media_kind": "video",
            "video": {
                "duration": grab(headers, "x-ron-video-duration"),
                "resolution": grab(headers, "x-ron-video-resolution"),
                "aspect_ratio": grab(headers, "x-ron-video-aspect-ratio"),
                "video_kind": grab(headers, "x-ron-video-kind"),
                "language": grab(headers, "x-ron-video-language"),
            }
        }),
    );

    root.insert(
        "provenance".to_owned(),
        json!({
            "created_at_ms": now_ms(),
            "source": "omnigate.video_upload",
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
            "mode": "video_lite",
        }),
    );

    root.insert(
        "media".to_owned(), 
        json!({
            "mode": "video_lite",
            "range_streaming": false,
            "transcoding": false,
            "drm": false,
            "note": "video-lite proof path only; production range/segment streaming remains future work"
        }),
    );

    let receipts = receipt_refs_from_storage_upload(storage_upload, owner);
    root.insert("receipts".to_owned(), json!(receipts));

    Value::Object(root)
}

fn build_podcast_manifest(
    headers: &HeaderMap,
    storage_upload: &Value,
    asset_cid: &str,
    owner: &OwnerSummary,
) -> Value {
    let title =
        grab(headers, "x-ron-asset-title").unwrap_or_else(|| "Untitled podcast episode".to_owned());
    let description = grab(headers, "x-ron-asset-description");
    let tags = tags_from_headers(headers);
    let content_type = grab(headers, "content-type");

    let cover_image_crab_url = grab(headers, "x-ron-podcast-cover-image-crab-url")
        .or_else(|| grab(headers, "x-ron-cover-image-crab-url"));
    let transcript_crab_url = grab(headers, "x-ron-podcast-transcript-crab-url")
        .or_else(|| grab(headers, "x-ron-transcript-crab-url"));
    let chapters_crab_url = grab(headers, "x-ron-podcast-chapters-crab-url");
    let show_page_crab_url = grab(headers, "x-ron-podcast-show-page-crab-url")
        .or_else(|| grab(headers, "x-ron-show-page-crab-url"));
    let guest_display = grab(headers, "x-ron-podcast-guest")
        .or_else(|| grab(headers, "x-ron-podcast-guests"))
        .or_else(|| grab(headers, "x-ron-podcast-guest-display"));

    let mut root = Map::new();
    root.insert("version".to_owned(), json!(1));
    root.insert("asset_cid".to_owned(), json!(asset_cid));
    root.insert("asset_kind".to_owned(), json!("podcast"));
    root.insert("kind".to_owned(), json!("podcast"));
    root.insert(
        "canonical_crab_url".to_owned(),
        json!(format!(
            "crab://{}.podcast",
            asset_cid.trim_start_matches("b3:")
        )),
    );

    if owner.passport_subject.is_some() && owner.wallet_account.is_some() {
        root.insert(
            "owner".to_owned(),
            json!({
                "passport_subject": owner.passport_subject.clone(),
                "wallet_account": owner.wallet_account.clone(),
            }),
        );
    }

    if owner.wallet_account.is_some() {
        root.insert(
            "payout".to_owned(),
            json!({
                "default_action": "content_view",
                "recipient_account": owner.wallet_account.clone(),
                "splits": [
                    {
                        "role": "creator",
                        "account": owner.wallet_account.clone(),
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
            "media_kind": "podcast",
            "podcast": {
                "show_title": grab(headers, "x-ron-podcast-show"),
                "host_display": grab(headers, "x-ron-podcast-host"),
                "guest_display": guest_display,
                "season_number": grab(headers, "x-ron-podcast-season"),
                "episode_number": grab(headers, "x-ron-podcast-episode"),
                "duration": grab(headers, "x-ron-podcast-duration"),
                "category": grab(headers, "x-ron-podcast-category"),
                "language": grab(headers, "x-ron-podcast-language"),
                "explicit_rating": grab(headers, "x-ron-podcast-explicit-rating"),
                "rights_attested": truthy_header(headers, "x-ron-podcast-rights-attested"),
                "guest_permission_attested": truthy_header(headers, "x-ron-podcast-guest-permission-attested"),
            }
        }),
    );

    root.insert(
        "linked_assets".to_owned(),
        json!({
            "cover_image_crab_url": cover_image_crab_url,
            "cover_image_upload_from_podcast_page": false,
            "transcript_crab_url": transcript_crab_url,
            "chapters_crab_url": chapters_crab_url,
            "show_page_crab_url": show_page_crab_url
        }),
    );

    root.insert(
        "rights_policy".to_owned(),
        json!({
            "rights_mode": grab(headers, "x-ron-podcast-rights-mode"),
            "license_mode": grab(headers, "x-ron-podcast-license-mode"),
            "legal_attestation": {
                "accepted": truthy_header(headers, "x-ron-podcast-rights-attested"),
                "guest_permission_attested": truthy_header(headers, "x-ron-podcast-guest-permission-attested"),
                "backend_verified": false,
                "local_ui_attestation_only": true,
                "proves_legal_ownership": false,
                "proves_guest_release": false
            }
        }),
    );

    root.insert(
        "provenance".to_owned(),
        json!({
            "created_at_ms": now_ms(),
            "source": "omnigate.podcast_upload",
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
            "mode": "podcast_lite",
        }),
    );

    root.insert(
        "media".to_owned(),
        json!({
            "mode": "podcast_lite",
            "range_streaming": false,
            "transcoding": false,
            "drm": false,
            "live_stream": false,
            "note": "podcast-lite proof path only; live stream/session delivery remains future work"
        }),
    );

    root.insert(
        "truth_boundary".to_owned(),
        json!({
            "backend_uploaded": true,
            "audio_asset_uploaded": true,
            "creates_live_stream_session": false,
            "cover_art_upload_from_podcast_page": false,
            "transcript_upload_from_podcast_page": false,
            "legal_ownership_backend_verified": false,
            "guest_release_backend_verified": false,
            "wallet_mutation_by_omnigate": false
        }),
    );

    let receipts = receipt_refs_from_storage_upload(storage_upload, owner);
    root.insert("receipts".to_owned(), json!(receipts));

    Value::Object(root)
}

fn build_music_manifest(
    headers: &HeaderMap,
    storage_upload: &Value,
    asset_cid: &str,
    owner: &OwnerSummary,
) -> Value {
    let title = grab(headers, "x-ron-asset-title").unwrap_or_else(|| "Untitled track".to_owned());
    let description = grab(headers, "x-ron-asset-description");
    let tags = tags_from_headers(headers);
    let content_type = grab(headers, "content-type");

    let cover_image_crab_url = grab(headers, "x-ron-cover-image-crab-url");
    let lyrics_crab_url = grab(headers, "x-ron-lyrics-crab-url");

    let mut root = Map::new();
    root.insert("version".to_owned(), json!(1));
    root.insert("asset_cid".to_owned(), json!(asset_cid));
    root.insert("asset_kind".to_owned(), json!("music"));
    root.insert("kind".to_owned(), json!("music"));

    if owner.passport_subject.is_some() && owner.wallet_account.is_some() {
        root.insert(
            "owner".to_owned(),
            json!({
                "passport_subject": owner.passport_subject.clone(),
                "wallet_account": owner.wallet_account.clone(),
            }),
        );
    }

    if owner.wallet_account.is_some() {
        root.insert(
            "payout".to_owned(),
            json!({
                "default_action": "content_view",
                "recipient_account": owner.wallet_account.clone(),
                "splits": [
                    {
                        "role": "creator",
                        "account": owner.wallet_account.clone(),
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
            "media_kind": "music",
            "music": {
                "artist_display": grab(headers, "x-ron-music-artist"),
                "album_title": grab(headers, "x-ron-music-album"),
                "release_type": grab(headers, "x-ron-music-release-type"),
                "duration": grab(headers, "x-ron-music-duration"),
                "genre": grab(headers, "x-ron-music-genre"),
                "language": grab(headers, "x-ron-music-language"),
                "bpm": grab(headers, "x-ron-music-bpm"),
                "key_signature": grab(headers, "x-ron-music-key"),
                "explicit_rating": grab(headers, "x-ron-music-explicit-rating"),
                "rights_attested": truthy_header(headers, "x-ron-music-rights-attested"),
            }
        }),
    );

    root.insert(
        "linked_assets".to_owned(),
        json!({
            "cover_image_crab_url": cover_image_crab_url,
            "cover_image_upload_from_music_page": false,
            "lyrics_crab_url": lyrics_crab_url,
            "lyrics_are_separate_asset": true,
            "renditions": [],
            "alternates": [],
        }),
    );

    root.insert(
        "rights_policy".to_owned(),
        json!({
            "rights_mode": grab(headers, "x-ron-music-rights-mode"),
            "license_mode": grab(headers, "x-ron-music-license-mode"),
            "legal_attestation": {
                "accepted": truthy_header(headers, "x-ron-music-rights-attested"),
                "backend_verified": false,
                "note": "creator attestation only; this route does not prove copyright ownership"
            }
        }),
    );

    root.insert(
        "provenance".to_owned(),
        json!({
            "created_at_ms": now_ms(),
            "source": "omnigate.music_upload",
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
            "mode": "music_lite",
        }),
    );

    root.insert(
        "media".to_owned(),
        json!({
            "mode": "music_lite",
            "range_streaming": false,
            "transcoding": false,
            "drm": false,
            "cover_art_upload_from_music_page": false,
            "note": "music-lite proof path only; production range/segment streaming remains future work"
        }),
    );

    let receipts = receipt_refs_from_storage_upload(storage_upload, owner);
    root.insert("receipts".to_owned(), json!(receipts));

    Value::Object(root)
}

fn build_stream_descriptor(
    request: &StreamAssetRequest,
    owner: &OwnerSummary,
    stream_id: &str,
) -> Value {
    let mut root = Map::new();
    root.insert("version".to_owned(), json!(1));
    root.insert("schema".to_owned(), json!("omnigate.stream-descriptor.v1"));
    root.insert("asset_kind".to_owned(), json!("stream"));
    root.insert("kind".to_owned(), json!("stream"));
    root.insert("stream_id".to_owned(), json!(stream_id));
    root.insert(
        "status".to_owned(),
        json!(request
            .status_hint
            .clone()
            .unwrap_or_else(|| "scheduled".to_owned())),
    );

    if owner.passport_subject.is_some() || owner.wallet_account.is_some() {
        root.insert(
            "owner".to_owned(),
            json!({
                "passport_subject": owner.passport_subject.clone(),
                "wallet_account": owner.wallet_account.clone(),
            }),
        );
    }

    root.insert(
        "metadata".to_owned(),
        json!({
            "title": request.title,
            "description": request.description,
            "tags": request.tags,
            "stream_kind": request.stream_kind.clone().unwrap_or_else(|| "live_video".to_owned()),
            "content_type": "application/json; charset=utf-8",
        }),
    );

    root.insert(
        "access_policy".to_owned(),
        if request.access_policy.is_null() {
            json!({
                "action": "stream_watch_interval",
                "asset": "roc",
                "manual_renew_only": true,
                "autopay_enabled": false
            })
        } else {
            request.access_policy.clone()
        },
    );

    root.insert("creator".to_owned(), request.creator.clone());
    root.insert("source".to_owned(), request.source.clone());
    root.insert("linked_assets".to_owned(), request.linked_assets.clone());
    root.insert("chat".to_owned(), request.chat.clone());
    root.insert("moderation".to_owned(), request.moderation.clone());
    root.insert("rights".to_owned(), request.rights.clone());
    root.insert("payout".to_owned(), request.payout.clone());

    root.insert(
        "live_delivery".to_owned(),
        json!({
            "descriptor_only": true,
            "live_segments_backend_required": true,
            "viewer_route": format!("/streams/{stream_id}/watch"),
            "segment_route": format!("/streams/{stream_id}/segments/{{seq}}"),
            "status_route": format!("/streams/{stream_id}"),
            "no_drm_claim": true,
            "note": "stream-lite descriptor only; live segment routes are future-gated"
        }),
    );

    root.insert(
        "provenance".to_owned(),
        json!({
            "created_at_ms": now_ms(),
            "source": "omnigate.stream_publish.descriptor",
            "parent_cids": [],
        }),
    );

    Value::Object(root)
}

fn build_stream_manifest(
    request: &StreamAssetRequest,
    storage_upload: &Value,
    asset_cid: &str,
    owner: &OwnerSummary,
    stream_id: &str,
) -> Value {
    let recipient_account = owner
        .wallet_account
        .clone()
        .or_else(|| stream_policy_recipient(request));

    let mut root = Map::new();
    root.insert("version".to_owned(), json!(1));
    root.insert("asset_cid".to_owned(), json!(asset_cid));
    root.insert("asset_kind".to_owned(), json!("stream"));
    root.insert("kind".to_owned(), json!("stream"));
    root.insert("stream_id".to_owned(), json!(stream_id));
    root.insert(
        "status".to_owned(),
        json!(request
            .status_hint
            .clone()
            .unwrap_or_else(|| "scheduled".to_owned())),
    );

    if owner.passport_subject.is_some() || owner.wallet_account.is_some() {
        root.insert(
            "owner".to_owned(),
            json!({
                "passport_subject": owner.passport_subject.clone(),
                "wallet_account": owner.wallet_account.clone(),
            }),
        );
    }

    if recipient_account.is_some() {
        root.insert(
            "payout".to_owned(),
            json!({
                "default_action": "content_view",
                "stream_action": "stream_watch_interval",
                "recipient_account": recipient_account.clone(),
                "splits": [
                    {
                        "role": "creator",
                        "account": recipient_account.clone(),
                        "bps": 10_000
                    }
                ]
            }),
        );
    }

    root.insert(
        "metadata".to_owned(),
        json!({
            "title": request.title,
            "description": request.description,
            "tags": request.tags,
            "stream_kind": request.stream_kind.clone().unwrap_or_else(|| "live_video".to_owned()),
            "content_type": "application/json; charset=utf-8",
        }),
    );

    root.insert(
        "access_policy".to_owned(),
        if request.access_policy.is_null() {
            json!({
                "action": "stream_watch_interval",
                "asset": "roc",
                "manual_renew_only": true,
                "autopay_enabled": false
            })
        } else {
            request.access_policy.clone()
        },
    );

    root.insert("linked_assets".to_owned(), request.linked_assets.clone());
    root.insert("chat".to_owned(), request.chat.clone());
    root.insert("moderation".to_owned(), request.moderation.clone());
    root.insert("rights".to_owned(), request.rights.clone());

    root.insert(
        "live_delivery".to_owned(),
        json!({
            "descriptor_only": true,
            "live_segments_backend_required": true,
            "viewer_route": format!("/streams/{stream_id}/watch"),
            "segment_route": format!("/streams/{stream_id}/segments/{{seq}}"),
            "status_route": format!("/streams/{stream_id}"),
            "no_drm_claim": true,
            "note": "stream-lite descriptor only; live segment routes are future-gated"
        }),
    );

    root.insert(
        "provenance".to_owned(),
        json!({
            "created_at_ms": now_ms(),
            "source": "omnigate.stream_publish.manifest",
            "parent_cids": [asset_cid],
        }),
    );

    root.insert(
        "storage".to_owned(),
        json!({
            "available": true,
            "content_type": "application/json; charset=utf-8",
            "raw_url": format!("/o/{asset_cid}"),
            "paid": true,
            "mode": "stream_descriptor"
        }),
    );

    let receipts = receipt_refs_from_storage_upload(storage_upload, owner);
    root.insert("receipts".to_owned(), json!(receipts));

    Value::Object(root)
}

fn stream_owner_from_request(headers: &HeaderMap, request: &StreamAssetRequest) -> OwnerSummary {
    let header_owner = owner_from_headers(headers);
    OwnerSummary {
        passport_subject: header_owner
            .passport_subject
            .or_else(|| stream_creator_string(request, "passport_subject")),
        wallet_account: header_owner
            .wallet_account
            .or_else(|| stream_creator_wallet(request))
            .or_else(|| stream_policy_recipient(request)),
    }
}

fn stream_creator_string(request: &StreamAssetRequest, key: &str) -> Option<String> {
    request
        .creator
        .as_object()
        .and_then(|object| object.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn stream_creator_wallet(request: &StreamAssetRequest) -> Option<String> {
    stream_creator_string(request, "wallet_account")
}

fn stream_policy_recipient(request: &StreamAssetRequest) -> Option<String> {
    request
        .access_policy
        .as_object()
        .and_then(|object| object.get("recipient_account"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn short_stable_id(seed: &str) -> String {
    let mut hash: u32 = 0x811c9dc5;

    for byte in seed.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }

    format!("{hash:08x}")
}

async fn put_stream_index_pointer(
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
        "asset_kind": "stream",
        "manifest_cid": manifest_cid,
        "owner_passport_subject": owner.passport_subject.clone(),
        "owner_wallet_account": owner.wallet_account.clone(),
        "updated_at_ms": now_ms(),
    });

    let body = match serde_json::to_vec(&body) {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "index_pointer_encode_failed",
                "failed to encode stream manifest pointer",
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
                "index stream manifest pointer upstream unavailable",
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
                "index stream manifest pointer upstream unavailable",
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

fn image_rendition_group_from_headers(headers: &HeaderMap, asset_cid: &str) -> Option<Value> {
    let raw = grab(headers, "x-ron-image-rendition-group")?;

    if raw.len() > IMAGE_RENDITION_GROUP_HEADER_LIMIT_BYTES {
        return None;
    }

    let value = serde_json::from_str::<Value>(&raw).ok()?;
    let renditions = value.get("renditions")?.as_array()?;
    let contains_self = renditions
        .iter()
        .any(|item| value_string(item, "cid").as_deref() == Some(asset_cid));

    if !contains_self {
        return None;
    }

    Some(value)
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

fn is_valid_video_content_type(content_type: &str) -> bool {
    let trimmed = content_type.trim().to_ascii_lowercase();
    !trimmed.is_empty() && !trimmed.chars().any(char::is_control) && trimmed.starts_with("video/")
}

fn is_valid_audio_content_type(content_type: &str) -> bool {
    let lower = content_type.trim().to_ascii_lowercase();
    lower.starts_with("audio/")
}

fn truthy_header(headers: &HeaderMap, name: &str) -> bool {
    let Some(value) = grab(headers, name) else {
        return false;
    };

    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "accepted" | "attested"
    )
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
