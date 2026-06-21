//! RO:QUICKCHAIN-PREFLIGHT — quote is read-only; pay uses svc-wallet only; no direct ledger mutation; integer minor units only; wallet_receipt.
//! RO:WHAT — Paid b3 asset content-view quote/pay routes for CrabLink.
//! RO:WHY — NEXT_LEVEL creator economy: visitors can pay creators for article/post/comment/image/video/stream descriptor views through wallet truth.
//! RO:INTERACTS — svc-index asset manifest pointers, svc-storage manifest objects, svc-wallet /v1/transfer, svc-gateway proxy.
//! RO:INVARIANTS — quote is read-only; pay uses svc-wallet only; no direct ledger mutation; integer minor units only.
//! RO:METRICS — covered by omnigate HTTP middleware and downstream wallet metrics.
//! RO:CONFIG — OMNIGATE_CONTENT_VIEW_PRICE_MINOR, OMNIGATE_WALLET_BASE_URL, OMNIGATE_WALLET_BEARER.
//! RO:SECURITY — strict DTOs; payout recipient must match manifest; fail closed when manifest/payout is incomplete.
//! RO:TEST — cargo test -p omnigate --test content_view.

use axum::{
    body::Bytes,
    http::{header, HeaderMap, HeaderName, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, time::Duration};

const DEFAULT_INDEX_BASE_URL: &str = "http://127.0.0.1:5304";
const DEFAULT_STORAGE_BASE_URL: &str = "http://127.0.0.1:15303";
const DEFAULT_WALLET_BASE_URL: &str = "http://127.0.0.1:8088";
const DEFAULT_WALLET_BEARER: &str = "dev";

const DEFAULT_CONTENT_VIEW_PRICE_MINOR: &str = "5";
const DEFAULT_ASSET: &str = "roc";
const DEFAULT_CURRENCY: &str = "ROC";
const DEFAULT_WALLET_NONCE: u64 = 1;

const CONTENT_VIEW_QUOTE_SCHEMA: &str = "omnigate.content-view-quote.v1";
const CONTENT_VIEW_PAYMENT_SCHEMA: &str = "omnigate.content-view-payment.v1";

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate content_view route reqwest client should build")
});

/// Router for `/v1/content/view/*`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/view/quote", post(content_view_quote))
        .route("/view/pay", post(content_view_pay))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentViewQuoteRequest {
    #[serde(default, alias = "assetCrabUrl", alias = "crab_url", alias = "url")]
    asset_crab_url: Option<String>,
    #[serde(default, alias = "asset_cid", alias = "assetCid")]
    asset_cid: Option<String>,
    #[serde(default, alias = "asset_kind", alias = "assetKind")]
    asset_kind: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    quantity: Option<u64>,
    #[serde(default, alias = "payerAccount")]
    payer_account: Option<String>,
    #[serde(default, alias = "viewerWalletAccount")]
    viewer_wallet_account: Option<String>,
    #[allow(dead_code)]
    #[serde(default, alias = "viewerPassportSubject")]
    viewer_passport_subject: Option<String>,
    #[serde(default, alias = "recipientAccount")]
    recipient_account: Option<String>,
    #[serde(default, alias = "maxAmountMinor")]
    max_amount_minor: Option<String>,
    #[serde(default, alias = "clientIdempotencyKey")]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentViewPayRequest {
    #[serde(default, alias = "assetCrabUrl", alias = "crab_url", alias = "url")]
    asset_crab_url: Option<String>,
    #[serde(default, alias = "asset_cid", alias = "assetCid")]
    asset_cid: Option<String>,
    #[serde(default, alias = "asset_kind", alias = "assetKind")]
    asset_kind: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    action: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    quantity: Option<u64>,
    #[serde(default, alias = "payerAccount")]
    payer_account: Option<String>,
    #[serde(default, alias = "viewerWalletAccount")]
    viewer_wallet_account: Option<String>,
    #[allow(dead_code)]
    #[serde(default, alias = "viewerPassportSubject")]
    viewer_passport_subject: Option<String>,
    #[serde(default, alias = "recipientAccount")]
    recipient_account: Option<String>,
    #[serde(default, alias = "amountMinor")]
    amount_minor: Option<String>,
    #[serde(default)]
    asset: Option<String>,
    #[allow(dead_code)]
    #[serde(default, alias = "quoteId")]
    quote_id: Option<String>,
    #[allow(dead_code)]
    #[serde(default, alias = "quoteHash")]
    quote_hash: Option<String>,
    #[serde(default)]
    nonce: Option<u64>,
    #[allow(dead_code)]
    #[serde(default)]
    quote: Option<Value>,
    #[serde(default, alias = "clientIdempotencyKey")]
    client_idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetManifestPointer {
    version: u16,
    asset_cid: String,
    asset_kind: String,
    manifest_cid: String,
    #[serde(default)]
    owner_passport_subject: Option<String>,
    #[serde(default)]
    owner_wallet_account: Option<String>,
    updated_at_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct AssetManifestDocument {
    version: u16,
    asset_cid: String,
    asset_kind: String,
    #[serde(default)]
    owner: Option<ManifestOwner>,
    #[serde(default)]
    payout: Option<ManifestPayout>,
    #[serde(default)]
    metadata: Option<ManifestMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestOwner {
    #[serde(default)]
    passport_subject: Option<String>,
    #[serde(default)]
    wallet_account: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestPayout {
    #[serde(default)]
    default_action: Option<String>,
    #[serde(default)]
    recipient_account: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestMetadata {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    content_type: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedAssetTarget {
    raw_hash_hex: String,
    asset_cid: String,
    asset_kind: String,
    canonical_crab: String,
}

#[derive(Debug, Clone)]
struct ContentViewContext {
    parsed: ParsedAssetTarget,
    manifest_cid: String,
    updated_at_ms: u64,
    owner_passport_subject: Option<String>,
    owner_wallet_account: Option<String>,
    payout_action: String,
    payout_recipient_account: String,
    title: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    content_type: Option<String>,
}

#[derive(Debug)]
struct UpstreamBody {
    status: StatusCode,
    body: Bytes,
}

#[derive(Debug, Serialize)]
struct Problem<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
    reason: &'a str,
}

/// Quote a paid view for a b3-backed asset.
///
/// The quote is read-only. It resolves the asset manifest pointer, hydrates the
/// manifest, validates the payout action, and returns the manifest-derived
/// recipient. It never calls wallet or ledger.
pub async fn content_view_quote(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<ContentViewQuoteRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_content_view_quote_request",
                "content view quote request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    let parsed = match parse_asset_target(
        request.asset_crab_url.as_deref(),
        request.asset_cid.as_deref(),
        request.asset_kind.as_deref(),
    ) {
        Ok(parsed) => parsed,
        Err(reason) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_content_view_asset",
                "content view requires a canonical b3-backed asset target",
                false,
                reason,
            );
        }
    };

    let payer_account = match content_view_payer_account(
        &headers,
        request.payer_account.as_deref(),
        request.viewer_wallet_account.as_deref(),
    ) {
        Some(account) => account,
        None => {
            return problem(
                StatusCode::BAD_REQUEST,
                "content_view_missing_payer",
                "content view quote requires a payer wallet account",
                false,
                "missing_payer_account",
            );
        }
    };

    if let Some(header_account) = grab(&headers, "x-ron-wallet-account") {
        if header_account != payer_account {
            return problem(
                StatusCode::BAD_REQUEST,
                "wallet_account_mismatch",
                "x-ron-wallet-account must match content view payer account",
                false,
                "payer_mismatch",
            );
        }
    }

    let ctx = match load_content_view_context(parsed, &headers).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    if !is_content_view_action(&ctx.payout_action, &ctx.parsed.asset_kind) {
        return problem(
            StatusCode::BAD_REQUEST,
            "content_view_not_payable",
            "asset manifest payout action is not content_view",
            false,
            "content_view_not_payable",
        );
    }

    if let Some(request_recipient) = clean_optional(request.recipient_account.as_deref()) {
        if request_recipient != ctx.payout_recipient_account {
            return problem(
                StatusCode::BAD_REQUEST,
                "content_view_recipient_mismatch",
                "content view recipient must match asset manifest payout recipient",
                false,
                "recipient_mismatch",
            );
        }
    }

    let amount_minor = content_view_price_minor();
    let quantity = request.quantity.unwrap_or(1).max(1);

    if let Some(max_amount_minor) = clean_optional(request.max_amount_minor.as_deref()) {
        let Ok(max_amount_minor) = normalize_minor_units_owned(&max_amount_minor) else {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_content_view_max_amount",
                "max_amount_minor must be a positive integer string",
                false,
                "bad_max_amount_minor",
            );
        };

        if decimal_gt(&amount_minor, &max_amount_minor) {
            return problem(
                StatusCode::CONFLICT,
                "content_view_quote_exceeds_max",
                "content view quote exceeds caller max_amount_minor",
                false,
                "quote_exceeds_max",
            );
        }
    }

    let viewer_passport_subject = clean_optional(request.viewer_passport_subject.as_deref())
        .or_else(|| grab(&headers, "x-ron-passport"));
    let quote_id = content_view_quote_id(
        &ctx.parsed.asset_cid,
        &ctx.parsed.asset_kind,
        &payer_account,
        &ctx.payout_recipient_account,
        &amount_minor,
    );
    let quote_hash = simple_hex_hash(&quote_id);
    let idempotency_key = clean_optional(request.client_idempotency_key.as_deref())
        .or_else(|| grab(&headers, "idempotency-key"))
        .unwrap_or_else(|| {
            content_view_idempotency_key(
                "quote",
                &ctx.parsed.asset_cid,
                &payer_account,
                &ctx.payout_recipient_account,
            )
        });

    let policy = json!({
        "source": "omnigate.content_view.v1",
        "fixed_dev_price_minor": &amount_minor,
        "price_env": "OMNIGATE_CONTENT_VIEW_PRICE_MINOR",
        "wallet_front_door": "svc-wallet /v1/transfer"
    });

    let asset_page = json!({
        "manifest_cid": &ctx.manifest_cid,
        "owner_passport_subject": &ctx.owner_passport_subject,
        "owner_wallet_account": &ctx.owner_wallet_account,
        "title": &ctx.title,
        "description": &ctx.description,
        "tags": &ctx.tags,
        "content_type": &ctx.content_type,
        "updated_at_ms": ctx.updated_at_ms
    });

    let quote = json!({
        "schema": CONTENT_VIEW_QUOTE_SCHEMA,
        "action": "content_view",
        "asset": DEFAULT_ASSET,
        "amount_minor": &amount_minor,
        "display_amount": format!("{} {}", amount_minor, DEFAULT_CURRENCY),
        "quantity": quantity,
        "payer_account": &payer_account,
        "viewer_wallet_account": &payer_account,
        "viewer_passport_subject": &viewer_passport_subject,
        "recipient_account": &ctx.payout_recipient_account,
        "asset_cid": &ctx.parsed.asset_cid,
        "asset_kind": &ctx.parsed.asset_kind,
        "asset_crab_url": &ctx.parsed.canonical_crab,
        "quote_id": &quote_id,
        "quote_hash": &quote_hash,
        "expires_in_seconds": 300,
        "policy": policy,
        "asset_page": asset_page
    });

    let next = json!({
        "pay": "/v1/content/view/pay",
        "public_pay": "/content/view/pay",
        "required": ["asset_crab_url", "payer_account", "amount_minor", "recipient_account"]
    });

    let response = json!({
        "schema": CONTENT_VIEW_QUOTE_SCHEMA,
        "ok": true,
        "action": "content_view",
        "asset": DEFAULT_ASSET,
        "amount_minor": &amount_minor,
        "display_amount": format!("{} {}", amount_minor, DEFAULT_CURRENCY),
        "quantity": quantity,
        "payer_account": &payer_account,
        "viewer_wallet_account": &payer_account,
        "viewer_passport_subject": &viewer_passport_subject,
        "recipient_account": &ctx.payout_recipient_account,
        "asset_cid": &ctx.parsed.asset_cid,
        "asset_kind": &ctx.parsed.asset_kind,
        "asset_crab_url": &ctx.parsed.canonical_crab,
        "quote_id": &quote_id,
        "quote_hash": &quote_hash,
        "client_idempotency_key": &idempotency_key,
        "expires_in_seconds": 300,
        "quote": quote,
        "next": next
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Pay for a b3-backed asset view through `svc-wallet` transfer.
pub async fn content_view_pay(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<ContentViewPayRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_content_view_pay_request",
                "content view pay request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    let parsed = match parse_asset_target(
        request.asset_crab_url.as_deref(),
        request.asset_cid.as_deref(),
        request.asset_kind.as_deref(),
    ) {
        Ok(parsed) => parsed,
        Err(reason) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_content_view_asset",
                "content view payment requires a canonical b3-backed asset target",
                false,
                reason,
            );
        }
    };

    let payer_account = match content_view_payer_account(
        &headers,
        request.payer_account.as_deref(),
        request.viewer_wallet_account.as_deref(),
    ) {
        Some(account) => account,
        None => {
            return problem(
                StatusCode::BAD_REQUEST,
                "content_view_missing_payer",
                "content view payment requires a payer wallet account",
                false,
                "missing_payer_account",
            );
        }
    };

    if let Some(header_account) = grab(&headers, "x-ron-wallet-account") {
        if header_account != payer_account {
            return problem(
                StatusCode::BAD_REQUEST,
                "wallet_account_mismatch",
                "x-ron-wallet-account must match content view payer account",
                false,
                "payer_mismatch",
            );
        }
    }

    let ctx = match load_content_view_context(parsed, &headers).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    if !is_content_view_action(&ctx.payout_action, &ctx.parsed.asset_kind) {
        return problem(
            StatusCode::BAD_REQUEST,
            "content_view_not_payable",
            "asset manifest payout action is not content_view",
            false,
            "content_view_not_payable",
        );
    }

    if payer_account == ctx.payout_recipient_account {
        return problem(
            StatusCode::BAD_REQUEST,
            "content_view_self_payment_not_allowed",
            "creator self-view payment is not a wallet transfer",
            false,
            "self_payment_not_allowed",
        );
    }

    if let Some(request_recipient) = clean_optional(request.recipient_account.as_deref()) {
        if request_recipient != ctx.payout_recipient_account {
            return problem(
                StatusCode::BAD_REQUEST,
                "content_view_recipient_mismatch",
                "content view recipient must match asset manifest payout recipient",
                false,
                "recipient_mismatch",
            );
        }
    }

    let amount_minor = match clean_optional(request.amount_minor.as_deref()) {
        Some(amount_minor) => match normalize_minor_units_owned(&amount_minor) {
            Ok(amount_minor) => amount_minor,
            Err(_) => {
                return problem(
                    StatusCode::BAD_REQUEST,
                    "invalid_content_view_amount",
                    "amount_minor must be a positive integer string",
                    false,
                    "bad_amount_minor",
                );
            }
        },
        None => content_view_price_minor(),
    };

    let expected_amount_minor = content_view_price_minor();
    if amount_minor != expected_amount_minor {
        return problem(
            StatusCode::CONFLICT,
            "content_view_amount_mismatch",
            "amount_minor must match the current content_view quote",
            false,
            "amount_mismatch",
        );
    }

    let asset = clean_optional(request.asset.as_deref())
        .unwrap_or_else(|| DEFAULT_ASSET.to_owned())
        .to_ascii_lowercase();
    if asset != DEFAULT_ASSET {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_content_view_asset_currency",
            "content_view payment currently supports only roc",
            false,
            "asset_must_be_roc",
        );
    }

    let idempotency_key = clean_optional(request.client_idempotency_key.as_deref())
        .or_else(|| grab(&headers, "idempotency-key"))
        .unwrap_or_else(|| {
            content_view_idempotency_key(
                "pay",
                &ctx.parsed.asset_cid,
                &payer_account,
                &ctx.payout_recipient_account,
            )
        });
    let nonce = request.nonce.unwrap_or_else(default_content_view_nonce);

    let wallet_response = match send_wallet_content_view_transfer(
        &headers,
        &payer_account,
        &ctx.payout_recipient_account,
        &amount_minor,
        nonce,
        &idempotency_key,
        &ctx,
    )
    .await
    {
        Ok(response) => response,
        Err(response) => return response,
    };

    let wallet_status = wallet_response.status;
    let wallet_body = wallet_response.body;
    let wallet_json = serde_json::from_slice::<Value>(&wallet_body).ok();

    if !wallet_status.is_success() {
        if wallet_status == StatusCode::CONFLICT {
            if let Some(expected_nonce) = wallet_json
                .as_ref()
                .and_then(|value| value_string(value, "message"))
                .and_then(|message| parse_expected_nonce(&message))
            {
                if expected_nonce != nonce {
                    match send_wallet_content_view_transfer(
                        &headers,
                        &payer_account,
                        &ctx.payout_recipient_account,
                        &amount_minor,
                        expected_nonce,
                        &idempotency_key,
                        &ctx,
                    )
                    .await
                    {
                        Ok(retry_response) if retry_response.status.is_success() => {
                            return content_view_payment_response(
                                StatusCode::OK,
                                &ctx,
                                &payer_account,
                                &amount_minor,
                                expected_nonce,
                                &idempotency_key,
                                serde_json::from_slice::<Value>(&retry_response.body).ok(),
                            );
                        }
                        Ok(retry_response) => {
                            return wallet_transfer_problem(
                                retry_response.status,
                                retry_response.body,
                            )
                        }
                        Err(response) => return response,
                    }
                }
            }
        }

        return wallet_transfer_problem(wallet_status, wallet_body);
    }

    content_view_payment_response(
        StatusCode::OK,
        &ctx,
        &payer_account,
        &amount_minor,
        nonce,
        &idempotency_key,
        wallet_json,
    )
}

async fn load_content_view_context(
    parsed: ParsedAssetTarget,
    headers: &HeaderMap,
) -> Result<ContentViewContext, Response> {
    let pointer = match fetch_asset_pointer(&parsed, headers).await {
        Ok(Some(pointer)) => pointer,
        Ok(None) => {
            return Err(problem(
                StatusCode::NOT_FOUND,
                "asset_manifest_pointer_not_found",
                "asset manifest pointer was not found",
                false,
                "asset_pointer_not_found",
            ));
        }
        Err(response) => return Err(response),
    };

    if pointer.version != 1
        || pointer.asset_cid != parsed.asset_cid
        || pointer.asset_kind != parsed.asset_kind
        || !is_canonical_b3_cid(&pointer.manifest_cid)
    {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "asset_pointer_invalid",
            "asset manifest pointer is invalid for paid content view",
            true,
            "asset_pointer_invalid",
        ));
    }

    let manifest = fetch_asset_manifest(&pointer.manifest_cid, headers).await?;

    if manifest.version != 1
        || manifest.asset_cid != parsed.asset_cid
        || manifest.asset_kind != parsed.asset_kind
    {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "asset_manifest_invalid",
            "asset manifest is invalid for paid content view",
            true,
            "asset_manifest_invalid",
        ));
    }

    let owner_passport_subject = manifest
        .owner
        .as_ref()
        .and_then(|owner| owner.passport_subject.clone())
        .or(pointer.owner_passport_subject);

    let owner_wallet_account = manifest
        .owner
        .as_ref()
        .and_then(|owner| owner.wallet_account.clone())
        .or(pointer.owner_wallet_account);

    let payout_action = manifest
        .payout
        .as_ref()
        .and_then(|payout| payout.default_action.clone())
        .unwrap_or_else(|| "content_view".to_owned());

    let payout_recipient_account = manifest
        .payout
        .as_ref()
        .and_then(|payout| payout.recipient_account.clone())
        .or_else(|| owner_wallet_account.clone())
        .and_then(|account| clean_optional(Some(&account)))
        .ok_or_else(|| {
            problem(
                StatusCode::BAD_GATEWAY,
                "content_view_missing_recipient",
                "asset manifest did not include a payout recipient account",
                true,
                "missing_recipient_account",
            )
        })?;

    let (title, description, tags, content_type) = match manifest.metadata {
        Some(metadata) => (
            metadata.title,
            metadata.description,
            metadata.tags,
            metadata.content_type,
        ),
        None => (None, None, Vec::new(), None),
    };

    Ok(ContentViewContext {
        parsed,
        manifest_cid: pointer.manifest_cid,
        updated_at_ms: pointer.updated_at_ms,
        owner_passport_subject,
        owner_wallet_account,
        payout_action,
        payout_recipient_account,
        title,
        description,
        tags,
        content_type,
    })
}

async fn fetch_asset_pointer(
    parsed: &ParsedAssetTarget,
    headers: &HeaderMap,
) -> Result<Option<AssetManifestPointer>, Response> {
    let route = format!("/v1/index/assets/{}/manifest", parsed.raw_hash_hex);
    let upstream_url = format!("{}{}", index_base_url().trim_end_matches('/'), route);
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
                "index asset pointer upstream unavailable",
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
            "index_asset_pointer_rejected",
            "index rejected asset pointer lookup",
            upstream_res.status().as_u16() >= 500,
            "index_asset_pointer_rejected",
        ));
    }

    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "index asset pointer upstream unavailable",
                true,
                "index_read",
            ));
        }
    };

    serde_json::from_slice::<AssetManifestPointer>(&body)
        .map(Some)
        .map_err(|_| {
            problem(
                StatusCode::BAD_GATEWAY,
                "index_asset_pointer_bad_json",
                "index asset pointer response was not valid JSON",
                true,
                "index_asset_pointer_bad_json",
            )
        })
}

async fn fetch_asset_manifest(
    manifest_cid: &str,
    headers: &HeaderMap,
) -> Result<AssetManifestDocument, Response> {
    let route = format!("/o/{manifest_cid}");
    let upstream_url = format!("{}{}", storage_base_url().trim_end_matches('/'), route);
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
                "storage asset manifest upstream unavailable",
                true,
                "storage_connect",
            ));
        }
    };

    if upstream_res.status() == StatusCode::NOT_FOUND {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "asset_manifest_missing",
            "asset manifest object was not found in storage",
            true,
            "asset_manifest_missing",
        ));
    }

    if !upstream_res.status().is_success() {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "storage_asset_manifest_rejected",
            "storage rejected asset manifest fetch",
            upstream_res.status().as_u16() >= 500,
            "storage_asset_manifest_rejected",
        ));
    }

    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "storage asset manifest upstream unavailable",
                true,
                "storage_read",
            ));
        }
    };

    serde_json::from_slice::<AssetManifestDocument>(&body).map_err(|_| {
        problem(
            StatusCode::BAD_GATEWAY,
            "asset_manifest_bad_json",
            "asset manifest object was not valid JSON",
            true,
            "asset_manifest_bad_json",
        )
    })
}

fn content_view_payment_response(
    status: StatusCode,
    ctx: &ContentViewContext,
    payer_account: &str,
    amount_minor: &str,
    nonce: u64,
    idempotency_key: &str,
    wallet_receipt: Option<Value>,
) -> Response {
    let txid = wallet_receipt
        .as_ref()
        .and_then(|value| value_string(value, "txid"));
    let receipt_hash = wallet_receipt
        .as_ref()
        .and_then(|value| value_string(value, "receipt_hash"));
    let ledger_root = wallet_receipt
        .as_ref()
        .and_then(|value| value_string(value, "ledger_root"));

    let payment = json!({
        "schema": CONTENT_VIEW_PAYMENT_SCHEMA,
        "action": "content_view",
        "asset": DEFAULT_ASSET,
        "amount_minor": amount_minor,
        "payer_account": payer_account,
        "recipient_account": &ctx.payout_recipient_account,
        "asset_cid": &ctx.parsed.asset_cid,
        "asset_kind": &ctx.parsed.asset_kind,
        "asset_crab_url": &ctx.parsed.canonical_crab,
        "nonce": nonce,
        "txid": &txid,
        "receipt_hash": &receipt_hash,
        "ledger_root": &ledger_root,
        "status": "paid",
        "wallet_receipt": &wallet_receipt
    });

    let receipt = json!({
        "kind": "content_view",
        "wallet_txid": &txid,
        "wallet_receipt_hash": &receipt_hash,
        "idempotency_key": idempotency_key,
        "asset_cid": &ctx.parsed.asset_cid,
        "asset_kind": &ctx.parsed.asset_kind,
        "asset_crab_url": &ctx.parsed.canonical_crab,
        "manifest_cid": &ctx.manifest_cid,
        "paid_at_ms": now_ms()
    });

    let response = json!({
        "schema": CONTENT_VIEW_PAYMENT_SCHEMA,
        "ok": true,
        "action": "content_view",
        "asset": DEFAULT_ASSET,
        "amount_minor": amount_minor,
        "payer_account": payer_account,
        "viewer_wallet_account": payer_account,
        "recipient_account": &ctx.payout_recipient_account,
        "asset_cid": &ctx.parsed.asset_cid,
        "asset_kind": &ctx.parsed.asset_kind,
        "asset_crab_url": &ctx.parsed.canonical_crab,
        "manifest_cid": &ctx.manifest_cid,
        "nonce": nonce,
        "txid": &txid,
        "receipt_hash": &receipt_hash,
        "ledger_root": &ledger_root,
        "wallet_receipt": &wallet_receipt,
        "payment": payment,
        "receipt": receipt
    });

    (status, Json(response)).into_response()
}

async fn send_wallet_content_view_transfer(
    headers: &HeaderMap,
    payer_account: &str,
    recipient_account: &str,
    amount_minor: &str,
    nonce: u64,
    idempotency_key: &str,
    ctx: &ContentViewContext,
) -> Result<UpstreamBody, Response> {
    let url = format!("{}/v1/transfer", wallet_base_url());
    let body = json!({
        "from": payer_account,
        "to": recipient_account,
        "asset": DEFAULT_ASSET,
        "amount_minor": amount_minor,
        "nonce": nonce,
        "idempotency_key": idempotency_key,
        "memo": format!("crablink content_view {}", ctx.parsed.canonical_crab),
    });

    let mut builder = HTTP_CLIENT
        .post(url)
        .bearer_auth(wallet_bearer())
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/json")
        .header("idempotency-key", idempotency_key)
        .json(&body);

    if let Some(correlation_id) = grab(headers, "x-correlation-id") {
        builder = builder.header("x-correlation-id", correlation_id);
    }

    if let Some(request_id) = grab(headers, "x-request-id") {
        builder = builder.header("x-request-id", request_id);
    }

    let upstream_res = match builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "wallet_transfer_unavailable",
                "svc-wallet transfer route is unavailable",
                true,
                "wallet_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "wallet_transfer_read_failed",
                "failed to read svc-wallet transfer response",
                true,
                "wallet_read",
            ));
        }
    };

    Ok(UpstreamBody { status, body })
}

fn wallet_transfer_problem(status: StatusCode, body: Bytes) -> Response {
    let retryable = status.as_u16() >= 500 || status == StatusCode::TOO_MANY_REQUESTS;
    let wallet_error = serde_json::from_slice::<Value>(&body).unwrap_or_else(|_| {
        json!({
            "message": String::from_utf8_lossy(&body).to_string()
        })
    });

    (
        status,
        Json(json!({
            "code": "wallet_content_view_transfer_rejected",
            "message": "svc-wallet rejected content view transfer",
            "retryable": retryable,
            "reason": "wallet_transfer_rejected",
            "wallet_status": status.as_u16(),
            "wallet_error": wallet_error
        })),
    )
        .into_response()
}

fn parse_asset_target(
    asset_crab_url: Option<&str>,
    asset_cid: Option<&str>,
    asset_kind: Option<&str>,
) -> Result<ParsedAssetTarget, &'static str> {
    if let Some(url) = clean_optional(asset_crab_url) {
        return parse_crab_asset_url(&url);
    }

    let asset_cid = clean_optional(asset_cid).ok_or("missing_asset_target")?;
    let asset_kind = clean_optional(asset_kind)
        .map(|value| value.to_ascii_lowercase())
        .ok_or("missing_asset_kind")?;

    let raw_hash_hex = asset_cid
        .strip_prefix("b3:")
        .ok_or("invalid_asset_cid")?
        .to_owned();

    canonicalize_hash(&raw_hash_hex)?;
    normalize_asset_kind(&asset_kind)?;

    Ok(ParsedAssetTarget {
        asset_cid: format!("b3:{raw_hash_hex}"),
        canonical_crab: format!("crab://{raw_hash_hex}.{asset_kind}"),
        raw_hash_hex,
        asset_kind,
    })
}

fn parse_crab_asset_url(input: &str) -> Result<ParsedAssetTarget, &'static str> {
    if input.chars().any(char::is_control) {
        return Err("control_character");
    }

    let target = input
        .trim()
        .strip_prefix("crab://")
        .ok_or("invalid_scheme")?;

    if target.starts_with("b3/") {
        return Err("b3_slash_prefix_rejected");
    }

    if target.contains('/') || target.contains('\\') || target.contains('?') || target.contains('#')
    {
        return Err("unsafe_asset_path");
    }

    let (hash, kind) = target.rsplit_once('.').ok_or("missing_asset_kind")?;
    let raw_hash_hex = canonicalize_hash(hash)?;
    let asset_kind = normalize_asset_kind(kind)?;

    Ok(ParsedAssetTarget {
        asset_cid: format!("b3:{raw_hash_hex}"),
        canonical_crab: format!("crab://{raw_hash_hex}.{asset_kind}"),
        raw_hash_hex,
        asset_kind,
    })
}

fn canonicalize_hash(hash: &str) -> Result<String, &'static str> {
    if hash.len() != 64 {
        return Err("invalid_hash_length");
    }

    if !hash
        .bytes()
        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err("invalid_hash_characters");
    }

    Ok(hash.to_owned())
}

fn normalize_asset_kind(kind: &str) -> Result<String, &'static str> {
    let kind = kind.trim().to_ascii_lowercase();

    let ok = matches!(
        kind.as_str(),
        "image"
            | "article"
            | "post"
            | "comment"
            | "video"
            | "stream"
            | "music"
            | "song"
            | "podcast"
    );

    if ok {
        Ok(kind)
    } else {
        Err("unsupported_asset_kind")
    }
}

fn is_content_view_action(action: &str, asset_kind: &str) -> bool {
    let action = action.trim().to_ascii_lowercase();

    action == "content_view"
        || action == "asset_view"
        || action == "view"
        || action == format!("{asset_kind}_view")
        || (asset_kind == "stream"
            && matches!(
                action.as_str(),
                "stream_view" | "stream_join" | "stream_watch_interval"
            ))
}

fn content_view_payer_account(
    headers: &HeaderMap,
    payer_account: Option<&str>,
    viewer_wallet_account: Option<&str>,
) -> Option<String> {
    clean_optional(payer_account)
        .or_else(|| clean_optional(viewer_wallet_account))
        .or_else(|| grab(headers, "x-ron-wallet-account"))
}

fn content_view_price_minor() -> String {
    env::var("OMNIGATE_CONTENT_VIEW_PRICE_MINOR")
        .ok()
        .and_then(|value| normalize_minor_units_owned(&value).ok())
        .filter(|value| value != "0")
        .unwrap_or_else(|| DEFAULT_CONTENT_VIEW_PRICE_MINOR.to_owned())
}

fn default_content_view_nonce() -> u64 {
    env::var("OMNIGATE_CONTENT_VIEW_NONCE")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|nonce| *nonce >= DEFAULT_WALLET_NONCE)
        .unwrap_or(DEFAULT_WALLET_NONCE)
}

fn wallet_base_url() -> String {
    env::var("OMNIGATE_WALLET_BASE_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_WALLET_BASE_URL.to_owned())
}

fn wallet_bearer() -> String {
    env::var("OMNIGATE_WALLET_BEARER")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_WALLET_BEARER.to_owned())
}

fn storage_base_url() -> String {
    env::var("OMNIGATE_STORAGE_BASE_URL")
        .or_else(|_| env::var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_STORAGE_BASE_URL.to_owned())
}

fn index_base_url() -> String {
    env::var("OMNIGATE_INDEX_BASE_URL")
        .or_else(|_| env::var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL"))
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_INDEX_BASE_URL.to_owned())
}

fn content_view_quote_id(
    asset_cid: &str,
    asset_kind: &str,
    payer_account: &str,
    recipient_account: &str,
    amount_minor: &str,
) -> String {
    let seed = format!(
        "content_view:{asset_cid}:{asset_kind}:{payer_account}:{recipient_account}:{amount_minor}"
    );
    format!("content-view-{}", simple_hex_hash(&seed))
}

fn content_view_idempotency_key(
    scope: &str,
    asset_cid: &str,
    payer_account: &str,
    recipient_account: &str,
) -> String {
    let seed = format!("content_view:{scope}:{asset_cid}:{payer_account}:{recipient_account}");
    format!("content-view-{scope}-{}", simple_hex_hash(&seed))
}

fn simple_hex_hash(input: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
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

fn normalize_minor_units_owned(value: &str) -> Result<String, &'static str> {
    let value = value.trim();
    if value.is_empty() || !value.as_bytes().iter().all(u8::is_ascii_digit) {
        return Err("not_integer");
    }
    let trimmed = value.trim_start_matches('0');
    if trimmed.is_empty() {
        return Err("zero_amount");
    }
    Ok(trimmed.to_owned())
}

fn decimal_gt(left: &str, right: &str) -> bool {
    let left = left.trim_start_matches('0');
    let right = right.trim_start_matches('0');
    left.len() > right.len() || (left.len() == right.len() && left > right)
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_expected_nonce(message: &str) -> Option<u64> {
    let marker = "expected ";
    let start = message.find(marker)? + marker.len();
    let rest = &message[start..];
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    digits.parse::<u64>().ok()
}

fn should_forward_header(name: &HeaderName) -> bool {
    if is_hop_by_hop_or_host(name) || name == header::CONTENT_LENGTH {
        return false;
    }

    name == header::AUTHORIZATION
        || name == header::ACCEPT
        || name == header::CONTENT_TYPE
        || super::header_policy::is_allowed_ron_context_header(name)
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
    use std::time::{SystemTime, UNIX_EPOCH};

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

#[cfg(test)]
mod stream_descriptor_content_view_tests {
    use super::*;

    #[test]
    fn content_view_accepts_stream_descriptor_kind() {
        let parsed = parse_crab_asset_url(
            "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.stream",
        )
        .expect("stream crab URL should parse for descriptor paid access");

        assert_eq!(parsed.asset_kind, "stream");
        assert_eq!(
            parsed.canonical_crab,
            "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.stream"
        );

        assert!(is_content_view_action("content_view", "stream"));
        assert!(is_content_view_action("stream_view", "stream"));
        assert!(is_content_view_action("stream_watch_interval", "stream"));
    }
}
