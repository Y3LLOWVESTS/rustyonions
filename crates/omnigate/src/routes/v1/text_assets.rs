//! RO:WHAT — NEXT_LEVEL text asset routes for post/comment/article prepare and publish.
//! RO:WHY — Turns the proven image/site b3+manifest+index pattern into site-attached text primitives one step at a time.
//! RO:INTERACTS — svc-storage `/paid/o/estimate`, `/paid/o`, `/o`; svc-index asset manifest pointer routes; svc-gateway `/assets/{post,comment,article}*`.
//! RO:INVARIANTS — post/comment/article only; no wallet calls; no ledger mutation; storage stores bytes; index owns pointers; no fake CIDs/receipts.
//! RO:METRICS — covered by omnigate HTTP middleware when mounted through `App::build`.
//! RO:CONFIG — `OMNIGATE_STORAGE_BASE_URL`/`OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL`; `OMNIGATE_INDEX_BASE_URL`/`OMNIGATE_DOWNSTREAM_INDEX_BASE_URL`.
//! RO:SECURITY — strict JSON DTOs; site attachment required; comment parent required; article title/summary fields validated; paid proof headers required for publish; hop-by-hop headers filtered.
//! RO:TEST — `tests/text_asset_publish.rs`, `tests/comment_asset_publish.rs`, `tests/article_asset_publish.rs`.

use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Map, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_STORAGE_BASE_URL: &str = "http://127.0.0.1:15303";
const DEFAULT_INDEX_BASE_URL: &str = "http://127.0.0.1:5304";

const DEFAULT_ACTION: &str = "paid_storage_put";
const DEFAULT_ASSET: &str = "roc";
const DEFAULT_CURRENCY: &str = "ROC";
const TEXT_CONTENT_TYPE: &str = "application/json; charset=utf-8";
const MAX_TEXT_CONTENT_BYTES: usize = 1_048_576;

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate text asset route reqwest client should build")
});

/// Router for `/v1/assets/post*`, `/v1/assets/comment*`, and `/v1/assets/article*`.
///
/// Article is now the third staged NEXT_LEVEL text primitive after the
/// post/comment green gates. It uses the same b3+manifest+index pattern while
/// requiring site attachment and article-specific metadata.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/post/prepare", post(post_prepare))
        .route("/post", post(post_publish))
        .route("/comment/prepare", post(comment_prepare))
        .route("/comment", post(comment_publish))
        .route("/article/prepare", post(article_prepare))
        .route("/article", post(article_publish))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextAssetKind {
    Post,
    Comment,
    Article,
}

impl TextAssetKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Post => "post",
            Self::Comment => "comment",
            Self::Article => "article",
        }
    }

    fn prepare_schema(self) -> &'static str {
        match self {
            Self::Post => "omnigate.post-asset-prepare.v1",
            Self::Comment => "omnigate.comment-asset-prepare.v1",
            Self::Article => "omnigate.article-asset-prepare.v1",
        }
    }

    fn publish_schema(self) -> &'static str {
        match self {
            Self::Post => "omnigate.post-asset-publish.v1",
            Self::Comment => "omnigate.comment-asset-publish.v1",
            Self::Article => "omnigate.article-asset-publish.v1",
        }
    }

    fn content_schema(self) -> &'static str {
        match self {
            Self::Post => "ron.post-content.v1",
            Self::Comment => "ron.comment-content.v1",
            Self::Article => "ron.article-content.v1",
        }
    }

    fn publish_route(self) -> &'static str {
        match self {
            Self::Post => "/v1/assets/post",
            Self::Comment => "/v1/assets/comment",
            Self::Article => "/v1/assets/article",
        }
    }

    fn default_title(self) -> &'static str {
        match self {
            Self::Post => "Untitled post",
            Self::Comment => "Comment",
            Self::Article => "Untitled article",
        }
    }

    fn metadata_kind_key(self) -> &'static str {
        match self {
            Self::Post => "post_kind",
            Self::Comment => "comment_kind",
            Self::Article => "article_kind",
        }
    }

    fn default_kind_value(self) -> &'static str {
        match self {
            Self::Post => "short_text",
            Self::Comment => "reply",
            Self::Article => "essay",
        }
    }

    fn relation(self) -> &'static str {
        match self {
            Self::Post => "published_on_site",
            Self::Comment => "comment_on_site",
            Self::Article => "article_on_site",
        }
    }

    fn parent_required(self) -> bool {
        matches!(self, Self::Comment)
    }

    fn parent_relation(self) -> &'static str {
        match self {
            Self::Post => "reply_or_thread_parent",
            Self::Comment => "comment_parent",
            Self::Article => "source_or_series_parent",
        }
    }

    fn missing_parent_reason(self) -> &'static str {
        match self {
            Self::Post => "missing_parent_reference",
            Self::Comment => "missing_parent_reference",
            Self::Article => "missing_parent_reference",
        }
    }

    fn title_required(self) -> bool {
        matches!(self, Self::Post | Self::Article)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextAssetRequest {
    #[serde(default)]
    title: Option<String>,

    #[serde(default)]
    body: String,

    #[serde(default)]
    subtitle: Option<String>,

    #[serde(default)]
    summary: Option<String>,

    #[serde(default, alias = "heroImageCrabUrl")]
    hero_image_crab_url: Option<String>,

    #[serde(default, alias = "linkedSourceCrabUrl")]
    linked_source_crab_url: Option<String>,

    #[serde(default, alias = "siteContextCrabUrl")]
    site_context_crab_url: Option<String>,

    #[serde(
        default,
        alias = "parentCrabUrl",
        alias = "target_crab_url",
        alias = "targetCrabUrl"
    )]
    parent_crab_url: Option<String>,

    #[serde(default, alias = "threadContextCrabUrl")]
    thread_context_crab_url: Option<String>,

    #[serde(default, alias = "creatorDisplay")]
    creator_display: Option<String>,

    #[serde(default)]
    language: Option<String>,

    #[serde(
        default,
        alias = "post_kind",
        alias = "postKind",
        alias = "comment_kind",
        alias = "commentKind",
        alias = "article_kind",
        alias = "articleKind"
    )]
    text_kind: Option<String>,

    #[serde(default)]
    visibility: Option<String>,

    #[serde(default, alias = "rightsMode")]
    rights_mode: Option<String>,

    #[serde(default, alias = "moderationMode")]
    moderation_mode: Option<String>,

    #[serde(default, alias = "contentWarning")]
    content_warning: Option<String>,

    #[serde(default, deserialize_with = "deserialize_tags")]
    tags: Vec<String>,

    #[serde(default, alias = "payerAccount")]
    payer_account: Option<String>,

    #[serde(default, alias = "ownerPassportSubject")]
    owner_passport_subject: Option<String>,

    #[serde(default, alias = "clientIdempotencyKey")]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct TextAssetPrepareResponse {
    schema: &'static str,
    asset_kind: &'static str,
    action: String,
    asset: String,
    bytes: u64,
    content_type: &'static str,
    title: String,
    site_connection: SiteConnectionSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_reference: Option<ParentReferenceSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_reference: Option<ParentReferenceSummary>,
    owner_passport_subject: String,
    payer_account: String,
    tags: Vec<String>,
    paid_storage: PaidStoragePrepareSummary,
    wallet_hold: WalletHoldTemplate,
    manifest_preview: TextManifestPreview,
    next: TextPrepareNext,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct TextAssetPublishResponse {
    schema: &'static str,
    asset_kind: &'static str,
    asset_cid: String,
    crab_url: String,
    storage_upload: Value,
    manifest: ManifestWriteSummary,
    index_pointer: IndexPointerSummary,
    owner: OwnerSummary,
    payout: PayoutSummary,
    site_connection: SiteConnectionSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_reference: Option<ParentReferenceSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_reference: Option<ParentReferenceSummary>,
    links: TextAssetLinks,
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
    payer_account: String,
    idempotency_key_hint: String,
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
struct TextManifestPreview {
    will_create_content_object: bool,
    will_create_manifest: bool,
    will_index_asset_pointer: bool,
    owner_source: &'static str,
    note: &'static str,
}

#[derive(Debug, Serialize)]
struct TextPrepareNext {
    create_hold: &'static str,
    submit_publish: &'static str,
    resolve_after_publish: String,
    required_publish_headers: Vec<&'static str>,
    optional_publish_headers: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct SiteConnectionSummary {
    required: bool,
    relation: &'static str,
    crab_url: String,
}

#[derive(Debug, Serialize)]
struct ParentReferenceSummary {
    relation: &'static str,
    crab_url: String,
    asset_kind: String,
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
    passport_subject: String,
    wallet_account: String,
}

#[derive(Debug, Serialize)]
struct PayoutSummary {
    default_action: &'static str,
    recipient_account: String,
}

#[derive(Debug, Serialize)]
struct TextAssetLinks {
    raw: String,
    crab: String,
    http_b3: String,
    resolve: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest_raw: Option<String>,
}

#[derive(Debug, Serialize)]
struct PaidProofSummary {
    paid_op: String,
    paid_asset: String,
    paid_estimate_minor: String,
    wallet_txid: String,
    wallet_receipt_hash: String,
    wallet_from: String,
    wallet_to: String,
}

#[derive(Debug, Serialize)]
struct Problem<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
    reason: &'a str,
}

#[derive(Debug, Clone, Copy)]
struct RouteProblemSpec {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
}

impl RouteProblemSpec {
    fn into_response(self) -> Response {
        problem(
            self.status,
            self.code,
            self.message,
            self.retryable,
            self.reason,
        )
    }
}

type RouteResult<T> = Result<T, RouteProblemSpec>;

fn route_problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
) -> RouteProblemSpec {
    RouteProblemSpec {
        status,
        code,
        message,
        retryable,
        reason,
    }
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

/// Prepare a post asset publish flow.
///
/// This validates the post DTO, computes canonical text content bytes, asks
/// storage for a paid-write estimate, and returns a wallet hold template. It
/// does not write bytes, create manifests, call wallet, or mutate ledger.
pub async fn post_prepare(headers: HeaderMap, body: Bytes) -> Response {
    text_asset_prepare(TextAssetKind::Post, headers, body).await
}

/// Publish a post asset through paid storage, then store a post manifest and
/// write the asset→manifest pointer in `svc-index`.
pub async fn post_publish(headers: HeaderMap, body: Bytes) -> Response {
    text_asset_publish(TextAssetKind::Post, headers, body).await
}

/// Prepare a comment asset publish flow.
///
/// Comments require both a named site context and a typed parent/target asset
/// URL. This returns a wallet hold template only; it does not mutate wallet,
/// ledger, storage, or index.
pub async fn comment_prepare(headers: HeaderMap, body: Bytes) -> Response {
    text_asset_prepare(TextAssetKind::Comment, headers, body).await
}

/// Publish a comment asset through paid storage, then store a comment manifest
/// and write the asset→manifest pointer in `svc-index`.
pub async fn comment_publish(headers: HeaderMap, body: Bytes) -> Response {
    text_asset_publish(TextAssetKind::Comment, headers, body).await
}

/// Prepare an article asset publish flow.
///
/// Articles require a named site context, title, and body. Optional summary,
/// subtitle, hero image, and source references become article metadata and
/// relation graph fields in the b3-backed content object/manifest.
pub async fn article_prepare(headers: HeaderMap, body: Bytes) -> Response {
    text_asset_prepare(TextAssetKind::Article, headers, body).await
}

/// Publish an article asset through paid storage, then store an article
/// manifest and write the asset→manifest pointer in `svc-index`.
pub async fn article_publish(headers: HeaderMap, body: Bytes) -> Response {
    text_asset_publish(TextAssetKind::Article, headers, body).await
}

async fn text_asset_prepare(kind: TextAssetKind, headers: HeaderMap, body: Bytes) -> Response {
    let request = match parse_text_request(&body, invalid_request_code(kind, "prepare")) {
        Ok(request) => request,
        Err(err) => return err.into_response(),
    };

    if let Err(err) = validate_text_request(kind, &request, invalid_request_code(kind, "prepare")) {
        return err.into_response();
    }

    let content_bytes = match build_text_content_bytes(kind, &request) {
        Ok(content_bytes) => content_bytes,
        Err(err) => return err.into_response(),
    };

    let bytes = match u64::try_from(content_bytes.len()) {
        Ok(bytes) => bytes,
        Err(_) => {
            return problem(
                StatusCode::PAYLOAD_TOO_LARGE,
                "text_content_too_large",
                "text content envelope exceeds the maximum supported byte length",
                false,
                "content_too_large",
            );
        }
    };

    let storage_estimate = match fetch_storage_estimate(bytes, headers).await {
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

    let title = title_for(kind, &request);
    let payer_account = clean_option(&request.payer_account).unwrap_or_default();
    let owner_passport_subject = clean_option(&request.owner_passport_subject).unwrap_or_default();
    let idempotency_key_hint = request
        .client_idempotency_key
        .clone()
        .unwrap_or_else(|| format!("prepare:{}:{action}:{bytes}", kind.as_str()));

    let response = TextAssetPrepareResponse {
        schema: kind.prepare_schema(),
        asset_kind: kind.as_str(),
        action: action.clone(),
        asset,
        bytes,
        content_type: TEXT_CONTENT_TYPE,
        title,
        site_connection: site_connection_summary(kind, &request),
        parent_reference: parent_reference_summary(kind, &request),
        thread_reference: thread_reference_summary(&request),
        owner_passport_subject,
        payer_account: payer_account.clone(),
        tags: normalize_tags(&request.tags),
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
            payer_account,
            idempotency_key_hint,
            capability: WalletCapabilityHint {
                required_action: "wallet.hold",
                resource: "paid_storage_put",
                audience: "svc-wallet",
                recommended_ttl_seconds: 300,
            },
        },
        manifest_preview: TextManifestPreview {
            will_create_content_object: true,
            will_create_manifest: true,
            will_index_asset_pointer: true,
            owner_source: "request.owner_passport_subject_or_publish_headers",
            note: "content object, manifest creation, and index pointer write happen after paid text asset publish succeeds",
        },
        next: TextPrepareNext {
            create_hold: "/v1/wallet/hold",
            submit_publish: kind.publish_route(),
            resolve_after_publish: format!("/v1/crab/resolve?url=crab://<hash>.{}", kind.as_str()),
            required_publish_headers: required_paid_headers(),
            optional_publish_headers: optional_publish_headers(),
        },
        warnings: Vec::new(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn text_asset_publish(kind: TextAssetKind, headers: HeaderMap, body: Bytes) -> Response {
    let request = match parse_text_request(&body, invalid_request_code(kind, "publish")) {
        Ok(request) => request,
        Err(err) => return err.into_response(),
    };

    if let Err(err) = validate_text_request(kind, &request, invalid_request_code(kind, "publish")) {
        return err.into_response();
    }

    let paid_proof = match paid_proof_from_headers(&headers, kind.as_str()) {
        Ok(paid_proof) => paid_proof,
        Err(err) => return err.into_response(),
    };

    let content_bytes = match build_text_content_bytes(kind, &request) {
        Ok(content_bytes) => content_bytes,
        Err(err) => return err.into_response(),
    };

    let mut content_headers = headers.clone();
    content_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(TEXT_CONTENT_TYPE),
    );
    content_headers.insert(
        HeaderName::from_static("x-ron-asset-kind"),
        HeaderValue::from_static(kind.as_str()),
    );

    let storage_upload = match send_to_storage(
        Method::POST,
        "/paid/o",
        content_headers,
        content_bytes,
        "storage paid text asset upload upstream unavailable",
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
                "storage paid text asset upload response was not valid JSON",
                true,
                "storage_bad_json",
            );
        }
    };

    let Some(asset_cid) = value_string(&storage_upload_json, "cid") else {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_missing_cid",
            "storage paid text asset upload response did not include cid",
            true,
            "storage_missing_cid",
        );
    };

    if !is_canonical_b3_cid(&asset_cid) {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_invalid_cid",
            "storage paid text asset upload response included invalid cid",
            true,
            "storage_invalid_cid",
        );
    }

    let owner = owner_from_request_and_headers(&request, &headers);
    let payout = PayoutSummary {
        default_action: "content_view",
        recipient_account: owner.wallet_account.clone(),
    };

    let manifest = build_text_manifest(
        kind,
        &request,
        &headers,
        &storage_upload_json,
        &asset_cid,
        &owner,
        &paid_proof,
    );

    let manifest_bytes = match serde_json::to_vec(&manifest) {
        Ok(bytes) => Bytes::from(bytes),
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "manifest_encode_failed",
                "failed to encode generated text asset manifest",
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
        match put_index_pointer(&headers, &asset_cid, kind.as_str(), manifest_cid, &owner).await {
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
    let crab_url = format!("crab://{raw_hash}.{}", kind.as_str());

    let manifest_raw = manifest_write
        .manifest_cid
        .as_ref()
        .map(|manifest_cid| format!("/o/{manifest_cid}"));

    let response = TextAssetPublishResponse {
        schema: kind.publish_schema(),
        asset_kind: kind.as_str(),
        asset_cid: asset_cid.clone(),
        crab_url: crab_url.clone(),
        storage_upload: storage_upload_json,
        manifest: manifest_write,
        index_pointer,
        owner,
        payout,
        site_connection: site_connection_summary(kind, &request),
        parent_reference: parent_reference_summary(kind, &request),
        thread_reference: thread_reference_summary(&request),
        links: TextAssetLinks {
            raw: format!("/o/{asset_cid}"),
            crab: crab_url.clone(),
            http_b3: format!("/v1/b3/{raw_hash}.{}", kind.as_str()),
            resolve: format!("/v1/crab/resolve?url={crab_url}"),
            manifest_raw,
        },
        warnings,
    };

    (StatusCode::OK, Json(response)).into_response()
}

fn parse_text_request(body: &[u8], code: &'static str) -> RouteResult<TextAssetRequest> {
    serde_json::from_slice::<TextAssetRequest>(body).map_err(|_| {
        route_problem(
            StatusCode::BAD_REQUEST,
            code,
            "text asset request must be strict JSON",
            false,
            "bad_json",
        )
    })
}

fn validate_text_request(
    kind: TextAssetKind,
    request: &TextAssetRequest,
    code: &'static str,
) -> RouteResult<()> {
    let title = clean_option(&request.title);

    if kind.title_required() && title.is_none() {
        return Err(route_problem(
            StatusCode::BAD_REQUEST,
            code,
            "text asset title is required",
            false,
            "missing_title",
        ));
    }

    if title
        .as_deref()
        .is_some_and(|value| value.chars().count() > 180)
    {
        return Err(route_problem(
            StatusCode::BAD_REQUEST,
            code,
            "text asset title must be 180 characters or fewer",
            false,
            "title_too_long",
        ));
    }

    if clean_string(&request.body).is_empty() {
        return Err(route_problem(
            StatusCode::BAD_REQUEST,
            code,
            "text asset body is required",
            false,
            "missing_body",
        ));
    }

    let Some(site_url) = clean_option(&request.site_context_crab_url) else {
        return Err(route_problem(
            StatusCode::BAD_REQUEST,
            code,
            "text asset requires a site context crab URL",
            false,
            "missing_site_context",
        ));
    };

    if !is_valid_named_site_crab_url(&site_url) {
        return Err(route_problem(
            StatusCode::BAD_REQUEST,
            code,
            "site_context_crab_url must be a named crab:// site URL, not a typed asset URL",
            false,
            "invalid_site_context",
        ));
    }

    match clean_option(&request.parent_crab_url) {
        Some(parent_url) => {
            let Some(parent_kind) = typed_asset_kind_from_crab_url(&parent_url) else {
                return Err(route_problem(
                    StatusCode::BAD_REQUEST,
                    code,
                    "parent_crab_url must be a typed crab://<hash>.<kind> asset URL",
                    false,
                    "invalid_parent_reference",
                ));
            };

            if !is_allowed_comment_parent_kind(&parent_kind) {
                return Err(route_problem(
                    StatusCode::BAD_REQUEST,
                    code,
                    "parent_crab_url references an unsupported parent asset kind",
                    false,
                    "invalid_parent_kind",
                ));
            }
        }
        None if kind.parent_required() => {
            return Err(route_problem(
                StatusCode::BAD_REQUEST,
                code,
                "comment requires parent_crab_url or target_crab_url",
                false,
                kind.missing_parent_reason(),
            ));
        }
        None => {}
    }

    if let Some(thread_url) = clean_option(&request.thread_context_crab_url) {
        let Some(thread_kind) = typed_asset_kind_from_crab_url(&thread_url) else {
            return Err(route_problem(
                StatusCode::BAD_REQUEST,
                code,
                "thread_context_crab_url must be a typed crab://<hash>.<kind> asset URL",
                false,
                "invalid_thread_reference",
            ));
        };

        if thread_kind != "thread" && thread_kind != "post" {
            return Err(route_problem(
                StatusCode::BAD_REQUEST,
                code,
                "thread_context_crab_url must reference a thread or post asset",
                false,
                "invalid_thread_kind",
            ));
        }
    }

    if let Some(subtitle) = clean_option(&request.subtitle) {
        if subtitle.chars().count() > 220 {
            return Err(route_problem(
                StatusCode::BAD_REQUEST,
                code,
                "article subtitle must be 220 characters or fewer",
                false,
                "subtitle_too_long",
            ));
        }
    }

    if let Some(summary) = clean_option(&request.summary) {
        if summary.chars().count() > 1_000 {
            return Err(route_problem(
                StatusCode::BAD_REQUEST,
                code,
                "article summary must be 1000 characters or fewer",
                false,
                "summary_too_long",
            ));
        }
    }

    if let Some(hero_image) = clean_option(&request.hero_image_crab_url) {
        match typed_asset_kind_from_crab_url(&hero_image) {
            Some(asset_kind) if asset_kind == "image" => {}
            Some(_) => {
                return Err(route_problem(
                    StatusCode::BAD_REQUEST,
                    code,
                    "hero_image_crab_url must reference a .image asset",
                    false,
                    "invalid_hero_image_kind",
                ));
            }
            None => {
                return Err(route_problem(
                    StatusCode::BAD_REQUEST,
                    code,
                    "hero_image_crab_url must be a typed crab://<hash>.image asset URL",
                    false,
                    "invalid_hero_image_reference",
                ));
            }
        }
    }

    if let Some(source) = clean_option(&request.linked_source_crab_url) {
        if typed_asset_kind_from_crab_url(&source).is_none() {
            return Err(route_problem(
                StatusCode::BAD_REQUEST,
                code,
                "linked_source_crab_url must be a typed crab://<hash>.<kind> asset URL",
                false,
                "invalid_source_reference",
            ));
        }
    }

    if clean_option(&request.payer_account).is_none() {
        return Err(route_problem(
            StatusCode::BAD_REQUEST,
            code,
            "text asset requires payer_account",
            false,
            "missing_payer_account",
        ));
    }

    if clean_option(&request.owner_passport_subject).is_none() {
        return Err(route_problem(
            StatusCode::BAD_REQUEST,
            code,
            "text asset requires owner_passport_subject",
            false,
            "missing_owner_passport_subject",
        ));
    }

    for tag in normalize_tags(&request.tags) {
        if tag.chars().any(char::is_control) {
            return Err(route_problem(
                StatusCode::BAD_REQUEST,
                code,
                "text asset tags must not contain control characters",
                false,
                "invalid_tag",
            ));
        }
    }

    Ok(())
}

fn build_text_content_bytes(kind: TextAssetKind, request: &TextAssetRequest) -> RouteResult<Bytes> {
    let content = build_text_content_envelope(kind, request);

    let bytes = match serde_json::to_vec(&content) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Err(route_problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "text_content_encode_failed",
                "failed to encode text content envelope",
                false,
                "text_content_encode_failed",
            ));
        }
    };

    if bytes.is_empty() {
        return Err(route_problem(
            StatusCode::BAD_REQUEST,
            "invalid_text_content",
            "text content envelope must not be empty",
            false,
            "empty_content",
        ));
    }

    if bytes.len() > MAX_TEXT_CONTENT_BYTES {
        return Err(route_problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "text_content_too_large",
            "text content envelope exceeds 1 MiB",
            false,
            "content_too_large",
        ));
    }

    Ok(Bytes::from(bytes))
}

fn build_text_content_envelope(kind: TextAssetKind, request: &TextAssetRequest) -> Value {
    let text_kind =
        clean_option(&request.text_kind).unwrap_or_else(|| kind.default_kind_value().to_owned());
    let mut metadata = Map::new();
    metadata.insert(
        "language".to_owned(),
        json!(clean_option(&request.language).unwrap_or_else(|| "en".to_owned())),
    );
    metadata.insert(kind.metadata_kind_key().to_owned(), json!(text_kind));
    metadata.insert(
        "visibility".to_owned(),
        json!(clean_option(&request.visibility).unwrap_or_else(|| "public_preview".to_owned())),
    );
    metadata.insert(
        "rights_mode".to_owned(),
        json!(clean_option(&request.rights_mode)
            .unwrap_or_else(|| "creator_owned_original".to_owned())),
    );
    metadata.insert(
        "moderation_mode".to_owned(),
        json!(clean_option(&request.moderation_mode)
            .unwrap_or_else(|| "site_policy_or_creator_default".to_owned())),
    );
    metadata.insert(
        "content_warning".to_owned(),
        json!(clean_option(&request.content_warning)),
    );
    metadata.insert("tags".to_owned(), json!(normalize_tags(&request.tags)));

    if matches!(kind, TextAssetKind::Article) {
        metadata.insert(
            "subtitle".to_owned(),
            json!(clean_option(&request.subtitle)),
        );
        metadata.insert("summary".to_owned(), json!(clean_option(&request.summary)));
        metadata.insert(
            "hero_image_crab_url".to_owned(),
            json!(clean_option(&request.hero_image_crab_url)),
        );
        metadata.insert(
            "linked_source_crab_url".to_owned(),
            json!(clean_option(&request.linked_source_crab_url)),
        );
    }

    let mut relations = Map::new();
    relations.insert(
        "site".to_owned(),
        json!(clean_option(&request.site_context_crab_url)),
    );
    if let Some(parent) = clean_option(&request.parent_crab_url) {
        relations.insert("parent".to_owned(), json!(parent.clone()));
        relations.insert("target".to_owned(), json!(parent));
    }
    if let Some(thread) = clean_option(&request.thread_context_crab_url) {
        relations.insert("thread".to_owned(), json!(thread));
    }
    if let Some(hero_image) = clean_option(&request.hero_image_crab_url) {
        relations.insert("hero_image".to_owned(), json!(hero_image));
    }
    if let Some(source) = clean_option(&request.linked_source_crab_url) {
        relations.insert("source".to_owned(), json!(source));
    }

    json!({
        "schema": kind.content_schema(),
        "kind": kind.as_str(),
        "asset_kind": kind.as_str(),
        "format": "text/plain; charset=utf-8",
        "title": title_for(kind, request),
        "body": clean_string(&request.body),
        "subtitle": clean_option(&request.subtitle),
        "summary": clean_option(&request.summary),
        "metadata": Value::Object(metadata),
        "relations": Value::Object(relations),
        "site_connection": {
            "required": true,
            "relation": kind.relation(),
            "crab_url": clean_option(&request.site_context_crab_url),
        },
        "parent_reference": parent_reference_summary(kind, request),
        "thread_reference": thread_reference_summary(request),
        "creator_display": clean_option(&request.creator_display),
        "created_at_ms": now_ms(),
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
                message: "storage estimate rejected text asset prepare request",
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
        HeaderValue::from_static("application/json"),
    );

    send_to_storage(
        Method::POST,
        "/o",
        manifest_headers,
        body,
        "storage text asset manifest object upstream unavailable",
    )
    .await
}

async fn put_index_pointer(
    headers: &HeaderMap,
    asset_cid: &str,
    asset_kind: &str,
    manifest_cid: &str,
    owner: &OwnerSummary,
) -> Result<UpstreamBody, Response> {
    let raw_hash = asset_cid.trim_start_matches("b3:");
    let route = format!("/v1/index/assets/{raw_hash}/manifest");
    let index_base = index_base_url();
    let upstream_url = format!("{}{}", index_base.trim_end_matches('/'), route);

    let body = json!({
        "asset_kind": asset_kind,
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
                "failed to encode text asset manifest pointer",
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

fn build_text_manifest(
    kind: TextAssetKind,
    request: &TextAssetRequest,
    headers: &HeaderMap,
    storage_upload: &Value,
    asset_cid: &str,
    owner: &OwnerSummary,
    paid_proof: &PaidProofSummary,
) -> Value {
    let mut root = Map::new();
    root.insert("version".to_owned(), json!(1));
    root.insert("asset_cid".to_owned(), json!(asset_cid));
    root.insert("asset_kind".to_owned(), json!(kind.as_str()));

    root.insert(
        "owner".to_owned(),
        json!({
            "passport_subject": owner.passport_subject,
            "wallet_account": owner.wallet_account,
        }),
    );

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

    root.insert(
        "metadata".to_owned(),
        json!({
            "title": title_for(kind, request),
            "description": clean_option(&request.content_warning),
            "tags": normalize_tags(&request.tags),
            "language": clean_option(&request.language).unwrap_or_else(|| "en".to_owned()),
            kind.metadata_kind_key(): clean_option(&request.text_kind).unwrap_or_else(|| kind.default_kind_value().to_owned()),
            "visibility": clean_option(&request.visibility).unwrap_or_else(|| "public_preview".to_owned()),
            "rights_mode": clean_option(&request.rights_mode).unwrap_or_else(|| "creator_owned_original".to_owned()),
            "moderation_mode": clean_option(&request.moderation_mode).unwrap_or_else(|| "site_policy_or_creator_default".to_owned()),
            "content_type": TEXT_CONTENT_TYPE,
            "creator_display": clean_option(&request.creator_display),
            "subtitle": clean_option(&request.subtitle),
            "summary": clean_option(&request.summary),
            "hero_image_crab_url": clean_option(&request.hero_image_crab_url),
            "linked_source_crab_url": clean_option(&request.linked_source_crab_url),
        }),
    );

    root.insert(
        "site_connection".to_owned(),
        json!(site_connection_summary(kind, request)),
    );
    root.insert(
        "parent_reference".to_owned(),
        json!(parent_reference_summary(kind, request)),
    );
    root.insert(
        "thread_reference".to_owned(),
        json!(thread_reference_summary(request)),
    );
    root.insert(
        "article_references".to_owned(),
        json!(article_references_summary(request)),
    );

    root.insert(
        "provenance".to_owned(),
        json!({
            "created_at_ms": now_ms(),
            "source": format!("omnigate.{}_publish", kind.as_str()),
            "parent_cids": [],
        }),
    );

    root.insert(
        "storage".to_owned(),
        json!({
            "available": true,
            "content_type": TEXT_CONTENT_TYPE,
            "raw_url": format!("/o/{asset_cid}"),
            "paid": true,
        }),
    );

    root.insert(
        "paid_proof".to_owned(),
        json!({
            "paid_op": paid_proof.paid_op,
            "paid_asset": paid_proof.paid_asset,
            "paid_estimate_minor": paid_proof.paid_estimate_minor,
            "wallet_txid": paid_proof.wallet_txid,
            "wallet_receipt_hash": paid_proof.wallet_receipt_hash,
            "wallet_from": paid_proof.wallet_from,
            "wallet_to": paid_proof.wallet_to,
        }),
    );

    root.insert(
        "request_headers".to_owned(),
        json!({
            "idempotency_key": grab(headers, "idempotency-key"),
            "correlation_id": grab(headers, "x-correlation-id"),
            "request_id": grab(headers, "x-request-id"),
        }),
    );

    let receipts = receipt_refs_from_storage_upload(storage_upload, owner);
    root.insert("receipts".to_owned(), json!(receipts));

    Value::Object(root)
}

fn paid_proof_from_headers(headers: &HeaderMap, asset_kind: &str) -> RouteResult<PaidProofSummary> {
    let paid_op = required_header(headers, "x-ron-paid-op")?;
    let paid_asset = required_header(headers, "x-ron-paid-asset")?;
    let paid_estimate_minor = required_header(headers, "x-ron-paid-estimate-minor")?;
    let wallet_txid = required_header(headers, "x-ron-wallet-txid")?;
    let wallet_receipt_hash = required_header(headers, "x-ron-wallet-receipt-hash")?;
    let wallet_from = required_header(headers, "x-ron-wallet-from")?;
    let wallet_to = required_header(headers, "x-ron-wallet-to")?;

    if !paid_op.eq_ignore_ascii_case("hold") {
        return Err(route_problem(
            StatusCode::BAD_REQUEST,
            "invalid_paid_proof",
            "x-ron-paid-op must be hold",
            false,
            "invalid_paid_op",
        ));
    }

    if !paid_asset.eq_ignore_ascii_case(DEFAULT_ASSET) {
        return Err(route_problem(
            StatusCode::BAD_REQUEST,
            "invalid_paid_proof",
            "x-ron-paid-asset must be roc",
            false,
            "invalid_paid_asset",
        ));
    }

    if let Some(header_asset_kind) = grab(headers, "x-ron-asset-kind") {
        if !header_asset_kind.eq_ignore_ascii_case(asset_kind) {
            return Err(route_problem(
                StatusCode::BAD_REQUEST,
                "invalid_paid_proof",
                "x-ron-asset-kind does not match the publish route asset kind",
                false,
                "invalid_asset_kind",
            ));
        }
    }

    Ok(PaidProofSummary {
        paid_op,
        paid_asset,
        paid_estimate_minor,
        wallet_txid,
        wallet_receipt_hash,
        wallet_from,
        wallet_to,
    })
}

fn required_header(headers: &HeaderMap, name: &'static str) -> RouteResult<String> {
    grab(headers, name).ok_or_else(|| {
        route_problem(
            StatusCode::PAYMENT_REQUIRED,
            "missing_paid_proof",
            "paid publish requires modern wallet hold proof headers",
            false,
            header_reason(name),
        )
    })
}

fn header_reason(name: &str) -> &'static str {
    match name {
        "x-ron-paid-op" => "missing_x_ron_paid_op",
        "x-ron-paid-asset" => "missing_x_ron_paid_asset",
        "x-ron-paid-estimate-minor" => "missing_x_ron_paid_estimate_minor",
        "x-ron-wallet-txid" => "missing_x_ron_wallet_txid",
        "x-ron-wallet-receipt-hash" => "missing_x_ron_wallet_receipt_hash",
        "x-ron-wallet-from" => "missing_x_ron_wallet_from",
        "x-ron-wallet-to" => "missing_x_ron_wallet_to",
        _ => "missing_paid_header",
    }
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
            .or_else(|| value_string(storage_upload, "paid_estimate_minor"))
            .and_then(|value| value.parse::<u64>().ok()),
        "account": owner.wallet_account,
        "created_at_ms": now_ms(),
    })]
}

fn owner_from_request_and_headers(request: &TextAssetRequest, headers: &HeaderMap) -> OwnerSummary {
    OwnerSummary {
        passport_subject: clean_option(&request.owner_passport_subject)
            .or_else(|| grab(headers, "x-ron-passport"))
            .unwrap_or_else(|| "unknown-passport".to_owned()),
        wallet_account: clean_option(&request.payer_account)
            .or_else(|| grab(headers, "x-ron-wallet-account"))
            .or_else(|| grab(headers, "x-ron-wallet-from"))
            .unwrap_or_else(|| "unknown-wallet".to_owned()),
    }
}

fn site_connection_summary(
    kind: TextAssetKind,
    request: &TextAssetRequest,
) -> SiteConnectionSummary {
    SiteConnectionSummary {
        required: true,
        relation: kind.relation(),
        crab_url: clean_option(&request.site_context_crab_url).unwrap_or_default(),
    }
}

fn parent_reference_summary(
    kind: TextAssetKind,
    request: &TextAssetRequest,
) -> Option<ParentReferenceSummary> {
    let crab_url = clean_option(&request.parent_crab_url)?;
    let asset_kind = typed_asset_kind_from_crab_url(&crab_url)?;

    Some(ParentReferenceSummary {
        relation: kind.parent_relation(),
        crab_url,
        asset_kind,
    })
}

fn thread_reference_summary(request: &TextAssetRequest) -> Option<ParentReferenceSummary> {
    let crab_url = clean_option(&request.thread_context_crab_url)?;
    let asset_kind = typed_asset_kind_from_crab_url(&crab_url)?;

    Some(ParentReferenceSummary {
        relation: "thread_context",
        crab_url,
        asset_kind,
    })
}

fn article_references_summary(request: &TextAssetRequest) -> Value {
    json!({
        "hero_image": clean_option(&request.hero_image_crab_url),
        "source": clean_option(&request.linked_source_crab_url),
    })
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

fn is_valid_named_site_crab_url(value: &str) -> bool {
    let trimmed = value.trim();

    if trimmed.chars().any(char::is_control) || trimmed.contains(char::is_whitespace) {
        return false;
    }

    let Some(rest) = trimmed.strip_prefix("crab://") else {
        return false;
    };

    if rest.is_empty() || rest.starts_with('@') {
        return false;
    }

    typed_asset_kind_from_crab_url(trimmed).is_none()
}

fn typed_asset_kind_from_crab_url(value: &str) -> Option<String> {
    let rest = value.trim().strip_prefix("crab://")?;
    let (hash, kind) = rest.rsplit_once('.')?;

    if hash.len() != 64 {
        return None;
    }

    if !hash
        .bytes()
        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }

    let kind = kind.trim().to_ascii_lowercase();

    if kind.is_empty()
        || kind.len() > 32
        || !kind
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
    {
        return None;
    }

    Some(kind)
}

fn is_allowed_comment_parent_kind(kind: &str) -> bool {
    matches!(
        kind,
        "post"
            | "comment"
            | "article"
            | "image"
            | "video"
            | "music"
            | "song"
            | "podcast"
            | "stream"
            | "thread"
            | "film"
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

fn clean_string(value: &str) -> String {
    value.trim().to_owned()
}

fn clean_option(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn title_for(kind: TextAssetKind, request: &TextAssetRequest) -> String {
    clean_option(&request.title).unwrap_or_else(|| kind.default_title().to_owned())
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .take(32)
        .map(ToOwned::to_owned)
        .collect()
}

fn invalid_request_code(kind: TextAssetKind, action: &str) -> &'static str {
    match (kind, action) {
        (TextAssetKind::Post, "prepare") => "invalid_post_prepare_request",
        (TextAssetKind::Post, "publish") => "invalid_post_publish_request",
        (TextAssetKind::Comment, "prepare") => "invalid_comment_prepare_request",
        (TextAssetKind::Comment, "publish") => "invalid_comment_publish_request",
        (TextAssetKind::Article, "prepare") => "invalid_article_prepare_request",
        (TextAssetKind::Article, "publish") => "invalid_article_publish_request",
        _ => "invalid_text_asset_request",
    }
}

fn required_paid_headers() -> Vec<&'static str> {
    vec![
        "Authorization",
        "Idempotency-Key",
        "x-ron-paid-op",
        "x-ron-paid-asset",
        "x-ron-paid-estimate-minor",
        "x-ron-wallet-txid",
        "x-ron-wallet-receipt-hash",
        "x-ron-wallet-from",
        "x-ron-wallet-to",
        "x-ron-asset-kind",
    ]
}

fn optional_publish_headers() -> Vec<&'static str> {
    vec![
        "x-ron-passport",
        "x-ron-wallet-account",
        "x-ron-asset-title",
        "x-ron-asset-description",
        "x-ron-asset-tags",
        "x-ron-permission",
        "x-ron-spend-limit",
        "x-correlation-id",
        "x-request-id",
    ]
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

fn deserialize_tags<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    match value {
        Value::Null => Ok(Vec::new()),
        Value::String(raw) => Ok(raw
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(ToOwned::to_owned)
            .collect()),
        Value::Array(items) => items
            .into_iter()
            .map(|item| match item {
                Value::String(tag) => Ok(tag),
                _ => Err(serde::de::Error::custom("tags must be strings")),
            })
            .collect(),
        _ => Err(serde::de::Error::custom(
            "tags must be an array of strings or a comma-separated string",
        )),
    }
}
